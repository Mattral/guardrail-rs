//! Shared server state and lifecycle handle.
//!
//! Kept separate from request-handling logic ([`crate::handler`]) and the
//! listener loop ([`crate::server`]) so each module has a single, obvious
//! responsibility: this one just defines *what data* every connection
//! handler needs, with no behavior of its own.

use std::net::SocketAddr;
use std::sync::Arc;

use guardrail_config::ConfigHandle;

use crate::metrics::Metrics;

/// Shared application state, cloned (cheaply, via `Arc`) into every
/// connection handler.
pub(crate) struct AppState {
    pub(crate) config: Arc<ConfigHandle>,
    pub(crate) http_client: reqwest::Client,
    pub(crate) metrics: Metrics,
}

/// A handle to a running server, returned by [`crate::run_server`].
///
/// Dropping this handle does **not** stop the server; call
/// [`ServerHandle::shutdown`] to request graceful shutdown.
pub struct ServerHandle {
    pub(crate) addr: SocketAddr,
    pub(crate) shutdown_tx: tokio::sync::watch::Sender<bool>,
}

impl ServerHandle {
    /// The address the server is bound to. Useful when `listen_addr` uses
    /// port `0` and the OS assigns a port.
    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    /// Request graceful shutdown. In-flight requests are allowed to complete.
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
    }
}
