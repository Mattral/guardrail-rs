//! Configuration validation — semantic checks beyond what `serde` can express.

use crate::schema::Config;

/// Validate a [`Config`] for internal consistency.
///
/// Collects and returns all errors rather than stopping at the first.
///
/// # Examples
///
/// ```rust
/// use guardrail_config::{validate_config, Config};
///
/// let toml_str = r#"
/// [server]
/// host = "0.0.0.0"
/// port = 8080
/// "#;
/// let config: Config = toml::from_str(toml_str).unwrap();
/// let errors = validate_config(&config);
/// assert!(errors.is_empty());
/// ```
pub fn validate_config(config: &Config) -> Vec<String> {
    let mut errors = Vec::new();

    // ── Server ────────────────────────────────────────────────────────────
    let listen_addr = config.server.listen_addr();
    if listen_addr.parse::<std::net::SocketAddr>().is_err() {
        errors.push(format!(
            "server: '{listen_addr}' is not a valid socket address (host={}, port={})",
            config.server.host, config.server.port,
        ));
    }

    if config.server.max_body_size_bytes == 0 {
        errors.push("server.max_body_size_bytes must be > 0".into());
    }

    // ── Upstream ──────────────────────────────────────────────────────────
    let url = config.upstream.url.trim();
    if !url.starts_with("http://") && !url.starts_with("https://") {
        errors.push(format!(
            "upstream.url '{url}' must start with http:// or https://"
        ));
    }
    if config.upstream.timeout_secs == 0 {
        errors.push("upstream.timeout_secs must be > 0".into());
    }

    // ── Auth ──────────────────────────────────────────────────────────────
    if config.auth.require_key && config.auth.keys.is_empty() {
        errors.push(
            "auth.require_key = true but auth.keys is empty; all requests would be rejected".into(),
        );
    }
    for (i, key) in config.auth.keys.iter().enumerate() {
        if key.trim().is_empty() {
            errors.push(format!("auth.keys[{i}] must not be blank"));
        }
    }

    // ── Pipeline stage IDs ────────────────────────────────────────────────
    const KNOWN_REQUEST_STAGES: &[&str] = &[
        "regex_injection",
        "onnx_injection",
        "pii_redactor",
        "toxicity",
        "policy",
    ];
    const KNOWN_RESPONSE_STAGES: &[&str] = &["output_pii_redactor"];

    for id in &config.pipeline.request_stages {
        if !KNOWN_REQUEST_STAGES.contains(&id.as_str()) {
            errors.push(format!(
                "pipeline.request_stages: unknown stage id '{id}'. Known: {}",
                KNOWN_REQUEST_STAGES.join(", ")
            ));
        }
    }
    for id in &config.pipeline.response_stages {
        if !KNOWN_RESPONSE_STAGES.contains(&id.as_str()) {
            errors.push(format!(
                "pipeline.response_stages: unknown stage id '{id}'. Known: {}",
                KNOWN_RESPONSE_STAGES.join(", ")
            ));
        }
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
    if config.stages.pii_redactor.enabled {
        for entity in &config.stages.pii_redactor.entities {
            if !VALID_ENTITIES.contains(&entity.as_str()) {
                errors.push(format!(
                    "stages.pii_redactor.entities: unknown entity type '{entity}'. \
                     Valid types: {}",
                    VALID_ENTITIES.join(", ")
                ));
            }
        }
        if config.stages.pii_redactor.entities.is_empty() {
            errors.push("stages.pii_redactor is enabled but entities list is empty".into());
        }
    }

    // ── ONNX stages ───────────────────────────────────────────────────────
    for (stage_name, enabled, model_path, _tokenizer_path, threshold) in [
        (
            "onnx_injection",
            config.stages.onnx_injection.enabled,
            &config.stages.onnx_injection.model_path,
            &config.stages.onnx_injection.tokenizer_path,
            config.stages.onnx_injection.threshold,
        ),
        (
            "toxicity",
            config.stages.toxicity.enabled,
            &config.stages.toxicity.model_path,
            &config.stages.toxicity.tokenizer_path,
            config.stages.toxicity.threshold,
        ),
    ] {
        if enabled {
            #[cfg(not(feature = "onnx"))]
            errors.push(format!(
                "stages.{stage_name} is enabled but this build lacks the 'onnx' feature"
            ));

            if !model_path.trim().is_empty() && !std::path::Path::new(model_path).exists() {
                errors.push(format!(
                    "stages.{stage_name}.model_path '{model_path}' does not exist"
                ));
            }
            if !(0.0..=1.0).contains(&threshold) {
                errors.push(format!(
                    "stages.{stage_name}.threshold must be in [0.0, 1.0], got {threshold}"
                ));
            }
        }
    }

    // ── Policy rules ──────────────────────────────────────────────────────
    for (i, rule) in config.policy.rules.iter().enumerate() {
        if rule.name.trim().is_empty() {
            errors.push(format!("policy.rules[{i}].name must not be empty"));
        }
        let cond = &rule.when;
        let any_condition = !cond.content_contains.is_empty()
            || cond.system_prompt_absent
            || cond.token_count_exceeds > 0
            || cond.always;
        if !any_condition {
            errors.push(format!(
                "policy.rules[{i}] ('{}') has no condition set (when.* must have \
                 at least one non-default value)",
                rule.name
            ));
        }
        if !cond.content_contains.is_empty()
            && cond.content_contains.iter().all(|k| k.trim().is_empty())
        {
            errors.push(format!(
                "policy.rules[{i}] ('{}') when.content_contains contains only blank strings",
                rule.name
            ));
        }
    }

    // ── Observability ─────────────────────────────────────────────────────
    const VALID_LOG_LEVELS: &[&str] = &["trace", "debug", "info", "warn", "error"];
    if !VALID_LOG_LEVELS.contains(&config.observability.log_level.as_str()) {
        errors.push(format!(
            "observability.log_level '{}' is not valid. Valid: {}",
            config.observability.log_level,
            VALID_LOG_LEVELS.join(", ")
        ));
    }

    const VALID_LOG_FORMATS: &[&str] = &["pretty", "json"];
    if !VALID_LOG_FORMATS.contains(&config.observability.log_format.as_str()) {
        errors.push(format!(
            "observability.log_format '{}' is not valid. Valid: pretty, json",
            config.observability.log_format,
        ));
    }

    let endpoint = config.observability.otlp_endpoint.trim();
    if !endpoint.is_empty()
        && !endpoint.starts_with("http://")
        && !endpoint.starts_with("https://")
        && !endpoint.starts_with("grpc://")
    {
        errors.push(format!(
            "observability.otlp_endpoint '{endpoint}' must start with \
             http://, https://, or grpc://"
        ));
    }

    // ── Audit log ─────────────────────────────────────────────────────────
    if config.observability.audit_log.enabled {
        if config.observability.audit_log.path.trim().is_empty() {
            errors.push(
                "observability.audit_log.path must not be empty when audit_log is enabled".into(),
            );
        }
        if config.observability.audit_log.max_size_mb == 0 {
            errors.push("observability.audit_log.max_size_mb must be > 0".into());
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal() -> &'static str {
        r#"
        [server]
        host = "0.0.0.0"
        port = 8080

        [upstream]
        url = "https://api.openai.com"
        "#
    }

    #[test]
    fn test_minimal_config_is_valid() {
        let config: Config = toml::from_str(minimal()).unwrap();
        assert!(validate_config(&config).is_empty());
    }

    #[test]
    fn test_invalid_port_zero_is_valid_socket_addr() {
        // Port 0 means OS-assigned; still a valid SocketAddr
        let toml = r#"[server]
host = "0.0.0.0"
port = 0

[upstream]
url = "https://api.openai.com"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(validate_config(&config).is_empty());
    }

    #[test]
    fn test_invalid_upstream_scheme() {
        let toml = r#"[server]
host = "0.0.0.0"
port = 8080

[upstream]
url = "ftp://example.com"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        let errors = validate_config(&config);
        assert!(errors.iter().any(|e| e.contains("upstream.url")));
    }

    #[test]
    fn test_auth_require_key_with_empty_keys_errors() {
        let toml = r#"[server]
host = "0.0.0.0"
port = 8080

[upstream]
url = "https://api.openai.com"

[auth]
require_key = true
"#;
        let config: Config = toml::from_str(toml).unwrap();
        let errors = validate_config(&config);
        assert!(errors.iter().any(|e| e.contains("auth.keys")));
    }

    #[test]
    fn test_auth_require_key_with_keys_is_valid() {
        let toml = r#"[server]
host = "0.0.0.0"
port = 8080

[upstream]
url = "https://api.openai.com"

[auth]
require_key = true
keys = ["grk-test-key-1"]
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(validate_config(&config).is_empty());
    }

    #[test]
    fn test_unknown_request_stage_errors() {
        let toml = r#"[server]
host = "0.0.0.0"
port = 8080

[upstream]
url = "https://api.openai.com"

[pipeline]
request_stages = ["regex_injection", "banana_stage"]
"#;
        let config: Config = toml::from_str(toml).unwrap();
        let errors = validate_config(&config);
        assert!(errors.iter().any(|e| e.contains("banana_stage")));
    }

    #[test]
    fn test_invalid_pii_entity() {
        let toml = r#"[server]
host = "0.0.0.0"
port = 8080

[upstream]
url = "https://api.openai.com"

[stages.pii_redactor]
enabled = true
entities = ["email", "bogus"]
"#;
        let config: Config = toml::from_str(toml).unwrap();
        let errors = validate_config(&config);
        assert!(errors.iter().any(|e| e.contains("bogus")));
    }

    #[test]
    fn test_policy_rule_no_condition_errors() {
        let toml = r#"[server]
host = "0.0.0.0"
port = 8080

[upstream]
url = "https://api.openai.com"

[[policy.rules]]
name = "empty-rule"
[policy.rules.when]
[policy.rules.then]
action = "block"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        let errors = validate_config(&config);
        assert!(errors.iter().any(|e| e.contains("no condition")));
    }

    #[test]
    fn test_policy_rule_content_contains_is_valid() {
        let toml = r#"[server]
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
        let config: Config = toml::from_str(toml).unwrap();
        assert!(validate_config(&config).is_empty());
    }

    #[test]
    fn test_invalid_log_level_errors() {
        let toml = r#"[server]
host = "0.0.0.0"
port = 8080

[upstream]
url = "https://api.openai.com"

[observability]
log_level = "verbose"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        let errors = validate_config(&config);
        assert!(errors.iter().any(|e| e.contains("log_level")));
    }

    #[test]
    fn test_invalid_otlp_scheme_errors() {
        let toml = r#"[server]
host = "0.0.0.0"
port = 8080

[upstream]
url = "https://api.openai.com"

[observability]
otlp_endpoint = "ftp://localhost:4317"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        let errors = validate_config(&config);
        assert!(errors.iter().any(|e| e.contains("otlp_endpoint")));
    }

    #[test]
    fn test_audit_log_valid() {
        let toml = r#"[server]
host = "0.0.0.0"
port = 8080

[upstream]
url = "https://api.openai.com"

[observability.audit_log]
enabled = true
path = "/var/log/guardrail/audit.ndjson"
max_size_mb = 100
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(validate_config(&config).is_empty());
    }

    #[test]
    fn test_audit_log_empty_path_errors() {
        let toml = r#"[server]
host = "0.0.0.0"
port = 8080

[upstream]
url = "https://api.openai.com"

[observability.audit_log]
enabled = true
path = ""
"#;
        let config: Config = toml::from_str(toml).unwrap();
        let errors = validate_config(&config);
        assert!(errors.iter().any(|e| e.contains("audit_log.path")));
    }
}
