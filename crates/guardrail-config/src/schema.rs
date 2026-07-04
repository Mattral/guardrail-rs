//! TOML configuration schema — matches §9 of the project spec exactly.
//!
//! The canonical TOML shape is:
//!
//! ```toml
//! [server]
//! host    = "127.0.0.1"
//! port    = 8080
//! workers = 0
//!
//! [upstream]
//! url             = "https://api.openai.com"
//! timeout_secs    = 120
//! connect_timeout = 10
//!
//! [auth]
//! require_key = false
//! # keys = ["grk-your-key-here"]
//!
//! [pipeline]
//! request_stages  = ["regex_injection", "onnx_injection", "pii_redactor", "toxicity", "policy"]
//! response_stages = ["output_pii_redactor"]
//!
//! [stages.regex_injection]
//! enabled     = true
//! rules_file  = ""
//! extra_rules = []
//! action      = "block"
//!
//! [stages.pii_redactor]
//! enabled  = true
//! entities = ["email", "phone", "credit_card", "ssn", "ip_address", "api_key", "aws_key"]
//! action   = "redact"
//!
//! [stages.pii_redactor.replacements]
//! email       = "[EMAIL]"
//! phone       = "[PHONE]"
//! credit_card = "[CARD]"
//! ssn         = "[SSN]"
//! ip_address  = "[IP_ADDRESS]"
//! api_key     = "[API_KEY]"
//! aws_key     = "[AWS_KEY]"
//!
//! [[policy.rules]]
//! name    = "block-competitor-mentions"
//! enabled = false
//! when.content_contains = ["competitor-a"]
//! then.action  = "block"
//! then.message = "Competitor mentions are not permitted."
//!
//! [observability]
//! log_level    = "info"
//! log_format   = "json"
//! metrics_port = 9090
//! otlp_endpoint = ""
//!
//! [observability.audit_log]
//! enabled    = true
//! path       = "./guardrail-audit.ndjson"
//! max_size_mb = 100
//! ```

use serde::Deserialize;

// ── Top-level ─────────────────────────────────────────────────────────────────

/// Top-level configuration structure.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Server bind settings.
    pub server: ServerConfig,
    /// Upstream LLM connection settings.
    #[serde(default)]
    pub upstream: UpstreamConfig,
    /// Optional caller authentication (guardrail-specific API keys).
    #[serde(default)]
    pub auth: AuthConfig,
    /// Pipeline ordering and error behavior.
    #[serde(default)]
    pub pipeline: PipelineConfig,
    /// Per-stage configuration.
    #[serde(default)]
    pub stages: StagesConfig,
    /// User-defined policy rules (evaluated last).
    #[serde(default)]
    pub policy: PolicyConfig,
    /// Observability: logging, metrics, tracing, audit log.
    #[serde(default)]
    pub observability: ObservabilityConfig,
}

// ── Server ────────────────────────────────────────────────────────────────────

/// Server bind address and worker settings.
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    /// Bind address (default: `"127.0.0.1"`).
    #[serde(default = "default_host")]
    pub host: String,
    /// Bind port (default: `8080`).
    #[serde(default = "default_port")]
    pub port: u16,
    /// Number of Tokio worker threads. `0` = number of logical CPUs (default).
    #[serde(default)]
    pub workers: usize,
    /// Maximum allowed request body size in bytes (default: 10 MiB).
    #[serde(default = "default_max_body_size")]
    pub max_body_size_bytes: usize,
}

impl ServerConfig {
    /// Returns `"host:port"` as a `SocketAddr`-formatted string.
    pub fn listen_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            workers: 0,
            max_body_size_bytes: default_max_body_size(),
        }
    }
}

fn default_host() -> String { "127.0.0.1".to_string() }
fn default_port() -> u16 { 8080 }
fn default_max_body_size() -> usize { 10 * 1024 * 1024 }

// ── Upstream ──────────────────────────────────────────────────────────────────

