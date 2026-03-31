//! HTTP server for LLM code review API.
//!
//! This module provides the server infrastructure for exposing
//! the review functionality via HTTP.

pub mod types;

// Re-export types for convenience
pub use types::{
    ErrorResponse, HealthResponse, InferenceRequest, InferenceResponse,
    RuleConfig, Violation, ProjectInput, FileMatch, CodeMatch,
};

use std::net::SocketAddr;
use std::time::Instant;

/// The review server.
pub struct Server {
    /// Server address
    addr: SocketAddr,
    /// Server start time for uptime tracking
    start_time: Instant,
}

impl Server {
    /// Creates a new server instance.
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            start_time: Instant::now(),
        }
    }

    /// Creates a server bound to all interfaces on the given port.
    pub fn bind(port: u16) -> Self {
        let addr: SocketAddr = ([0, 0, 0, 0], port).into();
        Self::new(addr)
    }

    /// Returns the server address.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Returns the server uptime in seconds.
    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Starts the server (placeholder - actual implementation would use tokio/axum).
    pub async fn serve(self) -> Result<(), ServerError> {
        // Placeholder: actual HTTP server implementation would go here
        // This is intentionally not implemented per requirements
        Ok(())
    }
}

/// Error type for server operations.
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    /// Binding error
    #[error("Failed to bind to address: {0}")]
    BindError(String),
    
    /// HTTP error
    #[error("HTTP error: {0}")]
    HttpError(String),
    
    /// Request parsing error
    #[error("Failed to parse request: {0}")]
    ParseError(String),
    
    /// Internal error
    #[error("Internal server error: {0}")]
    Internal(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_creation() {
        let server = Server::bind(8080);
        let addr = server.addr();
        assert_eq!(addr.port(), 8080);
    }

    #[test]
    fn uptime_tracking() {
        let server = Server::bind(8080);
        // Uptime should be very small immediately after creation
        let uptime = server.uptime_seconds();
        assert!(uptime < 1);
    }
}
