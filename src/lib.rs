//! # LLM Review SDK
//!
//! SDK for LLM-powered code review in Platform Network challenges.
//!
//! ## Features
//!
//! - **Rules Engine**: Configurable review rules and validation
//! - **Agents**: LLM-powered review agents with function calling
//! - **Inference**: Multi-provider LLM inference abstraction
//! - **Workflow**: Orchestration layer for review pipelines
//! - **Server**: HTTP server for review API
//!
//! ## Feature Flags
//!
//! - `std` (default): Enables full standard library support
//! - `async`: Enables async runtime support
//! - `plagiarism`: Enables plagiarism detection integration
//!
//! ## Example
//!
//! ```rust,ignore
//! use llm_review_sdk::{ReviewAgent, InferenceClient, ReviewWorkflow};
//!
//! let client = InferenceClient::new(Default::default())?;
//! let workflow = ReviewWorkflow::new(client)?;
//! let result = workflow.review(code).await?;
//! ```

#![warn(missing_docs)]

// Core modules
pub mod rules;

#[cfg(feature = "std")]
pub mod agents;

#[cfg(feature = "std")]
pub mod inference;

#[cfg(feature = "std")]
pub mod workflow;

#[cfg(feature = "std")]
pub mod server;

#[cfg(feature = "plagiarism")]
pub mod integration;

// Re-exports
pub use rules::{Rule, RuleMeta, Severity, RuleRegistry, RuleContext, RuleVisitor, RuleViolation};

#[cfg(feature = "std")]
pub use agents::{ReviewAgent, AgentInput, AgentOutput, AgentConfig, AgentError, AgentRegistry, Tool};

#[cfg(feature = "std")]
pub use inference::{LlmClient, LlmConfig, LlmError, Provider};

#[cfg(feature = "std")]
pub use workflow::{ReviewWorkflow, WorkflowError};

#[cfg(feature = "std")]
pub use server::{Server, ServerError, InferenceRequest, InferenceResponse};

/// Crate-level error type
#[cfg(feature = "std")]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Rule evaluation error
    #[error("Rule error: {0}")]
    Rule(#[from] rules::RuleError),
    
    /// Agent error
    #[error("Agent error: {0}")]
    Agent(#[from] agents::AgentError),
    
    /// Inference error
    #[error("Inference error: {0}")]
    Inference(#[from] inference::LlmError),
    
    /// Server error
    #[error("Server error: {0}")]
    Server(#[from] server::ServerError),
    
    /// Workflow error
    #[error("Workflow error: {0}")]
    Workflow(#[from] workflow::WorkflowError),
}

/// Result type for this crate
#[cfg(feature = "std")]
pub type Result<T> = std::result::Result<T, Error>;