/// Upstream LLM API connection settings.
#[derive(Debug, Clone, Deserialize)]
pub struct UpstreamConfig {
    /// Base URL of the upstream LLM API (default: `"https://api.openai.com"`).
    #[serde(default = "default_upstream_url")]
    pub url: String,
    /// Per-request timeout in seconds (default: `120`).
    #[serde(default = "default_upstream_timeout")]
    pub timeout_secs: u64,
    /// TCP connection timeout in seconds (default: `10`).
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout: u64,
}

impl Default for UpstreamConfig {
    fn default() -> Self {
        Self {
            url: default_upstream_url(),
            timeout_secs: default_upstream_timeout(),
            connect_timeout: default_connect_timeout(),
        }
    }
}

fn default_upstream_url() -> String { "https://api.openai.com".to_string() }
fn default_upstream_timeout() -> u64 { 120 }
fn default_connect_timeout() -> u64 { 10 }

// ── Auth ──────────────────────────────────────────────────────────────────────

/// Caller authentication configuration.
///
/// When `require_key = true`, every incoming request must present one of the
/// configured `keys` in the `X-Guardrail-Key` header. Requests that don't are
/// rejected with HTTP 401 before the pipeline runs.
///
/// This is separate from the upstream API key; `guardrail-rs` never inspects,
/// stores, or modifies the upstream `Authorization` header.
#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    /// Whether to require callers to present a guardrail API key.
    /// Default: `false`.
    #[serde(default)]
    pub require_key: bool,
    /// Accepted guardrail API keys.
    /// If `require_key = true` and this list is empty, all requests are rejected.
    #[serde(default)]
    pub keys: Vec<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            require_key: false,
            keys: Vec::new(),
        }
    }
}

// ── Pipeline ──────────────────────────────────────────────────────────────────

/// Pipeline ordering and error-behavior settings.
#[derive(Debug, Clone, Deserialize)]
pub struct PipelineConfig {
    /// Ordered list of stage IDs to run on the **request** side.
    /// Remove a stage ID to disable it. Order is significant.
    #[serde(default = "default_request_stages")]
    pub request_stages: Vec<String>,

    /// Ordered list of stage IDs to run on the **response** side.
    #[serde(default = "default_response_stages")]
    pub response_stages: Vec<String>,

    /// What to do when a stage returns an error.
    ///
    /// - `"allow"` (default): treat the error as `Decision::Allow` (fail-open).
    /// - `"block"`: treat the error as `Decision::Block` (fail-closed).
    #[serde(default)]
    pub on_error: OnErrorBehavior,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            request_stages: default_request_stages(),
            response_stages: default_response_stages(),
            on_error: OnErrorBehavior::Allow,
        }
    }
}

fn default_request_stages() -> Vec<String> {
    vec![
        "regex_injection".into(),
        "onnx_injection".into(),
        "pii_redactor".into(),
        "toxicity".into(),
        "policy".into(),
    ]
}

fn default_response_stages() -> Vec<String> {
    vec!["output_pii_redactor".into()]
}

/// Behavior when a pipeline stage returns an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OnErrorBehavior {
    /// Fail open: treat stage errors as `Allow`.
    #[default]
    Allow,
    /// Fail closed: treat stage errors as `Block`.
    Block,
}

// ── Stages ────────────────────────────────────────────────────────────────────

/// Per-stage configuration block.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct StagesConfig {
    /// Regex-based prompt injection scanner.
    #[serde(default)]
    pub regex_injection: RegexInjectionConfig,
    /// ONNX-based semantic injection classifier (`onnx` feature).
    #[serde(default)]
    pub onnx_injection: OnnxStageConfig,
    /// PII detection and redaction.
    #[serde(default, alias = "pii_redaction")]
    pub pii_redactor: PiiRedactorConfig,
    /// ONNX-based toxicity classifier (`onnx` feature).
    #[serde(default)]
    pub toxicity: ToxicityStageConfig,
}

