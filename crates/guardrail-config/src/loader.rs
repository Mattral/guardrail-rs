//! Configuration loading, hot-reload, and pipeline construction.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use arc_swap::ArcSwap;
use guardrail_classifiers::{PiiRedactor, RegexInjectionScanner};
use guardrail_core::{
    pipeline::{Pipeline, PipelineBuilder},
    policy::{PolicyAction, PolicyCondition, PolicyEngine, PolicyRule},
};

use crate::schema::{Config, PiiEntityList};

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

/// Load a [`Config`] from a TOML file, then apply environment-variable
/// overrides, and finally validate.
///
/// ## Environment variable overlay
///
/// The following variables, if set, override the corresponding TOML fields:
///
/// | Variable | Overrides |
/// |----------|-----------|
/// | `GUARDRAIL_CONFIG` | *(path; handled by the caller — not a config field)* |
/// | `GUARDRAIL_UPSTREAM` | `upstream.url` |
/// | `GUARDRAIL_PORT` | `server.listen_addr()` port component |
/// | `GUARDRAIL_LOG_LEVEL` | `observability.log_level` |
/// | `GUARDRAIL_OTLP_ENDPOINT` | `observability.otlp_endpoint` |
///
/// # Errors
///
/// Returns [`ConfigLoadError::Io`] if the file cannot be read,
/// [`ConfigLoadError::Parse`] if it isn't valid TOML, or
/// [`ConfigLoadError::Validation`] if validation fails after env overrides.
///
/// # Examples
///
/// ```rust,no_run
/// use guardrail_config::loader::load_config;
///
/// // GUARDRAIL_UPSTREAM=https://api.anthropic.com overrides upstream_url at runtime.
/// let config = load_config("guardrail.toml").unwrap();
/// ```
pub fn load_config(path: impl AsRef<Path>) -> Result<Config, ConfigLoadError> {
    let path = path.as_ref();
    let contents = std::fs::read_to_string(path).map_err(|source| ConfigLoadError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    let mut config: Config =
        toml::from_str(&contents).map_err(|source| ConfigLoadError::Parse {
            path: path.to_path_buf(),
            source,
        })?;

    // Apply environment-variable overrides (later layers win over TOML).
    apply_env_overrides(&mut config);

    let errors = crate::validate::validate_config(&config);
    if !errors.is_empty() {
        return Err(ConfigLoadError::Validation(errors));
    }

    Ok(config)
}

/// Apply environment-variable overrides to a partially-constructed [`Config`].
///
/// Recognized variables: `GUARDRAIL_UPSTREAM`, `GUARDRAIL_PORT`,
/// `GUARDRAIL_LOG_LEVEL`, `GUARDRAIL_OTLP_ENDPOINT`.
///
/// Unknown or malformed values are silently ignored; validation runs
/// afterwards and will catch issues.
fn apply_env_overrides(config: &mut Config) {
    if let Ok(upstream) = std::env::var("GUARDRAIL_UPSTREAM") {
        if !upstream.trim().is_empty() {
            tracing::debug!(env = "GUARDRAIL_UPSTREAM", value = %upstream, "applying env override");
            config.upstream.url = upstream;
        }
    }

    if let Ok(port_str) = std::env::var("GUARDRAIL_PORT") {
        if let Ok(port) = port_str.trim().parse::<u16>() {
            tracing::debug!(env = "GUARDRAIL_PORT", port, "applying env override");
            config.server.port = port;
        }
    }

    if let Ok(level) = std::env::var("GUARDRAIL_LOG_LEVEL") {
        if !level.trim().is_empty() {
            tracing::debug!(
                env = "GUARDRAIL_LOG_LEVEL",
                value = %level,
                "applying env override"
            );
            config.observability.log_level = level;
        }
    }

    if let Ok(endpoint) = std::env::var("GUARDRAIL_OTLP_ENDPOINT") {
        if !endpoint.trim().is_empty() {
            tracing::debug!(
                env = "GUARDRAIL_OTLP_ENDPOINT",
                value = %endpoint,
                "applying env override"
            );
            config.observability.otlp_endpoint = endpoint;
        }
    }
}

/// Build a [`Pipeline`] from a validated [`Config`].
///
/// Stages are added in the order specified by `config.pipeline.request_stages`.
/// Unknown stage IDs are skipped (validation catches them beforehand).
///
/// # Errors
///
/// Returns [`ConfigLoadError::StageBuild`] if a stage's patterns fail to compile.
pub fn build_pipeline(config: &Config) -> Result<Pipeline, ConfigLoadError> {
    let mut builder = PipelineBuilder::default();

    for stage_id in &config.pipeline.request_stages {
        match stage_id.as_str() {
            "regex_injection" if config.stages.regex_injection.enabled => {
                let log_only =
                    config.stages.regex_injection.action == crate::schema::StageAction::LogOnly;

                let scanner = if !config.stages.regex_injection.rules_file.trim().is_empty() {
                    let path = &config.stages.regex_injection.rules_file;
                    let mut contents =
                        std::fs::read_to_string(path).map_err(|source| ConfigLoadError::Io {
                            path: PathBuf::from(path),
                            source,
                        })?;
                    // Append extra_rules to the file content.
                    for rule in &config.stages.regex_injection.extra_rules {
                        contents.push('\n');
                        contents.push_str(rule);
                    }
                    RegexInjectionScanner::from_rule_str(&contents, !log_only).map_err(
                        |source| ConfigLoadError::StageBuild {
                            stage: "regex_injection".into(),
                            source,
                        },
                    )?
                } else {
                    let bundled = include_str!("injection.rules");
                    let mut bundled_rules = bundled.to_string();
                    for rule in &config.stages.regex_injection.extra_rules {
                        bundled_rules.push('\n');
                        bundled_rules.push_str(rule);
                    }
                    RegexInjectionScanner::from_rule_str(&bundled_rules, !log_only).map_err(
                        |source| ConfigLoadError::StageBuild {
                            stage: "regex_injection".into(),
                            source,
                        },
                    )?
                };
                builder = builder.stage(scanner);
            }

            #[cfg(feature = "onnx")]
            "onnx_injection" if config.stages.onnx_injection.enabled => {
                let model_path = &config.stages.onnx_injection.model_path;
                let tokenizer_path = &config.stages.onnx_injection.tokenizer_path;
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

            "pii_redactor" => {
                if let Some(redactor) = build_pii_redactor(config)? {
                    builder = builder.stage(redactor);
                }
            }

            #[cfg(feature = "onnx")]
            "toxicity" if config.stages.toxicity.enabled => {
                let model_path = &config.stages.toxicity.model_path;
                let tokenizer_path = &config.stages.toxicity.tokenizer_path;
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

            "policy" if !config.policy.rules.is_empty() => {
                let rules = convert_policy_rules(&config.policy.rules);
                builder = builder.stage(PolicyEngine::new(rules));
            }

            _ => {} // disabled or unknown stage — skip
        }
    }

    Ok(builder.build())
}

/// Convert schema `PolicyRuleConfig` entries (using `when`/`then` shape) to
/// the core `PolicyRule` type.
fn convert_policy_rules(rules: &[crate::schema::PolicyRuleConfig]) -> Vec<PolicyRule> {
    rules
        .iter()
        .map(|r| {
            let condition = {
                let w = &r.when;
                if w.always {
                    PolicyCondition::Always
                } else if !w.content_contains.is_empty() {
                    PolicyCondition::ContentContains(
                        w.content_contains
                            .iter()
                            .map(|k| k.to_lowercase())
                            .collect(),
                    )
                } else if w.system_prompt_absent {
                    PolicyCondition::SystemPromptAbsent
                } else if w.token_count_exceeds > 0 {
                    PolicyCondition::TokenCountExceeds(w.token_count_exceeds)
                } else {
                    PolicyCondition::Always
                }
            };

            let action = match r.then.action {
                crate::schema::PolicyAction::Allow => PolicyAction::Allow,
                crate::schema::PolicyAction::Redact => PolicyAction::Redact,
                crate::schema::PolicyAction::Block => PolicyAction::Block,
                crate::schema::PolicyAction::LogOnly => PolicyAction::LogOnly,
            };

            PolicyRule {
                name: r.name.clone(),
                enabled: r.enabled,
                condition,
                action,
                message: r.then.message.clone(),
            }
        })
        .collect()
}

/// Construct the [`PiiRedactor`] used by the **request**-side
/// `pii_redactor` stage, if enabled.
///
/// Returns `Ok(None)` if `config.stages.pii_redactor.enabled` is `false`.
///
/// # Errors
///
/// Returns [`ConfigLoadError::StageBuild`] if the entity list is invalid or
/// the underlying regex patterns fail to compile.
fn build_pii_redactor(config: &Config) -> Result<Option<PiiRedactor>, ConfigLoadError> {
    if !config.stages.pii_redactor.enabled {
        return Ok(None);
    }

    let entities =
        PiiEntityList::from_strings(&config.stages.pii_redactor.entities).map_err(|e| {
            ConfigLoadError::StageBuild {
                stage: "pii_redactor".into(),
                source: guardrail_core::error::GuardrailError::Config(e),
            }
        })?;

    let redactor = PiiRedactor::new(entities.0, config.stages.pii_redactor.validate_luhn).map_err(
        |source| ConfigLoadError::StageBuild {
            stage: "pii_redactor".into(),
            source,
        },
    )?;

    Ok(Some(redactor))
}

/// Construct the [`PiiRedactor`] used for **response**-side redaction, if
/// both `stages.pii_redactor.enabled` and `stages.pii_redactor.redact_responses`
/// are `true`.
///
/// This intentionally shares the same entity list, Luhn-validation setting,
/// and replacement tokens as the request-side redactor — there is no
/// separate configuration surface for response redaction beyond the
/// `redact_responses` toggle, so that enabling output redaction can never
/// silently use different (e.g. weaker) rules than the input side.
///
/// # Errors
///
/// Returns [`ConfigLoadError::StageBuild`] under the same conditions as
/// `build_pii_redactor`.
///
/// # Examples
///
/// ```rust
/// use guardrail_config::loader::build_response_redactor;
/// use guardrail_config::Config;
///
/// let toml_str = r#"
/// [server]
/// host = "0.0.0.0"
/// port = 8080
///
/// [upstream]
/// url = "https://api.openai.com"
///
/// [stages.pii_redactor]
/// enabled = true
/// redact_responses = true
/// "#;
///
/// let config: Config = toml::from_str(toml_str).unwrap();
/// let redactor = build_response_redactor(&config).unwrap();
/// assert!(redactor.is_some());
/// ```
pub fn build_response_redactor(config: &Config) -> Result<Option<PiiRedactor>, ConfigLoadError> {
    if !config.stages.pii_redactor.redact_responses {
        return Ok(None);
    }
    build_pii_redactor(config)
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
    /// `Some(redactor)` if response-side PII redaction is enabled; `None` otherwise.
    /// Wrapped in an extra `Option` layer so `ArcSwap` always holds a value
    /// (an empty `Arc<None>` when response redaction is disabled).
    response_redactor: ArcSwap<Option<PiiRedactor>>,
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
        let response_redactor = build_response_redactor(&config)?;

        Ok(Self {
            path,
            config: ArcSwap::from_pointee(config),
            pipeline: ArcSwap::from_pointee(pipeline),
            response_redactor: ArcSwap::from_pointee(response_redactor),
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
        let new_response_redactor = build_response_redactor(&new_config)?;

        self.config.store(Arc::new(new_config));
        self.pipeline.store(Arc::new(new_pipeline));
        self.response_redactor
            .store(Arc::new(new_response_redactor));

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

    /// Get a snapshot of the response-side PII redactor configuration.
    ///
    /// Returns an `Arc<Option<PiiRedactor>>`: `Arc::new(None)` if response
    /// redaction is disabled (`stages.pii_redactor.enabled = false` or
    /// `stages.pii_redactor.redact_responses = false`), or
    /// `Arc::new(Some(redactor))` otherwise. Callers typically do:
    ///
    /// ```rust,no_run
    /// # use guardrail_config::ConfigHandle;
    /// # let handle = ConfigHandle::load("guardrail.toml").unwrap();
    /// let redactor_snapshot = handle.response_redactor();
    /// if let Some(redactor) = (*redactor_snapshot).as_ref() {
    ///     // redact response body using `redactor`
    ///     let _ = redactor;
    /// }
    /// ```
    pub fn response_redactor(&self) -> Arc<Option<PiiRedactor>> {
        self.response_redactor.load_full()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validate_config;
    use std::io::Write;

    /// Guards every test that either mutates `GUARDRAIL_UPSTREAM` /
    /// `GUARDRAIL_PORT` / `GUARDRAIL_LOG_LEVEL` / `GUARDRAIL_OTLP_ENDPOINT`
    /// (via `std::env::set_var`) or calls [`load_config`] expecting a
    /// specific field value / validation outcome that such an override
    /// could silently change out from under it. `std::env::set_var` mutates
    /// process-global state, and `cargo test`/`nextest` run test functions
    /// in parallel by default — without this, `test_env_override_behavior`
    /// setting e.g. `GUARDRAIL_PORT=9999` can interleave with
    /// `test_load_minimal_config` asserting `port == 8080` on a different
    /// thread and fail it nondeterministically (as it did in CI:
    /// `test_load_minimal_config` saw port 9999, and
    /// `test_load_invalid_semantics_errors` saw its intentionally-invalid
    /// `ftp://` upstream URL silently "fixed" by a concurrently-applied
    /// `GUARDRAIL_UPSTREAM` override, so the validation error it expected
    /// never fired).
    ///
    /// `.unwrap_or_else(|poisoned| poisoned.into_inner())` recovers from
    /// poisoning: if some other test panics while holding this lock, that
    /// failure is already reported on its own, and there's no shared data
    /// here to actually be left inconsistent, so there's no reason to
    /// cascade the failure into every other test that happens to run
    /// afterwards.
    static ENV_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn lock_env_tests() -> std::sync::MutexGuard<'static, ()> {
        ENV_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn write_temp_config(contents: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(contents.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    const MINIMAL: &str = r#"
[server]
host = "0.0.0.0"
port = 8080

[upstream]
url = "https://api.openai.com"
"#;

    #[test]
    fn test_load_minimal_config() {
        let _guard = lock_env_tests();
        let f = write_temp_config(MINIMAL);
        let config = load_config(f.path()).unwrap();
        assert_eq!(config.upstream.url, "https://api.openai.com");
        assert_eq!(config.server.port, 8080);
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
        let _guard = lock_env_tests();
        // Bad upstream scheme triggers validation failure.
        let f = write_temp_config(
            r#"
[server]
host = "0.0.0.0"
port = 8080

[upstream]
url = "ftp://example.com"
"#,
        );
        let result = load_config(f.path());
        assert!(matches!(result, Err(ConfigLoadError::Validation(_))));
    }

    #[test]
    fn test_build_pipeline_minimal() {
        let config: Config = toml::from_str(MINIMAL).unwrap();
        let pipeline = build_pipeline(&config).unwrap();
        // Default request_stages = [regex_injection, onnx_injection, pii_redactor, toxicity, policy]
        // Only enabled ones run: regex_injection + pii_redactor = 2
        assert_eq!(pipeline.len(), 2);
    }

    #[test]
    fn test_build_pipeline_with_policy() {
        let toml_str = r#"
[server]
host = "0.0.0.0"
port = 8080

[upstream]
url = "https://api.openai.com"

[[policy.rules]]
name = "test-rule"
[policy.rules.when]
always = true
[policy.rules.then]
action = "block"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        let pipeline = build_pipeline(&config).unwrap();
        // regex_injection + pii_redactor + policy = 3
        assert_eq!(pipeline.len(), 3);
    }

    #[test]
    fn test_config_handle_load_and_reload() {
        let f = write_temp_config(MINIMAL);
        let handle = ConfigHandle::load(f.path()).unwrap();
        assert_eq!(handle.pipeline().len(), 2);
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
        assert_eq!(snapshot1.len(), snapshot2.len());
    }

    #[test]
    fn test_disabled_stages_produce_empty_pipeline() {
        let toml_str = r#"
[server]
host = "0.0.0.0"
port = 8080

[upstream]
url = "https://api.openai.com"

[stages.regex_injection]
enabled = false

[stages.pii_redactor]
enabled = false
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        let pipeline = build_pipeline(&config).unwrap();
        assert_eq!(pipeline.len(), 0);
    }

    // ── Response-side redaction ─────────────────────────────────────────────

    #[test]
    fn test_response_redactor_disabled_by_default() {
        let config: Config = toml::from_str(MINIMAL).unwrap();
        assert!(!config.stages.pii_redactor.redact_responses);
        let redactor = build_response_redactor(&config).unwrap();
        assert!(redactor.is_none());
    }

    #[test]
    fn test_response_redactor_enabled() {
        let toml_str = r#"
[server]
host = "0.0.0.0"
port = 8080

[upstream]
url = "https://api.openai.com"

[stages.pii_redactor]
enabled = true
redact_responses = true
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        let redactor = build_response_redactor(&config).unwrap();
        assert!(redactor.is_some());
    }

    #[test]
    fn test_response_redactor_not_built_if_pii_stage_disabled() {
        let toml_str = r#"
[server]
host = "0.0.0.0"
port = 8080

[upstream]
url = "https://api.openai.com"

[stages.pii_redactor]
enabled = false
redact_responses = true
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        let redactor = build_response_redactor(&config).unwrap();
        assert!(redactor.is_none());
    }

    #[test]
    fn test_config_handle_response_redactor_default_none() {
        let f = write_temp_config(MINIMAL);
        let handle = ConfigHandle::load(f.path()).unwrap();
        assert!(handle.response_redactor().is_none());
    }

    // ── Policy rule with when/then shape ────────────────────────────────────

    #[test]
    fn test_policy_rule_when_then_content_contains() {
        let toml_str = r#"
[server]
host = "0.0.0.0"
port = 8080

[upstream]
url = "https://api.openai.com"

[[policy.rules]]
name = "block-competitor"
[policy.rules.when]
content_contains = ["competitor-x"]
[policy.rules.then]
action = "block"
message = "Not permitted."
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(validate_config(&config).is_empty());
        let pipeline = build_pipeline(&config).unwrap();
        // regex + pii + policy
        assert_eq!(pipeline.len(), 3);
    }

    // ── Environment variable overlay ────────────────────────────────────────
    // All five assertions live in a single test function rather than five
    // separate `#[test]` functions, because `std::env::set_var`/`remove_var`
    // mutate process-global state and cargo test/nextest run different test
    // functions in parallel by default. Two functions touching the same env
    // var name concurrently (GUARDRAIL_UPSTREAM is used by both the
    // "set a value" and "empty string is ignored" cases; GUARDRAIL_PORT by
    // both the "set a value" and "invalid value is ignored" cases) would
    // otherwise race against each other and produce flaky failures.

    #[test]
    fn test_env_override_behavior() {
        let _guard = lock_env_tests();
        let f = write_temp_config(MINIMAL);

        // 1. GUARDRAIL_UPSTREAM overrides upstream.url.
        std::env::set_var("GUARDRAIL_UPSTREAM", "https://api.anthropic.com");
        let result = load_config(f.path());
        std::env::remove_var("GUARDRAIL_UPSTREAM");
        assert_eq!(result.unwrap().upstream.url, "https://api.anthropic.com");

        // 2. GUARDRAIL_PORT overrides server.port.
        std::env::set_var("GUARDRAIL_PORT", "9999");
        let result = load_config(f.path());
        std::env::remove_var("GUARDRAIL_PORT");
        assert_eq!(result.unwrap().server.port, 9999);

        // 3. GUARDRAIL_LOG_LEVEL overrides observability.log_level.
        std::env::set_var("GUARDRAIL_LOG_LEVEL", "debug");
        let result = load_config(f.path());
        std::env::remove_var("GUARDRAIL_LOG_LEVEL");
        assert_eq!(result.unwrap().observability.log_level, "debug");

        // 4. An empty-string env var must NOT override the TOML value.
        std::env::set_var("GUARDRAIL_UPSTREAM", "");
        let result = load_config(f.path());
        std::env::remove_var("GUARDRAIL_UPSTREAM");
        assert_eq!(result.unwrap().upstream.url, "https://api.openai.com");

        // 5. An unparseable port value must be silently ignored, not error.
        std::env::set_var("GUARDRAIL_PORT", "notaport");
        let result = load_config(f.path());
        std::env::remove_var("GUARDRAIL_PORT");
        assert_eq!(result.unwrap().server.port, 8080);
    }

    /// `injection.rules` here and `guardrail-classifiers/src/rules/injection.rules`
    /// are two physically separate files that are supposed to define the
    /// *same* default rule set (see the module comment atop this crate's
    /// own `injection.rules` for why they aren't shared via a single file).
    /// They drifted silently once already — this test, plus the
    /// `injection-rules-sync` CI job, are the two independent guards
    /// against it happening again.
    ///
    /// Deliberately reads both files at runtime via `CARGO_MANIFEST_DIR`
    /// rather than `include_str!`: the sibling crate's file lives outside
    /// this crate's own package directory, so it won't exist if this crate
    /// is ever built in isolation (e.g. from an extracted `cargo package`
    /// tarball) — a scenario where `include_str!` would be a hard compile
    /// error. This test instead skips itself in that case; the workspace
    /// checkout used by `just test` / CI always has both files, so the
    /// check still runs everywhere it matters.
    #[test]
    fn bundled_injection_rules_match_guardrail_classifiers_copy() {
        let config_copy = include_str!("injection.rules");

        let classifiers_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../guardrail-classifiers/src/rules/injection.rules");
        let Ok(classifiers_copy) = std::fs::read_to_string(&classifiers_path) else {
            eprintln!(
                "skipping: {} not present (expected when this crate is built \
                 outside the guardrail-rs workspace checkout)",
                classifiers_path.display()
            );
            return;
        };

        fn strip_noise(s: &str) -> Vec<&str> {
            s.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .collect()
        }

        assert_eq!(
            strip_noise(config_copy),
            strip_noise(&classifiers_copy),
            "guardrail-config/src/injection.rules and \
             guardrail-classifiers/src/rules/injection.rules have diverged — \
             update whichever one is stale so both crates ship the same \
             default prompt-injection rule set"
        );
    }
}
