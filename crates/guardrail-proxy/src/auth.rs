//! Caller authentication (`[auth]`).
//!
//! Optional shared-secret check, separate from the upstream provider's
//! `Authorization` header (which is always forwarded opaquely and never
//! inspected here). See `docs/threat-model.md` for the security properties
//! and residual risk of this mechanism.

use guardrail_config::schema::AuthConfig;
use hyper::HeaderMap;

/// Endpoints that never require a caller key, regardless of `[auth]`
/// configuration — monitoring systems need unauthenticated access to these.
pub(crate) const AUTH_EXEMPT_PATHS: &[&str] = &["/healthz", "/metrics"];

/// The header callers must present their guardrail API key in.
pub(crate) const AUTH_HEADER_NAME: &str = "x-guardrail-key";

/// Check whether a request is authorized to proceed.
///
/// Returns `true` if the request should be allowed past the auth gate:
/// either `auth.require_key` is `false`, the path is exempt, or the
/// presented key matches one of `auth.keys`.
///
/// Extracted as a pure function (no I/O, no state beyond what's passed in)
/// specifically so it's unit-testable without spinning up a real TCP
/// listener — the [`crate::server`] integration tests cover the end-to-end
/// HTTP behavior, and the tests in this module cover every branch of the
/// authorization decision itself in isolation.
///
/// # Examples
///
/// ```rust
/// use guardrail_config::schema::AuthConfig;
/// use guardrail_proxy::auth::is_authorized;
/// use hyper::HeaderMap;
///
/// let config = AuthConfig { require_key: true, keys: vec!["secret".into()] };
///
/// let mut headers = HeaderMap::new();
/// headers.insert("x-guardrail-key", "secret".parse().unwrap());
/// assert!(is_authorized(&config, "/v1/chat/completions", &headers));
///
/// let empty_headers = HeaderMap::new();
/// assert!(!is_authorized(&config, "/v1/chat/completions", &empty_headers));
///
/// // Health checks are always exempt, even with no key presented.
/// assert!(is_authorized(&config, "/healthz", &empty_headers));
/// ```
pub fn is_authorized(config: &AuthConfig, path: &str, headers: &HeaderMap) -> bool {
    if !config.require_key {
        return true;
    }
    if AUTH_EXEMPT_PATHS.contains(&path) {
        return true;
    }

    let presented_key = headers.get(AUTH_HEADER_NAME).and_then(|v| v.to_str().ok());

    match presented_key {
        Some(key) => config.keys.iter().any(|k| k == key),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers_with_key(key: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(AUTH_HEADER_NAME, key.parse().unwrap());
        h
    }

    #[test]
    fn test_disabled_auth_always_allows() {
        let config = AuthConfig {
            require_key: false,
            keys: vec![],
        };
        assert!(is_authorized(
            &config,
            "/v1/chat/completions",
            &HeaderMap::new()
        ));
    }

    #[test]
    fn test_missing_key_rejected() {
        let config = AuthConfig {
            require_key: true,
            keys: vec!["secret".into()],
        };
        assert!(!is_authorized(
            &config,
            "/v1/chat/completions",
            &HeaderMap::new()
        ));
    }

    #[test]
    fn test_wrong_key_rejected() {
        let config = AuthConfig {
            require_key: true,
            keys: vec!["secret".into()],
        };
        let headers = headers_with_key("wrong");
        assert!(!is_authorized(&config, "/v1/chat/completions", &headers));
    }

    #[test]
    fn test_correct_key_allowed() {
        let config = AuthConfig {
            require_key: true,
            keys: vec!["secret".into()],
        };
        let headers = headers_with_key("secret");
        assert!(is_authorized(&config, "/v1/chat/completions", &headers));
    }

    #[test]
    fn test_one_of_multiple_keys_allowed() {
        let config = AuthConfig {
            require_key: true,
            keys: vec!["key-a".into(), "key-b".into()],
        };
        assert!(is_authorized(
            &config,
            "/v1/chat/completions",
            &headers_with_key("key-b")
        ));
    }

    #[test]
    fn test_healthz_exempt_even_without_key() {
        let config = AuthConfig {
            require_key: true,
            keys: vec!["secret".into()],
        };
        assert!(is_authorized(&config, "/healthz", &HeaderMap::new()));
    }

    #[test]
    fn test_metrics_exempt_even_without_key() {
        let config = AuthConfig {
            require_key: true,
            keys: vec!["secret".into()],
        };
        assert!(is_authorized(&config, "/metrics", &HeaderMap::new()));
    }

    #[test]
    fn test_proxy_path_not_exempt() {
        let config = AuthConfig {
            require_key: true,
            keys: vec!["secret".into()],
        };
        assert!(!is_authorized(&config, "/v1/messages", &HeaderMap::new()));
    }

    #[test]
    fn test_empty_keys_list_rejects_everything() {
        // Misconfiguration (require_key=true, keys=[]) is caught by config
        // validation separately, but this function must still fail closed
        // rather than panic or accidentally allow.
        let config = AuthConfig {
            require_key: true,
            keys: vec![],
        };
        assert!(!is_authorized(
            &config,
            "/v1/chat/completions",
            &headers_with_key("anything")
        ));
    }
}
