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
        Some(key) => matches_any_key(&config.keys, key),
        None => false,
    }
}

/// Compare `presented` against every configured key without leaking, through
/// timing, how many leading bytes matched or which key matched.
///
/// Each comparison is constant-time in the key contents (`subtle`), and every
/// key is checked even after a match. Key *lengths* can still be observed, as
/// with any comparison that doesn't hash first; keys are meant to be long
/// random strings, so their length isn't the secret part.
fn matches_any_key(keys: &[String], presented: &str) -> bool {
    use subtle::{Choice, ConstantTimeEq};

    let presented = presented.as_bytes();
    let matched = keys.iter().fold(Choice::from(0), |acc, k| {
        acc | k.as_bytes().ct_eq(presented)
    });
    matched.into()
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
    fn test_matches_any_key() {
        let keys = vec!["key-a".to_string(), "key-b".to_string()];
        assert!(matches_any_key(&keys, "key-a"));
        assert!(matches_any_key(&keys, "key-b"));
        assert!(!matches_any_key(&keys, "key-c"));
        // Prefixes, extensions and empty strings never match.
        assert!(!matches_any_key(&keys, "key-"));
        assert!(!matches_any_key(&keys, "key-a2"));
        assert!(!matches_any_key(&keys, ""));
        assert!(!matches_any_key(&[], "key-a"));
    }

    #[test]
    fn test_key_comparison_does_not_use_eq() {
        // Guards against reintroducing a short-circuiting `==` on the secret.
        let src = include_str!("auth.rs").replace("\r\n", "\n");
        let body = src
            .split("fn matches_any_key")
            .nth(1)
            .and_then(|rest| rest.split("\n}\n").next())
            .expect("matches_any_key body");
        assert!(body.contains("ct_eq"), "key comparison must use ct_eq");
        assert!(!body.contains("=="), "key comparison must not use ==");
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
