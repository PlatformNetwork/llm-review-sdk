//! LLM-powered review agents.
//!
//! This module provides the agent system for LLM-based code review,
//! including function calling capabilities and tool integration.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use thiserror::Error;

/// Severity levels for agent-detected violations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Informational suggestions
    Info = 1,
    /// Warnings that should be addressed
    Warning = 2,
    /// Errors that must be fixed
    Error = 3,
}

impl Default for Severity {
    fn default() -> Self {
        Severity::Warning
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Info => write!(f, "info"),
            Severity::Warning => write!(f, "warning"),
            Severity::Error => write!(f, "error"),
        }
    }
}

/// Errors that can occur during agent execution.
#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub enum AgentError {
    /// Agent execution failed
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    /// Tool execution error
    #[error("Tool error: {0}")]
    ToolError(String),
    /// Operation timed out
    #[error("Timeout: {0}")]
    Timeout(String),
    /// Invalid input provided
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    /// Agent not found
    #[error("Agent not found: {0}")]
    NotFound(String),
}

/// A tool that can be invoked by an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// Unique identifier for the tool
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description of what the tool does
    pub description: String,
    /// JSON Schema for tool parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters_schema: Option<serde_json::Value>,
}

impl Tool {
    /// Create a new tool with the given ID and name.
    pub fn new(id: impl Into<String>, name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            parameters_schema: None,
        }
    }

    /// Set the parameters schema.
    pub fn with_schema(mut self, schema: serde_json::Value) -> Self {
        self.parameters_schema = Some(schema);
        self
    }
}

/// A violation detected by an agent during review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    /// Rule or check that produced this violation
    pub rule_id: String,
    /// Severity level
    pub severity: Severity,
    /// Human-readable message
    pub message: String,
    /// File location (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Line number (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// Column number (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
    /// Suggested fix (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

impl Violation {
    /// Create a new violation.
    pub fn new(rule_id: impl Into<String>, severity: Severity, message: impl Into<String>) -> Self {
        Self {
            rule_id: rule_id.into(),
            severity,
            message: message.into(),
            file: None,
            line: None,
            column: None,
            suggestion: None,
        }
    }

    /// Set the file location.
    pub fn with_file(mut self, file: impl Into<String>) -> Self {
        self.file = Some(file.into());
        self
    }

    /// Set the line number.
    pub fn with_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    /// Set the column number.
    pub fn with_column(mut self, column: usize) -> Self {
        self.column = Some(column);
        self
    }

    /// Set a suggested fix.
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

/// Input provided to an agent for review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInput {
    /// Code to review
    pub code: String,
    /// File path (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    /// Language of the code (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Additional context (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<HashMap<String, serde_json::Value>>,
}

impl AgentInput {
    /// Create new agent input with the given code.
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            file_path: None,
            language: None,
            context: None,
        }
    }

    /// Set the file path.
    pub fn with_file(mut self, path: impl Into<String>) -> Self {
        self.file_path = Some(path.into());
        self
    }

    /// Set the language.
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }
}

/// Output from an agent after review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    /// Violations detected
    pub violations: Vec<Violation>,
    /// Summary of the review
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Additional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

impl Default for AgentOutput {
    fn default() -> Self {
        Self {
            violations: Vec::new(),
            summary: None,
            metadata: None,
        }
    }
}

impl AgentOutput {
    /// Create a new empty output.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create output with violations.
    pub fn with_violations(violations: Vec<Violation>) -> Self {
        Self {
            violations,
            ..Self::default()
        }
    }

    /// Add a summary.
    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }
}

/// Configuration for review agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Unique identifier for the agent
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description of the agent's purpose
    pub description: String,
    /// Tools available to this agent
    #[serde(default)]
    pub tools: Vec<Tool>,
    /// Maximum iterations for the agent
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
    /// Timeout in milliseconds
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    /// Additional configuration options
    #[serde(default)]
    pub options: HashMap<String, serde_json::Value>,
}

fn default_max_iterations() -> usize {
    10
}

fn default_timeout_ms() -> u64 {
    30000
}

impl AgentConfig {
    /// Create a new agent configuration.
    pub fn new(id: impl Into<String>, name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            tools: Vec::new(),
            max_iterations: default_max_iterations(),
            timeout_ms: default_timeout_ms(),
            options: HashMap::new(),
        }
    }

    /// Add a tool to the agent.
    pub fn with_tool(mut self, tool: Tool) -> Self {
        self.tools.push(tool);
        self
    }

    /// Set the maximum iterations.
    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    /// Set the timeout.
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}

/// Trait for LLM-powered review agents.
///
/// Agents use LLM inference to review code and detect violations,
/// optionally using tools for enhanced capabilities.
#[async_trait]
pub trait ReviewAgent: Send + Sync {
    /// Get the agent's configuration.
    fn config(&self) -> &AgentConfig;

    /// Execute the agent with the given input.
    async fn execute(&self, input: AgentInput) -> Result<AgentOutput, AgentError>;

    /// Get available tools.
    fn tools(&self) -> &[Tool] {
        &self.config().tools
    }

    /// Returns a boxed clone of this agent.
    fn boxed_clone(&self) -> Box<dyn ReviewAgent>;
}

/// Registry for discovering and managing review agents.
#[derive(Default)]
pub struct AgentRegistry {
    agents: HashMap<String, Box<dyn ReviewAgent>>,
}

impl AgentRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an agent with the registry.
    pub fn register<A: ReviewAgent + 'static>(&mut self, agent: A) -> Option<Box<dyn ReviewAgent>> {
        let id = agent.config().id.clone();
        self.agents.insert(id, Box::new(agent))
    }

    /// Register a boxed agent directly.
    pub fn register_boxed(&mut self, agent: Box<dyn ReviewAgent>) -> Option<Box<dyn ReviewAgent>> {
        let id = agent.config().id.clone();
        self.agents.insert(id, agent)
    }

    /// Get an agent by ID.
    pub fn get(&self, id: &str) -> Option<&dyn ReviewAgent> {
        self.agents.get(id).map(|a| a.as_ref())
    }

    /// Check if an agent is registered.
    pub fn contains(&self, id: &str) -> bool {
        self.agents.contains_key(id)
    }

    /// Get the number of registered agents.
    pub fn len(&self) -> usize {
        self.agents.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }

    /// Iterate over all registered agents.
    pub fn iter(&self) -> impl Iterator<Item = &dyn ReviewAgent> {
        self.agents.values().map(|a| a.as_ref())
    }

    /// Get all registered agent IDs.
    pub fn agent_ids(&self) -> impl Iterator<Item = &String> {
        self.agents.keys()
    }

    /// Alias for agent_ids() for backwards compatibility.
    pub fn agent_names(&self) -> impl Iterator<Item = &String> {
        self.agent_ids()
    }

    /// Remove an agent by ID.
    pub fn remove(&mut self, id: &str) -> Option<Box<dyn ReviewAgent>> {
        self.agents.remove(id)
    }

    /// Clear all agents from the registry.
    pub fn clear(&mut self) {
        self.agents.clear();
    }
}

impl Clone for AgentRegistry {
    fn clone(&self) -> Self {
        let agents = self
            .agents
            .iter()
            .map(|(k, v)| (k.clone(), v.boxed_clone()))
            .collect();
        Self { agents }
    }
}
