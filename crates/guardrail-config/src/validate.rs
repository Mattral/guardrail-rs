//! Configuration validation.

use crate::schema::{Config, PolicyConditionConfig};

/// Validate a [`Config`] for internal consistency.
///
/// This is run after TOML deserialization and before the pipeline is built.
/// Validation catches errors that the type system alone cannot, such as:
///
/// - Invalid PII entity names
/// - ONNX stages enabled without model paths
/// - Threshold values out of `[0.0, 1.0]`
/// - Empty policy rule names
/// - Listen address that fails to parse as a socket address
///
/// # Errors
///
/// Returns a `Vec<String>` of human-readable error messages. An empty `Vec`
/// means the configuration is valid.
///
/// # Examples
///
/// ```rust
/// use guardrail_config::{validate_config, Config};
///
/// let toml_str = r#"
/// [server]
/// listen_addr = "0.0.0.0:8080"
/// upstream_url = "https://api.openai.com"
/// "#;
///
/// let config: Config = toml::from_str(toml_str).unwrap();
/// let errors = validate_config(&config);
/// assert!(errors.is_empty());
/// ```
pub fn validate_config(config: &Config) -> Vec<String> {
    let mut errors = Vec::new();

    // ── Server ────────────────────────────────────────────────────────────
    if config.server.listen_addr.parse::<std::net::SocketAddr>().is_err() {
        errors.push(format!(
            "server.listen_addr '{}' is not a valid socket address (expected host:port)",
            config.server.listen_addr
        ));
    }

    if !config.server.upstream_url.starts_with("http://")
        && !config.server.upstream_url.starts_with("https://")
    {
        errors.push(format!(
            "server.upstream_url '{}' must start with http:// or https://",
            config.server.upstream_url
        ));
    }

    if config.server.max_body_size_bytes == 0 {
        errors.push("server.max_body_size_bytes must be greater than 0".into());
    }

    // ── PII entities ──────────────────────────────────────────────────────
    const VALID_ENTITIES: &[&str] = &[
        "email",
        "phone",
        "credit_card",
        "ssn",
        "ip_address",
        "api_key",
        "aws_key",
    ];

    if config.stages.pii_redaction.enabled {
        for entity in &config.stages.pii_redaction.entities {
            if !VALID_ENTITIES.contains(&entity.as_str()) {
                errors.push(format!(
                    "stages.pii_redaction.entities contains unknown entity type '{entity}'. \
                     Valid types: {}",
                    VALID_ENTITIES.join(", ")
                ));
            }
        }

        if config.stages.pii_redaction.entities.is_empty() {
            errors.push(
                "stages.pii_redaction is enabled but entities list is empty".into(),
            );
        }
    }

    // ── ONNX stages ───────────────────────────────────────────────────────
    for (stage_name, onnx_cfg) in [
        ("onnx_injection", &config.stages.onnx_injection),
        ("toxicity", &config.stages.toxicity),
    ] {
        if onnx_cfg.enabled {
            #[cfg(not(feature = "onnx"))]
            {
                let _ = stage_name;
                errors.push(format!(
                    "stages.{stage_name} is enabled but guardrail-rs was built without the 'onnx' feature"
                ));
            }

            if onnx_cfg.model_path.is_none() {
                errors.push(format!(
                    "stages.{stage_name} is enabled but model_path is not set"
                ));
            }
            if onnx_cfg.tokenizer_path.is_none() {
                errors.push(format!(
                    "stages.{stage_name} is enabled but tokenizer_path is not set"
                ));
            }
            if !(0.0..=1.0).contains(&onnx_cfg.threshold) {
                errors.push(format!(
                    "stages.{stage_name}.threshold must be in [0.0, 1.0], got {}",
                    onnx_cfg.threshold
                ));
            }
        }
    }

    // ── Policy rules ──────────────────────────────────────────────────────
    for (i, rule) in config.policy.rules.iter().enumerate() {
        if rule.name.trim().is_empty() {
            errors.push(format!("policy.rules[{i}].name must not be empty"));
        }

        if let PolicyConditionConfig::ContentContains { keywords } = &rule.condition {
            if keywords.is_empty() {
                errors.push(format!(
                    "policy.rules[{i}] ('{}') has condition.type = \"content_contains\" \
                     but keywords is empty",
                    rule.name
                ));
            }
        }

        if let PolicyConditionConfig::TokenCountExceeds { limit } = &rule.condition {
            if *limit == 0 {
                errors.push(format!(
                    "policy.rules[{i}] ('{}') has condition.limit = 0, which would \
                     match every request",
                    rule.name
                ));
            }
        }
    }

    // ── Observability ─────────────────────────────────────────────────────
    if config
        .observability
        .metrics_addr
        .parse::<std::net::SocketAddr>()
        .is_err()
    {
        errors.push(format!(
            "observability.metrics_addr '{}' is not a valid socket address",
            config.observability.metrics_addr
        ));
    }

    const VALID_LOG_LEVELS: &[&str] = &["trace", "debug", "info", "warn", "error"];
    if !VALID_LOG_LEVELS.contains(&config.observability.log_level.as_str()) {
        errors.push(format!(
            "observability.log_level '{}' is not valid. Valid levels: {}",
            config.observability.log_level,
            VALID_LOG_LEVELS.join(", ")
        ));
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_valid_toml() -> &'static str {
        r#"
        [server]
        listen_addr = "0.0.0.0:8080"
        upstream_url = "https://api.openai.com"
        "#
    }

    #[test]
    fn test_minimal_config_is_valid() {
        let config: Config = toml::from_str(minimal_valid_toml()).unwrap();
        let errors = validate_config(&config);
        assert!(errors.is_empty(), "errors: {errors:?}");
    }

    #[test]
    fn test_invalid_listen_addr() {
        let toml_str = r#"
        [server]
        listen_addr = "not-an-addr"
        upstream_url = "https://api.openai.com"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        let errors = validate_config(&config);
        assert!(errors.iter().any(|e| e.contains("listen_addr")));
    }

    #[test]
    fn test_invalid_upstream_scheme() {
        let toml_str = r#"
        [server]
        listen_addr = "0.0.0.0:8080"
        upstream_url = "ftp://example.com"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        let errors = validate_config(&config);
        assert!(errors.iter().any(|e| e.contains("upstream_url")));
    }

    #[test]
    fn test_invalid_pii_entity() {
        let toml_str = r#"
        [server]
        listen_addr = "0.0.0.0:8080"
        upstream_url = "https://api.openai.com"

        [stages.pii_redaction]
        enabled = true
        entities = ["email", "bogus_entity"]
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        let errors = validate_config(&config);
        assert!(errors.iter().any(|e| e.contains("bogus_entity")));
    }

    #[test]
    fn test_onnx_enabled_without_model_path() {
        let toml_str = r#"
        [server]
        listen_addr = "0.0.0.0:8080"
        upstream_url = "https://api.openai.com"

        [stages.onnx_injection]
        enabled = true
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        let errors = validate_config(&config);
        assert!(errors.iter().any(|e| e.contains("model_path")));
        assert!(errors.iter().any(|e| e.contains("tokenizer_path")));
    }

    #[test]
    fn test_invalid_threshold() {
        let toml_str = r#"
        [server]
        listen_addr = "0.0.0.0:8080"
        upstream_url = "https://api.openai.com"

        [stages.onnx_injection]
        enabled = true
        model_path = "model.onnx"
        tokenizer_path = "tok.json"
        threshold = 1.5
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        let errors = validate_config(&config);
        assert!(errors.iter().any(|e| e.contains("threshold")));
    }

    #[test]
    fn test_empty_policy_rule_name() {
        let toml_str = r#"
        [server]
        listen_addr = "0.0.0.0:8080"
        upstream_url = "https://api.openai.com"

        [[policy.rules]]
        name = ""
        action = "block"
        condition.type = "always"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        let errors = validate_config(&config);
        assert!(errors.iter().any(|e| e.contains("name must not be empty")));
    }

    #[test]
    fn test_content_contains_empty_keywords() {
        let toml_str = r#"
        [server]
        listen_addr = "0.0.0.0:8080"
        upstream_url = "https://api.openai.com"

        [[policy.rules]]
        name = "test-rule"
        action = "block"
        condition.type = "content_contains"
        condition.keywords = []
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        let errors = validate_config(&config);
        assert!(errors.iter().any(|e| e.contains("keywords is empty")));
    }

    #[test]
    fn test_invalid_log_level() {
        let toml_str = r#"
        [server]
        listen_addr = "0.0.0.0:8080"
        upstream_url = "https://api.openai.com"

        [observability]
        log_level = "verbose"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        let errors = validate_config(&config);
        assert!(errors.iter().any(|e| e.contains("log_level")));
    }
}
