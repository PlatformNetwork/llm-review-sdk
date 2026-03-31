//! HTTP types for the review server.
//!
//! Request and response types for the inference API.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Import types from other modules
use crate::rules::RuleViolation;
use crate::agents::AgentConfig;

/// Violation type alias for convenience.
pub type Violation = RuleViolation;

/// Configuration for a rule in a review request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleConfig {
    /// Rule identifier
    pub rule_id: String,
    /// Rule severity level
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
    /// Rule-specific configuration options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<serde_json::Value>,
}

/// Request to perform code review inference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRequest {
    /// Name of the agent to use for review
    pub agent_name: String,
    /// Source code to analyze
    pub input_code: String,
    /// Optional code to compare against (for similarity detection)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comparison_code: Option<String>,
    /// Rules to apply during review
    pub rules: Vec<RuleConfig>,
    /// Agent configuration
    pub config: AgentConfig,
    /// Unique identifier for this request
    pub request_id: String,
}

/// Response from a code review inference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResponse {
    /// Unique identifier matching the request
    pub request_id: String,
    /// List of violations found
    pub violations: Vec<Violation>,
    /// Summary of the review
    pub summary: String,
    /// Overall confidence score (0.0 to 1.0)
    pub confidence: f64,
    /// Processing duration in milliseconds
    pub duration_ms: u64,
    /// Additional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Health check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Server status
    pub status: String,
    /// Server version
    pub version: String,
    /// List of available review agents
    pub agents_available: Vec<String>,
    /// Server uptime in seconds
    pub uptime_seconds: u64,
}

/// Error response for API errors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error code
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Additional error details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl ErrorResponse {
    /// Creates a new error response.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    /// Adds details to the error response.
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inference_request_serialization() {
        let request = InferenceRequest {
            agent_name: "test-agent".to_string(),
            input_code: "fn main() {}".to_string(),
            comparison_code: None,
            rules: vec![],
            config: AgentConfig::new("test", "Test Agent", "A test agent"),
            request_id: uuid::Uuid::new_v4().to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("test-agent"));
        assert!(json.contains("fn main()"));
    }

    #[test]
    fn inference_response_serialization() {
        let response = InferenceResponse {
            request_id: "test-123".to_string(),
            violations: vec![],
            summary: "No issues found".to_string(),
            confidence: 0.95,
            duration_ms: 150,
            metadata: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test-123"));
        assert!(json.contains("No issues found"));
        assert!(json.contains("0.95"));
    }

    #[test]
    fn error_response_creation() {
        let error = ErrorResponse::new("ERR001", "Test error")
            .with_details("Additional details");

        assert_eq!(error.code, "ERR001");
        assert_eq!(error.message, "Test error");
        assert!(error.details.is_some());
    }

    #[test]
    fn health_response_serialization() {
        let health = HealthResponse {
            status: "healthy".to_string(),
            version: "0.1.0".to_string(),
            agents_available: vec!["agent1".to_string(), "agent2".to_string()],
            uptime_seconds: 3600,
        };

        let json = serde_json::to_string(&health).unwrap();
        assert!(json.contains("healthy"));
        assert!(json.contains("0.1.0"));
        assert!(json.contains("agent1"));
    }
}