/// Per-stage action when the stage fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StageAction {
    /// Block the request entirely (default for injection/toxicity stages).
    #[default]
    Block,
    /// Redact the sensitive content and continue (default for PII stage).
    Redact,
    /// Log the match but allow the request through (dry-run mode).
    LogOnly,
}

/// Configuration for [`guardrail_classifiers::RegexInjectionScanner`].
#[derive(Debug, Clone, Deserialize)]
pub struct RegexInjectionConfig {
    /// Whether this stage is enabled. Default: `true`.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Path to a custom rule file. Empty string = use bundled rules.
    #[serde(default)]
    pub rules_file: String,
    /// Additional regex patterns appended to the active rule set.
    #[serde(default)]
    pub extra_rules: Vec<String>,
    /// Action on match. Default: `"block"`.
    #[serde(default)]
    pub action: StageAction,
}

impl Default for RegexInjectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rules_file: String::new(),
            extra_rules: Vec::new(),
            action: StageAction::Block,
        }
    }
}

/// Custom PII replacement tokens.
#[derive(Debug, Clone, Deserialize)]
pub struct PiiReplacements {
    /// Replacement for email addresses. Default: `"[EMAIL]"`.
    #[serde(default = "default_email_replacement")]
    pub email: String,
    /// Replacement for phone numbers. Default: `"[PHONE]"`.
    #[serde(default = "default_phone_replacement")]
    pub phone: String,
    /// Replacement for credit card numbers. Default: `"[CARD]"`.
    #[serde(default = "default_card_replacement")]
    pub credit_card: String,
    /// Replacement for SSNs. Default: `"[SSN]"`.
    #[serde(default = "default_ssn_replacement")]
    pub ssn: String,
    /// Replacement for IP addresses. Default: `"[IP_ADDRESS]"`.
    #[serde(default = "default_ip_replacement")]
    pub ip_address: String,
    /// Replacement for API keys. Default: `"[API_KEY]"`.
    #[serde(default = "default_apikey_replacement")]
    pub api_key: String,
    /// Replacement for AWS keys. Default: `"[AWS_KEY]"`.
    #[serde(default = "default_awskey_replacement")]
    pub aws_key: String,
}

impl Default for PiiReplacements {
    fn default() -> Self {
        Self {
            email: default_email_replacement(),
            phone: default_phone_replacement(),
            credit_card: default_card_replacement(),
            ssn: default_ssn_replacement(),
            ip_address: default_ip_replacement(),
            api_key: default_apikey_replacement(),
            aws_key: default_awskey_replacement(),
        }
    }
}

fn default_email_replacement() -> String { "[EMAIL]".into() }
fn default_phone_replacement() -> String { "[PHONE]".into() }
fn default_card_replacement() -> String { "[CARD]".into() }
fn default_ssn_replacement() -> String { "[SSN]".into() }
fn default_ip_replacement() -> String { "[IP_ADDRESS]".into() }
fn default_apikey_replacement() -> String { "[API_KEY]".into() }
fn default_awskey_replacement() -> String { "[AWS_KEY]".into() }

/// Configuration for [`guardrail_classifiers::PiiRedactor`].
#[derive(Debug, Clone, Deserialize)]
pub struct PiiRedactorConfig {
    /// Whether this stage is enabled. Default: `true`.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Which entity types to detect and redact.
    #[serde(default = "default_pii_entities")]
    pub entities: Vec<String>,
    /// Action when PII is found. Default: `"redact"`.
    #[serde(default = "default_redact_action")]
    pub action: StageAction,
    /// Whether to apply Luhn validation to credit card candidates. Default: `true`.
    #[serde(default = "default_true")]
    pub validate_luhn: bool,
    /// Custom replacement tokens.
    #[serde(default)]
    pub replacements: PiiReplacements,
    /// Whether to also scan and redact PII in non-streaming responses. Default: `false`.
    #[serde(default)]
    pub redact_responses: bool,
}

