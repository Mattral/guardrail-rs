//! Configuration loading, hot-reload, and pipeline construction.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use arc_swap::ArcSwap;
use guardrail_classifiers::{PiiRedactor, RegexInjectionScanner};
use guardrail_core::{
    pipeline::{Pipeline, PipelineBuilder},
    policy::{PolicyAction, PolicyCondition, PolicyEngine, PolicyRule},
};

use crate::schema::{
    Config, PiiEntityList, PolicyActionConfig, PolicyConditionConfig,
};

/// Errors that can occur while loading or validating configuration.
#[derive(Debug, thiserror::Error)]
pub enum ConfigLoadError {
    /// The config file could not be read from disk.
    #[error("failed to read config file '{path}': {source}")]
    Io {
        /// Path that failed to read.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The config file contents could not be parsed as TOML.
    #[error("failed to parse config file '{path}': {source}")]
    Parse {
        /// Path that failed to parse.
        path: PathBuf,
        /// Underlying TOML error.
        #[source]
        source: toml::de::Error,
    },

    /// The config was syntactically valid TOML but failed semantic validation.
    #[error("configuration validation failed:\n{}", .0.join("\n"))]
    Validation(Vec<String>),

    /// A stage's regex pattern failed to compile.
    #[error("failed to build stage '{stage}': {source}")]
    StageBuild {
        /// The stage that failed to build.
        stage: String,
        /// Underlying error.
        #[source]
        source: guardrail_core::error::GuardrailError,
    },
}

/// Load a [`Config`] from a TOML file and validate it.
///
/// # Errors
///
/// Returns [`ConfigLoadError::Io`] if the file cannot be read,
/// [`ConfigLoadError::Parse`] if it isn't valid TOML, or
/// [`ConfigLoadError::Validation`] if validation fails.
///
/// # Examples
///
/// ```rust,no_run
/// use guardrail_config::loader::load_config;
///
/// let config = load_config("guardrail.toml").unwrap();
/// ```
pub fn load_config(path: impl AsRef<Path>) -> Result<Config, ConfigLoadError> {
    let path = path.as_ref();
    let contents = std::fs::read_to_string(path).map_err(|source| ConfigLoadError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    let config: Config = toml::from_str(&contents).map_err(|source| ConfigLoadError::Parse {
        path: path.to_path_buf(),
        source,
    })?;

    let errors = crate::validate::validate_config(&config);
    if !errors.is_empty() {
        return Err(ConfigLoadError::Validation(errors));
    }

    Ok(config)
}

/// Build a [`Pipeline`] from a validated [`Config`].
///
/// Stages are added in a fixed, security-conscious order:
///
/// 1. `regex_injection` — fastest, catches the majority of attacks
/// 2. `onnx_injection` — semantic detection (if `onnx` feature enabled)
/// 3. `pii_redaction` — sanitizes before the toxicity check sees the text
/// 4. `toxicity` — slowest, runs last among classifiers
/// 5. `policy_engine` — user-defined rules, evaluated last
///
/// # Errors
///
/// Returns [`ConfigLoadError::StageBuild`] if a stage's patterns fail to compile,
/// or if an ONNX stage is enabled without the `onnx` feature.
pub fn build_pipeline(config: &Config) -> Result<Pipeline, ConfigLoadError> {
    let mut builder = PipelineBuilder::default();

    // 1. Regex injection scanner
    if config.stages.regex_injection.enabled {
        let scanner = match &config.stages.regex_injection.custom_rules_path {
            Some(path) => {
                let contents = std::fs::read_to_string(path).map_err(|source| {
                    ConfigLoadError::Io {
                        path: PathBuf::from(path),
                        source,
                    }
                })?;
                RegexInjectionScanner::from_rule_str(
                    &contents,
                    !config.stages.regex_injection.log_only,
                )
                .map_err(|source| ConfigLoadError::StageBuild {
                    stage: "regex_injection".into(),
                    source,
                })?
            }
            None => {
                if config.stages.regex_injection.log_only {
                    RegexInjectionScanner::from_rule_str(
                        include_str!("../../guardrail-classifiers/src/rules/injection.rules"),
                        false,
                    )
                    .map_err(|source| ConfigLoadError::StageBuild {
                        stage: "regex_injection".into(),
                        source,
                    })?
                } else {
                    RegexInjectionScanner::default()
                }
            }
        };
        builder = builder.stage(scanner);
    }

    // 2. ONNX injection classifier
    #[cfg(feature = "onnx")]
    if config.stages.onnx_injection.enabled {
        let model_path = config
            .stages
            .onnx_injection
            .model_path
            .as_ref()
            .expect("validated: model_path present");
        let tokenizer_path = config
            .stages
            .onnx_injection
            .tokenizer_path
            .as_ref()
            .expect("validated: tokenizer_path present");

        let classifier = guardrail_classifiers::OnnxInjectionClassifier::load(
            model_path,
            tokenizer_path,
            config.stages.onnx_injection.threshold,
        )
        .map_err(|source| ConfigLoadError::StageBuild {
            stage: "onnx_injection".into(),
            source,
        })?;
        builder = builder.stage(classifier);
    }

    // 3. PII redaction
    if config.stages.pii_redaction.enabled {
        let entities = PiiEntityList::from_strings(&config.stages.pii_redaction.entities)
            .map_err(|e| ConfigLoadError::StageBuild {
                stage: "pii_redaction".into(),
                source: guardrail_core::error::GuardrailError::Config(e),
            })?;

        let redactor = PiiRedactor::new(entities.0, config.stages.pii_redaction.validate_luhn)
            .map_err(|source| ConfigLoadError::StageBuild {
                stage: "pii_redaction".into(),
                source,
            })?;
        builder = builder.stage(redactor);
    }

    // 4. ONNX toxicity classifier
    #[cfg(feature = "onnx")]
    if config.stages.toxicity.enabled {
        let model_path = config
            .stages
            .toxicity
            .model_path
            .as_ref()
            .expect("validated: model_path present");
        let tokenizer_path = config
            .stages
            .toxicity
            .tokenizer_path
            .as_ref()
            .expect("validated: tokenizer_path present");

        let classifier = guardrail_classifiers::ToxicityClassifier::load(
            model_path,
            tokenizer_path,
            config.stages.toxicity.threshold,
        )
        .map_err(|source| ConfigLoadError::StageBuild {
            stage: "toxicity".into(),
            source,
        })?;
        builder = builder.stage(classifier);
    }

    // 5. Policy engine
    if !config.policy.rules.is_empty() {
        let rules = config
            .policy
            .rules
            .iter()
            .map(|r| PolicyRule {
                name: r.name.clone(),
                enabled: r.enabled,
                condition: convert_condition(&r.condition),
                action: convert_action(r.action),
                message: r.message.clone(),
            })
            .collect();
        builder = builder.stage(PolicyEngine::new(rules));
    }

    Ok(builder.build())
}

fn convert_condition(c: &PolicyConditionConfig) -> PolicyCondition {
    match c {
        PolicyConditionConfig::ContentContains { keywords } => {
            PolicyCondition::ContentContains(
                keywords.iter().map(|k| k.to_lowercase()).collect(),
            )
        }
        PolicyConditionConfig::SystemPromptAbsent => PolicyCondition::SystemPromptAbsent,
        PolicyConditionConfig::TokenCountExceeds { limit } => {
            PolicyCondition::TokenCountExceeds(*limit)
        }
        PolicyConditionConfig::Always => PolicyCondition::Always,
    }
}

fn convert_action(a: PolicyActionConfig) -> PolicyAction {
    match a {
        PolicyActionConfig::Allow => PolicyAction::Allow,
        PolicyActionConfig::Redact => PolicyAction::Redact,
        PolicyActionConfig::Block => PolicyAction::Block,
        PolicyActionConfig::LogOnly => PolicyAction::LogOnly,
    }
}

/// A hot-reloadable handle to the current [`Config`] and [`Pipeline`].
///
/// Wraps both in [`ArcSwap`] so that [`ConfigHandle::pipeline`] can be called
/// from the hot path without locking, while [`ConfigHandle::reload`] can be
/// called concurrently (e.g., from a SIGHUP handler) to atomically swap in a
/// newly-built pipeline.
///
/// # Examples
///
/// ```rust,no_run
/// use guardrail_config::ConfigHandle;
///
/// let handle = ConfigHandle::load("guardrail.toml").unwrap();
///
/// // On the hot path:
/// let pipeline = handle.pipeline();
///
/// // On SIGHUP:
/// handle.reload().unwrap();
/// ```
pub struct ConfigHandle {
    path: PathBuf,
    config: ArcSwap<Config>,
    pipeline: ArcSwap<Pipeline>,
}

impl ConfigHandle {
    /// Load configuration and build the initial pipeline.
    ///
    /// # Errors
    ///
    /// See [`load_config`] and [`build_pipeline`].
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigLoadError> {
        let path = path.as_ref().to_path_buf();
        let config = load_config(&path)?;
        let pipeline = build_pipeline(&config)?;

        Ok(Self {
            path,
            config: ArcSwap::from_pointee(config),
            pipeline: ArcSwap::from_pointee(pipeline),
        })
    }

    /// Re-read the config file from disk, rebuild the pipeline, and atomically
    /// swap it in.
    ///
    /// In-flight requests continue using the pipeline snapshot they already
    /// acquired via [`ConfigHandle::pipeline`]; only new requests see the
    /// updated configuration.
    ///
    /// # Errors
    ///
    /// If reloading fails for any reason, the **existing** configuration and
    /// pipeline remain active and an error is returned describing the failure.
    pub fn reload(&self) -> Result<(), ConfigLoadError> {
        let new_config = load_config(&self.path)?;
        let new_pipeline = build_pipeline(&new_config)?;

        self.config.store(Arc::new(new_config));
        self.pipeline.store(Arc::new(new_pipeline));

        tracing::info!(path = %self.path.display(), "configuration reloaded");
        Ok(())
    }

    /// Get a snapshot of the current pipeline.
    ///
    /// The returned `Arc` is a point-in-time snapshot; subsequent calls to
    /// [`ConfigHandle::reload`] will not affect a snapshot already held by
    /// an in-flight request.
    pub fn pipeline(&self) -> Arc<Pipeline> {
        self.pipeline.load_full()
    }

    /// Get a snapshot of the current configuration.
    pub fn config(&self) -> Arc<Config> {
        self.config.load_full()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_temp_config(contents: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(contents.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    const MINIMAL: &str = r#"
        [server]
        listen_addr = "0.0.0.0:8080"
        upstream_url = "https://api.openai.com"
    "#;

    #[test]
    fn test_load_minimal_config() {
        let f = write_temp_config(MINIMAL);
        let config = load_config(f.path()).unwrap();
        assert_eq!(config.server.upstream_url, "https://api.openai.com");
    }

    #[test]
    fn test_load_missing_file_errors() {
        let result = load_config("/nonexistent/path/guardrail.toml");
        assert!(matches!(result, Err(ConfigLoadError::Io { .. })));
    }

    #[test]
    fn test_load_invalid_toml_errors() {
        let f = write_temp_config("this is not valid toml {{{");
        let result = load_config(f.path());
        assert!(matches!(result, Err(ConfigLoadError::Parse { .. })));
    }

    #[test]
    fn test_load_invalid_semantics_errors() {
        let f = write_temp_config(
            r#"
            [server]
            listen_addr = "not-valid"
            upstream_url = "https://api.openai.com"
            "#,
        );
        let result = load_config(f.path());
        assert!(matches!(result, Err(ConfigLoadError::Validation(_))));
    }

    #[test]
    fn test_build_pipeline_minimal() {
        let config: Config = toml::from_str(MINIMAL).unwrap();
        let pipeline = build_pipeline(&config).unwrap();
        // regex_injection + pii_redaction enabled by default
        assert_eq!(pipeline.len(), 2);
    }

    #[test]
    fn test_build_pipeline_with_policy() {
        let toml_str = r#"
            [server]
            listen_addr = "0.0.0.0:8080"
            upstream_url = "https://api.openai.com"

            [[policy.rules]]
            name = "test-rule"
            action = "block"
            condition.type = "always"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        let pipeline = build_pipeline(&config).unwrap();
        // regex_injection + pii_redaction + policy_engine
        assert_eq!(pipeline.len(), 3);
    }

    #[test]
    fn test_config_handle_load_and_reload() {
        let f = write_temp_config(MINIMAL);
        let handle = ConfigHandle::load(f.path()).unwrap();
        assert_eq!(handle.pipeline().len(), 2);

        // Reload with same content should succeed.
        handle.reload().unwrap();
        assert_eq!(handle.pipeline().len(), 2);
    }

    #[test]
    fn test_config_handle_pipeline_snapshot_stable() {
        let f = write_temp_config(MINIMAL);
        let handle = ConfigHandle::load(f.path()).unwrap();
        let snapshot1 = handle.pipeline();
        handle.reload().unwrap();
        let snapshot2 = handle.pipeline();

        // Both snapshots are valid pipelines (length unchanged since config didn't change)
        assert_eq!(snapshot1.len(), snapshot2.len());
    }

    #[test]
    fn test_disabled_stages_produce_empty_pipeline() {
        let toml_str = r#"
            [server]
            listen_addr = "0.0.0.0:8080"
            upstream_url = "https://api.openai.com"

            [stages.regex_injection]
            enabled = false

            [stages.pii_redaction]
            enabled = false
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        let pipeline = build_pipeline(&config).unwrap();
        assert_eq!(pipeline.len(), 0);
    }
}
