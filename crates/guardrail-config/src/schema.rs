//! TOML configuration schema.

use serde::Deserialize;

/// Top-level configuration structure, deserialized directly from `guardrail.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Server / proxy network settings.
    pub server: ServerConfig,
    /// Pipeline-wide behavior settings.
    #[serde(default)]
    pub pipeline: PipelineConfig,
    /// Per-stage configuration.
    #[serde(default)]
    pub stages: StagesConfig,
    /// User-defined policy rules.
    #[serde(default)]
    pub policy: PolicyConfig,
    /// Observability settings.
    #[serde(default)]
    pub observability: ObservabilityConfig,
}

/// Server and upstream connection settings.
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    /// Address to bind the proxy server to, e.g. `"0.0.0.0:8080"`.
    pub listen_addr: String,
    /// Base URL of the upstream LLM API, e.g. `"https://api.openai.com"`.
    pub upstream_url: String,
    /// Request timeout to the upstream, in seconds. Default: 60.
    #[serde(default = "default_upstream_timeout")]
    pub upstream_timeout_secs: u64,
    /// Maximum allowed request body size, in bytes. Default: 10 MiB.
    #[serde(default = "default_max_body_size")]
    pub max_body_size_bytes: usize,
}

fn default_upstream_timeout() -> u64 {
    60
}

fn default_max_body_size() -> usize {
    10 * 1024 * 1024
}

/// Pipeline-wide behavior.
#[derive(Debug, Clone, Deserialize)]
pub struct PipelineConfig {
    /// What to do when a stage returns an error.
    ///
    /// - `"allow"` (default): treat the error as `Decision::Allow` (fail-open).
    /// - `"block"`: treat the error as `Decision::Block` (fail-closed).
    #[serde(default = "default_on_error")]
    pub on_error: OnErrorBehavior,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            on_error: OnErrorBehavior::Allow,
        }
    }
}

fn default_on_error() -> OnErrorBehavior {
    OnErrorBehavior::Allow
}

/// Behavior when a pipeline stage returns an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnErrorBehavior {
    /// Fail open: treat stage errors as `Allow`.
    Allow,
    /// Fail closed: treat stage errors as `Block`.
    Block,
}

/// Per-stage configuration block.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct StagesConfig {
    /// Regex-based prompt injection scanner.
    #[serde(default)]
    pub regex_injection: RegexInjectionConfig,
    /// PII detection and redaction.
    #[serde(default)]
    pub pii_redaction: PiiRedactionConfig,
    /// ONNX-based semantic injection classifier (requires `onnx` feature).
    #[serde(default)]
    pub onnx_injection: OnnxStageConfig,
    /// ONNX-based toxicity classifier (requires `onnx` feature).
    #[serde(default)]
    pub toxicity: OnnxStageConfig,
}

/// Configuration for [`guardrail_classifiers::RegexInjectionScanner`].
#[derive(Debug, Clone, Deserialize)]
pub struct RegexInjectionConfig {
    /// Whether this stage is enabled. Default: `true`.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Path to a custom rule file. If `None`, the bundled rules are used.
    #[serde(default)]
    pub custom_rules_path: Option<String>,
    /// If `true`, matches are logged but not blocked. Default: `false`.
    #[serde(default)]
    pub log_only: bool,
}

impl Default for RegexInjectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            custom_rules_path: None,
            log_only: false,
        }
    }
}

/// Configuration for [`guardrail_classifiers::PiiRedactor`].
#[derive(Debug, Clone, Deserialize)]
pub struct PiiRedactionConfig {
    /// Whether this stage is enabled. Default: `true`.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Which entity types to detect and redact.
    ///
    /// Valid values: `"email"`, `"phone"`, `"credit_card"`, `"ssn"`,
    /// `"ip_address"`, `"api_key"`, `"aws_key"`.
    #[serde(default = "default_pii_entities")]
    pub entities: Vec<String>,
    /// Whether to apply Luhn validation to candidate credit card numbers.
    /// Default: `true`.
    #[serde(default = "default_true")]
    pub validate_luhn: bool,
    /// Whether to also scan and redact PII in non-streaming LLM **responses**
    /// before returning them to the caller. Default: `false`.
    ///
    /// This adds a JSON parse + re-serialize pass to every non-streaming
    /// response (see [`guardrail_proxy::response`]). Streaming responses
    /// (`"stream": true`) are never affected by this option — see
    /// `docs/architecture.md` for the streaming-redaction roadmap.
    ///
    /// [`guardrail_proxy::response`]: https://docs.rs/guardrail-proxy/latest/guardrail_proxy/response/index.html
    #[serde(default)]
    pub redact_responses: bool,
}

impl Default for PiiRedactionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            entities: default_pii_entities(),
            validate_luhn: true,
            redact_responses: false,
        }
    }
}

fn default_pii_entities() -> Vec<String> {
    vec![
        "email".into(),
        "phone".into(),
        "credit_card".into(),
        "ssn".into(),
        "ip_address".into(),
        "api_key".into(),
        "aws_key".into(),
    ]
}

/// Converts a list of entity-type strings (as used in TOML config) into
/// [`guardrail_classifiers::PiiEntityType`] values.
///
/// This is a thin newtype wrapper used by `guardrail-config::loader` to avoid
/// a direct dependency cycle between schema parsing and classifier construction.
pub struct PiiEntityList(pub Vec<guardrail_classifiers::PiiEntityType>);

