//! Workflow orchestration for review pipelines.
//!
//! This module provides the orchestration layer for coordinating
//! code review operations across multiple agents and rules.

use crate::agents::AgentRegistry;
use crate::inference::LlmClient;
use crate::rules::RuleRegistry;
use std::sync::Arc;

/// Orchestrates code review workflows.
pub struct ReviewWorkflow {
    /// Agent registry for agent lookup
    agents: AgentRegistry,
    /// Rule registry for rule lookup
    rules: RuleRegistry,
    /// LLM client for inference
    llm_client: Option<Arc<dyn LlmClient>>,
}

impl ReviewWorkflow {
    /// Creates a new workflow instance.
    pub fn new(agents: AgentRegistry, rules: RuleRegistry) -> Self {
        Self {
            agents,
            rules,
            llm_client: None,
        }
    }

    /// Sets the LLM client for the workflow.
    pub fn with_llm_client(mut self, client: Arc<dyn LlmClient>) -> Self {
        self.llm_client = Some(client);
        self
    }

    /// Returns a reference to the agent registry.
    pub fn agents(&self) -> &AgentRegistry {
        &self.agents
    }

    /// Returns a reference to the rule registry.
    pub fn rules(&self) -> &RuleRegistry {
        &self.rules
    }

    /// Executes a review workflow.
    ///
    /// # Arguments
    /// * `agent_name` - Name of the agent to use
    /// * `input_code` - Code to review
    /// * `rule_ids` - IDs of rules to apply
    ///
    /// # Returns
    /// A result containing the workflow output or an error.
    pub async fn execute(
        &self,
        agent_name: &str,
        _input_code: &str,
        _rule_ids: &[&str],
    ) -> Result<WorkflowOutput, WorkflowError> {
        // Verify agent is registered
        if self.agents.get(agent_name).is_none() {
            return Err(WorkflowError::AgentNotRegistered {
                agent: agent_name.to_string(),
            });
        }

        // Placeholder: actual workflow execution would go here
        // This is intentionally minimal per requirements

        Ok(WorkflowOutput {
            agent_name: agent_name.to_string(),
            violations: vec![],
            confidence: 1.0,
        })
    }
}

/// Output from a workflow execution.
#[derive(Debug, Clone)]
pub struct WorkflowOutput {
    /// Agent that performed the review
    pub agent_name: String,
    /// Violations found
    pub violations: Vec<crate::rules::RuleViolation>,
    /// Confidence score
    pub confidence: f64,
}

/// Error type for workflow operations.
#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    /// Agent not found in registry
    #[error("Agent not registered: {agent}")]
    AgentNotRegistered {
        /// Agent name
        agent: String,
    },

    /// Rule not found in registry
    #[error("Rule not registered: {rule}")]
    RuleNotRegistered {
        /// Rule ID
        rule: String,
    },

    /// Execution failed
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    /// Operation timed out
    #[error("Workflow timeout after {0}ms")]
    Timeout(u64),

    /// Maximum steps exceeded
    #[error("Maximum workflow steps exceeded (limit: {0})")]
    MaxStepsExceeded(u32),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_creation() {
        let agents = AgentRegistry::new();
        let rules = RuleRegistry::new();
        let workflow = ReviewWorkflow::new(agents, rules);
        
        // Verify registries are accessible
        assert!(workflow.agents().is_empty());
        assert!(workflow.rules().is_empty());
    }

    #[tokio::test]
    async fn workflow_agent_not_found() {
        let agents = AgentRegistry::new();
        let rules = RuleRegistry::new();
        let workflow = ReviewWorkflow::new(agents, rules);

        let result = workflow.execute("nonexistent", "code", &[]).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            WorkflowError::AgentNotRegistered { agent } => {
                assert_eq!(agent, "nonexistent");
            }
            _ => panic!("Expected AgentNotRegistered error"),
        }
    }
}
