//! LLM inference abstraction for multi-provider support.
//!
//! This module provides traits and types for abstracting LLM inference
//! across multiple providers (Ollama, OpenAI, Anthropic).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Supported LLM providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    /// Local Ollama inference
    Ollama,
    /// OpenAI API
    OpenAI,
    /// Anthropic Claude API
    Anthropic,
}

impl Default for Provider {
    fn default() -> Self {
        Provider::Ollama
    }
}

/// Configuration for LLM clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// The LLM provider to use
    pub provider: Provider,
    /// Base URL for the API (e.g., "http://localhost:11434" for Ollama)
    #[serde(default = "default_base_url")]
    pub base_url: String,
    /// API key for authentication (optional for local providers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Model identifier to use
    pub model: String,
    /// Maximum tokens to generate
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    /// Temperature for sampling (0.0 - 2.0)
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// Additional provider-specific options
    #[serde(default)]
    pub options: HashMap<String, serde_json::Value>,
}

fn default_base_url() -> String {
    "http://localhost:11434".to_string()
}

fn default_max_tokens() -> usize {
    4096
}

fn default_temperature() -> f32 {
    0.7
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: Provider::default(),
            base_url: default_base_url(),
            api_key: None,
            model: "llama2".to_string(),
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            options: HashMap::new(),
        }
    }
}

/// Errors that can occur during LLM inference.
#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub enum LlmError {
    /// Connection failed
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    /// Request timed out
    #[error("Request timed out after {0}ms")]
    Timeout(u64),
    /// Invalid request
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    /// Rate limited
    #[error("Rate limited: {0}")]
    RateLimited(String),
    /// API error from provider
    #[error("API error: {0}")]
    ApiError(String),
    /// Model not found
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    /// Authentication failed
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    /// Response parsing error
    #[error("Failed to parse response: {0}")]
    ParseError(String),
}

/// A single message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role of the message sender
    pub role: String,
    /// Content of the message
    pub content: String,
}

/// Request to generate a completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// Conversation messages
    pub messages: Vec<Message>,
    /// Maximum tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,
    /// Temperature for sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

/// Response from a completion request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// Generated content
    pub content: String,
    /// Token usage information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
}

/// Token usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Input tokens used
    pub prompt_tokens: usize,
    /// Output tokens generated
    pub completion_tokens: usize,
    /// Total tokens
    pub total_tokens: usize,
}

/// Trait for LLM inference clients.
///
/// This trait abstracts LLM inference across multiple providers,
/// allowing the same code to work with Ollama, OpenAI, and Anthropic.
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Generate a completion for the given request.
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError>;

    /// Get the current configuration.
    fn config(&self) -> &LlmConfig;

    /// Get the provider type.
    fn provider(&self) -> Provider {
        self.config().provider
    }

    /// Check if the client is healthy and can connect.
    async fn health_check(&self) -> Result<(), LlmError>;

    /// Returns a boxed clone of this client.
    fn boxed_clone(&self) -> Box<dyn LlmClient>;
}

pub mod providers;