impl PiiEntityList {
    /// Convert string entity names into typed [`guardrail_classifiers::PiiEntityType`] values.
    ///
    /// # Errors
    ///
    /// Returns an error string if any name is not a recognized entity type.
    /// Configuration validation ([`crate::validate::validate_config`]) should
    /// already have caught this; this function exists as a defense-in-depth
    /// check at pipeline-construction time.
    pub fn from_strings(names: &[String]) -> Result<Self, String> {
        use guardrail_classifiers::PiiEntityType;

        let mut out = Vec::with_capacity(names.len());
        for name in names {
            let entity = match name.as_str() {
                "email" => PiiEntityType::Email,
                "phone" => PiiEntityType::Phone,
                "credit_card" => PiiEntityType::CreditCard,
                "ssn" => PiiEntityType::Ssn,
                "ip_address" => PiiEntityType::IpAddress,
                "api_key" => PiiEntityType::ApiKey,
                "aws_key" => PiiEntityType::AwsKey,
                other => return Err(format!("unknown PII entity type '{other}'")),
            };
            out.push(entity);
        }
        Ok(PiiEntityList(out))
    }
}

/// Configuration for ONNX-backed stages (`onnx_injection`, `toxicity`).
#[derive(Debug, Clone, Deserialize)]
pub struct OnnxStageConfig {
    /// Whether this stage is enabled. Default: `false` (requires model files).
    #[serde(default)]
    pub enabled: bool,
    /// Path to the `.onnx` model file.
    #[serde(default)]
    pub model_path: Option<String>,
    /// Path to the HuggingFace tokenizer file/directory.
    #[serde(default)]
    pub tokenizer_path: Option<String>,
    /// Decision threshold in `[0.0, 1.0]`.
    #[serde(default = "default_threshold")]
    pub threshold: f32,
}

impl Default for OnnxStageConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            model_path: None,
            tokenizer_path: None,
            threshold: default_threshold(),
        }
    }
}

fn default_threshold() -> f32 {
    0.85
}

fn default_true() -> bool {
    true
}

/// User-defined policy configuration.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PolicyConfig {
    /// List of policy rules, evaluated in order.
    #[serde(default)]
    pub rules: Vec<PolicyRuleConfig>,
}

/// A single policy rule, as deserialized from TOML.
#[derive(Debug, Clone, Deserialize)]
pub struct PolicyRuleConfig {
    /// Human-readable rule name.
    pub name: String,
    /// Whether the rule is active. Default: `true`.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// The condition under which this rule fires.
    pub condition: PolicyConditionConfig,
    /// The action to take when the condition matches.
    pub action: PolicyActionConfig,
    /// Optional custom message returned in the block response.
    #[serde(default)]
    pub message: Option<String>,
}

/// Policy condition, as deserialized from TOML using `type` as the tag.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PolicyConditionConfig {
    /// Matches if any keyword appears in the request content.
    ContentContains {
        /// List of keywords to match (case-insensitive).
        keywords: Vec<String>,
    },
    /// Matches if the request has no system prompt.
    SystemPromptAbsent,
    /// Matches if the approximate token count exceeds the given limit.
    TokenCountExceeds {
        /// The maximum allowed approximate token count.
        limit: usize,
    },
    /// Always matches.
    Always,
}

/// Policy action, as deserialized from TOML.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyActionConfig {
    /// Allow the request.
    Allow,
    /// Redact the request.
    Redact,
    /// Block the request.
    Block,
    /// Log only.
    LogOnly,
}

/// Observability (metrics, tracing, audit log) configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct ObservabilityConfig {
    /// Address for the Prometheus `/metrics` endpoint. Default: `"0.0.0.0:9090"`.
    #[serde(default = "default_metrics_addr")]
    pub metrics_addr: String,
    /// Log level filter, e.g. `"info"`, `"debug"`. Default: `"info"`.
    #[serde(default = "default_log_level")]
    pub log_level: String,
    /// Whether to emit logs as JSON. Default: `false`.
    #[serde(default)]
    pub json_logs: bool,
    /// Rotating NDJSON audit log file settings.
    #[serde(default)]
    pub audit_log: AuditLogConfig,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            metrics_addr: default_metrics_addr(),
            log_level: default_log_level(),
            json_logs: false,
            audit_log: AuditLogConfig::default(),
        }
    }
}

/// Rotating NDJSON audit log file configuration.
///
/// When enabled, every pipeline decision (`allow`, `redact`, `block`) is
/// additionally written as a newline-delimited JSON record to a rotating
/// file under `directory`, independent of `log_level`/`json_logs` (which
/// control general application logs). See [`guardrail_proxy::audit_log`] for
/// the implementation.
///
/// [`guardrail_proxy::audit_log`]: https://docs.rs/guardrail-proxy/latest/guardrail_proxy/audit_log/index.html
#[derive(Debug, Clone, Deserialize)]
pub struct AuditLogConfig {
    /// Whether the NDJSON audit log file is enabled. Default: `false`.
    #[serde(default)]
    pub enabled: bool,
    /// Directory to write audit log files into. Default: `"./audit-logs"`.
    #[serde(default = "default_audit_log_directory")]
    pub directory: String,
    /// File name prefix; rotation suffixes (e.g. `.2026-06-13`) are appended
    /// by the rolling file appender. Default: `"audit"`.
    #[serde(default = "default_audit_log_prefix")]
    pub file_name_prefix: String,
    /// Rotation interval: `"minutely"`, `"hourly"`, `"daily"`, or `"never"`.
    /// Default: `"daily"`.
    #[serde(default = "default_audit_log_rotation")]
    pub rotation: String,
}

impl Default for AuditLogConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            directory: default_audit_log_directory(),
            file_name_prefix: default_audit_log_prefix(),
            rotation: default_audit_log_rotation(),
        }
    }
}

fn default_audit_log_directory() -> String {
    "./audit-logs".to_string()
}

fn default_audit_log_prefix() -> String {
    "audit".to_string()
}

fn default_audit_log_rotation() -> String {
    "daily".to_string()
}

fn default_metrics_addr() -> String {
    "0.0.0.0:9090".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}
