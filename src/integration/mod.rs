//! Integration with plagiarism-sdk
//!
//! This module provides types and functions for integrating
//! with the plagiarism detection SDK.

use serde::{Deserialize, Serialize};

#[cfg(feature = "plagiarism")]
use plagiarism_sdk as plagiarism;

/// Result of plagiarism integration check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlagiarismCheckResult {
    /// Whether plagiarism was detected
    pub is_plagiarism: bool,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
    /// Similarity score (0 to 100)
    pub similarity_score: u8,
    /// Human-readable explanation
    pub reasoning: String,
}

/// Error type for integration operations
#[derive(Debug, thiserror::Error)]
pub enum IntegrationError {
    /// Plagiarism check failed
    #[error("Plagiarism check failed: {0}")]
    PlagiarismCheckFailed(String),

    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

#[cfg(feature = "plagiarism")]
pub use plagiarism::*;