impl Default for PiiRedactorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            entities: default_pii_entities(),
            action: StageAction::Redact,
            validate_luhn: true,
            replacements: PiiReplacements::default(),
            redact_responses: false,
        }
    }
}

fn default_redact_action() -> StageAction { StageAction::Redact }

fn default_pii_entities() -> Vec<String> {
    vec![
        "email".into(), "phone".into(), "credit_card".into(),
        "ssn".into(), "ip_address".into(), "api_key".into(), "aws_key".into(),
    ]
}

/// Configuration for ONNX-backed injection/toxicity stages.
#[derive(Debug, Clone, Deserialize)]
pub struct OnnxStageConfig {
    /// Whether this stage is enabled. Default: `false` (requires model files).
    #[serde(default)]
    pub enabled: bool,
    /// Path to the `.onnx` model file. Empty string = use bundled model (if available).
    #[serde(default)]
    pub model_path: String,
    /// Path to the HuggingFace `tokenizer.json`.
    #[serde(default)]
    pub tokenizer_path: String,
    /// Decision threshold in `[0.0, 1.0]`. Default: `0.85`.
    #[serde(default = "default_threshold")]
    pub threshold: f32,
    /// Action on positive classification. Default: `"block"`.
    #[serde(default)]
    pub action: StageAction,
    /// What to do if inference fails. Default: `"allow"` (fail-open).
    #[serde(default)]
    pub on_error: OnErrorBehavior,
}

impl Default for OnnxStageConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            model_path: String::new(),
            tokenizer_path: String::new(),
            threshold: default_threshold(),
            action: StageAction::Block,
            on_error: OnErrorBehavior::Allow,
        }
    }
}

/// Configuration specific to the toxicity classifier (adds `scan_roles`).
#[derive(Debug, Clone, Deserialize)]
pub struct ToxicityStageConfig {
    /// Whether this stage is enabled.
    #[serde(default)]
    pub enabled: bool,
    /// Path to the `.onnx` model file.
    #[serde(default)]
    pub model_path: String,
    /// Path to the HuggingFace tokenizer.
    #[serde(default)]
    pub tokenizer_path: String,
    /// Threshold. Default: `0.90`.
    #[serde(default = "default_toxicity_threshold")]
    pub threshold: f32,
    /// Action on classification. Default: `"block"`.
    #[serde(default)]
    pub action: StageAction,
    /// Which message roles to scan. Default: `["user"]`.
    #[serde(default = "default_scan_roles")]
    pub scan_roles: Vec<String>,
    /// On-error behavior.
    #[serde(default)]
    pub on_error: OnErrorBehavior,
}

impl Default for ToxicityStageConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            model_path: String::new(),
            tokenizer_path: String::new(),
            threshold: default_toxicity_threshold(),
            action: StageAction::Block,
            scan_roles: default_scan_roles(),
            on_error: OnErrorBehavior::Allow,
        }
    }
}

fn default_threshold() -> f32 { 0.85 }
fn default_toxicity_threshold() -> f32 { 0.90 }
fn default_scan_roles() -> Vec<String> { vec!["user".into()] }
fn default_true() -> bool { true }

// ── Policy ────────────────────────────────────────────────────────────────────

/// User-defined policy configuration.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PolicyConfig {
    /// List of policy rules, evaluated in order.
    #[serde(default)]
    pub rules: Vec<PolicyRuleConfig>,
}

/// A single policy rule using the `when`/`then` TOML shape from the spec.
///
/// # Example
///
/// ```toml
/// [[policy.rules]]
/// name    = "block-competitor-mentions"
/// enabled = false
/// when.content_contains = ["competitor-a", "competitor-b"]
/// then.action  = "block"
/// then.message = "Competitor mentions are not permitted."
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct PolicyRuleConfig {
    /// Human-readable rule name (used in audit logs).
    pub name: String,
    /// Whether the rule is active. Default: `true`.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// The condition that triggers this rule.
    pub when: PolicyConditionConfig,
    /// The action to take when the condition matches.
    pub then: PolicyActionConfig,
}

/// Policy condition (flattened `when.*` table).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct PolicyConditionConfig {
    /// Matches if any keyword appears in any message content (case-insensitive).
    #[serde(default)]
    pub content_contains: Vec<String>,
    /// Matches if no system prompt is present.
    #[serde(default)]
    pub system_prompt_absent: bool,
    /// Matches if the approximate token count exceeds this value. `0` = disabled.
    #[serde(default)]
    pub token_count_exceeds: usize,
    /// Always matches. Takes precedence over other conditions if `true`.
    #[serde(default)]
    pub always: bool,
}

/// Policy action (the `then.*` table).
#[derive(Debug, Clone, Deserialize)]
pub struct PolicyActionConfig {
    /// The action to take: `"allow"`, `"redact"`, `"block"`, or `"log_only"`.
    #[serde(default)]
    pub action: PolicyAction,
    /// Custom message returned to the blocked caller.
    #[serde(default)]
    pub message: Option<String>,
}

impl Default for PolicyActionConfig {
    fn default() -> Self {
        Self { action: PolicyAction::Block, message: None }
    }
}

/// Action taken when a policy condition matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PolicyAction {
    /// Allow the request through.
    Allow,
    /// Redact (reserved; treated as Allow by the current PolicyEngine).
    Redact,
    /// Block the request.
    #[default]
    Block,
    /// Log the match but allow through.
    LogOnly,
}

// ── Observability ─────────────────────────────────────────────────────────────

/// Observability (metrics, tracing, audit log) configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct ObservabilityConfig {
    /// Log level: `"trace"`, `"debug"`, `"info"`, `"warn"`, `"error"`. Default: `"info"`.
    #[serde(default = "default_log_level")]
    pub log_level: String,
    /// Log format: `"json"` or `"pretty"`. Default: `"pretty"`.
    #[serde(default = "default_log_format")]
    pub log_format: String,
    /// Prometheus scrape port. `0` disables the separate metrics port. Default: `9090`.
    #[serde(default = "default_metrics_port")]
    pub metrics_port: u16,
    /// OpenTelemetry OTLP gRPC endpoint. Empty = disabled. Default: `""`.
    #[serde(default)]
    pub otlp_endpoint: String,
    /// Structured audit log settings.
    #[serde(default)]
    pub audit_log: AuditLogConfig,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            log_level: default_log_level(),
            log_format: default_log_format(),
            metrics_port: default_metrics_port(),
            otlp_endpoint: String::new(),
            audit_log: AuditLogConfig::default(),
        }
    }
}

fn default_log_level() -> String { "info".to_string() }
fn default_log_format() -> String { "pretty".to_string() }
fn default_metrics_port() -> u16 { 9090 }

/// Rotating NDJSON audit log configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct AuditLogConfig {
    /// Whether the audit log is enabled. Default: `false`.
    #[serde(default)]
    pub enabled: bool,
    /// Path to the audit log file. Default: `"./guardrail-audit.ndjson"`.
    #[serde(default = "default_audit_path")]
    pub path: String,
    /// Rotate when the file exceeds this size in MB. Default: `100`.
    #[serde(default = "default_max_size_mb")]
    pub max_size_mb: u64,
}

impl Default for AuditLogConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            path: default_audit_path(),
            max_size_mb: default_max_size_mb(),
        }
    }
}

fn default_audit_path() -> String { "./guardrail-audit.ndjson".to_string() }
fn default_max_size_mb() -> u64 { 100 }

// ── PiiEntityList helper ──────────────────────────────────────────────────────

/// Converts a list of entity-type strings into
/// [`guardrail_classifiers::PiiEntityType`] values.
pub struct PiiEntityList(pub Vec<guardrail_classifiers::PiiEntityType>);

impl PiiEntityList {
    /// Convert string entity names into typed values.
    ///
    /// # Errors
    ///
    /// Returns a string error if any name is not a recognized entity type.
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
