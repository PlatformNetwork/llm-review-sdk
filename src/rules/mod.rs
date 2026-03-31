//! Rule system for LLM code review.
//!
//! This module provides the core infrastructure for defining and managing
//! review rules, following the ESLint pattern where each rule has metadata
//! and a factory method to create a visitor.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

// Re-export for convenience
pub use serde_json::Value as JsonValue;

/// Severity levels for rule violations.
///
/// Ordered by severity: Error > Warning > Info.
/// This ordering is used for filtering and prioritization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Informational messages, suggestions for improvement
    Info = 1,
    /// Warnings that should be addressed but don't block
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
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Info => write!(f, "info"),
        }
    }
}
/// Error type for rule operations.
#[derive(Debug, Clone, thiserror::Error, Serialize, Deserialize)]
pub enum RuleError {
    /// Rule not found
    #[error("Rule not found: {0}")]
    NotFound(String),
    
    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    
    /// Execution error
    #[error("Rule execution error: {0}")]
    ExecutionError(String),
}


/// Metadata describing a rule.
///
/// Contains all the information needed to identify, describe, and configure
/// a rule. This follows the ESLint rule meta pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleMeta {
    /// Unique identifier for the rule (e.g., "no-unused-vars")
    pub id: String,
    /// Human-readable description of what the rule checks
    pub description: String,
    /// Default severity level for violations from this rule
    pub severity: Severity,
    /// JSON Schema for rule configuration options, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<JsonValue>,
}

impl RuleMeta {
    /// Creates a new RuleMeta with the given id and description.
    ///
    /// Uses default severity (Warning) and no schema.
    pub fn new(id: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            severity: Severity::default(),
            schema: None,
        }
    }

    /// Sets the severity level.
    pub fn severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// Sets the JSON Schema for rule options.
    pub fn schema(mut self, schema: JsonValue) -> Self {
        self.schema = Some(schema);
        self
    }

    /// Builder pattern: with severity
    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// Builder pattern: with schema
    pub fn with_schema(mut self, schema: JsonValue) -> Self {
        self.schema = Some(schema);
        self
    }
}

/// Context provided to rules during analysis.
///
/// Note: This is a forward declaration. The full implementation
/// is in `context.rs` (separate task).
#[derive(Debug, Clone)]
pub struct RuleContext {
    /// Placeholder for context data
    _marker: std::marker::PhantomData<()>,
}

impl Default for RuleContext {
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

/// Visitor trait for traversing and analyzing code.
///
/// Rules implement this trait to perform their analysis.
/// Note: This is a forward declaration. The full implementation
/// is in `context.rs` (separate task).
pub trait RuleVisitor: Send {
    /// Called when analysis completes
    fn finish(self: Box<Self>) -> Vec<RuleViolation>;
}

/// A violation found by a rule.
///
/// Note: This is a forward declaration. The full implementation
/// is in `violation.rs` (separate task).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleViolation {
    /// The rule ID that produced this violation
    pub rule_id: String,
    /// Severity level
    pub severity: Severity,
    /// Human-readable message
    pub message: String,
}

impl RuleViolation {
    /// Creates a new violation
    pub fn new(rule_id: impl Into<String>, severity: Severity, message: impl Into<String>) -> Self {
        Self {
            rule_id: rule_id.into(),
            severity,
            message: message.into(),
        }
    }
}

/// Trait for defining review rules.
///
/// Rules follow the ESLint pattern: they have metadata describing themselves
/// and a factory method to create a visitor for analysis.
///
/// This trait is object-safe, allowing rules to be stored as `dyn Rule`.
pub trait Rule: Send + Sync {
    /// Returns the rule's metadata.
    fn meta(&self) -> &RuleMeta;

    /// Creates a new visitor for this rule with the given context.
    fn create(&self, context: RuleContext) -> Box<dyn RuleVisitor>;

    /// Returns a boxed clone of this rule.
    ///
    /// Required for storing rules in registries.
    fn boxed_clone(&self) -> Box<dyn Rule>;
}

/// A typed rule that wraps a function-based implementation.
///
/// Useful for simple rules that don't need full visitor complexity.
pub struct SimpleRule {
    meta: RuleMeta,
    create_fn: fn(RuleContext) -> Box<dyn RuleVisitor>,
}

impl SimpleRule {
    /// Creates a new simple rule.
    pub fn new(
        meta: RuleMeta,
        create_fn: fn(RuleContext) -> Box<dyn RuleVisitor>,
    ) -> Self {
        Self { meta, create_fn }
    }
}

impl Rule for SimpleRule {
    fn meta(&self) -> &RuleMeta {
        &self.meta
    }

    fn create(&self, context: RuleContext) -> Box<dyn RuleVisitor> {
        (self.create_fn)(context)
    }

    fn boxed_clone(&self) -> Box<dyn Rule> {
        Box::new(Self {
            meta: self.meta.clone(),
            create_fn: self.create_fn,
        })
    }
}

/// Registry for discovering and managing rules.
///
/// Rules are registered with unique IDs and can be retrieved later.
#[derive(Default)]
pub struct RuleRegistry {
    rules: HashMap<String, Box<dyn Rule>>,
}

impl RuleRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a rule with the registry.
    ///
    /// Returns the previous rule with the same ID, if any.
    pub fn register<R: Rule + 'static>(&mut self, rule: R) -> Option<Box<dyn Rule>> {
        let id = rule.meta().id.clone();
        self.rules.insert(id, Box::new(rule))
    }

    /// Registers a boxed rule directly.
    pub fn register_boxed(&mut self, rule: Box<dyn Rule>) -> Option<Box<dyn Rule>> {
        let id = rule.meta().id.clone();
        self.rules.insert(id, rule)
    }

    /// Gets a rule by its ID.
    pub fn get(&self, id: &str) -> Option<&dyn Rule> {
        self.rules.get(id).map(|r| r.as_ref())
    }

    /// Checks if a rule is registered.
    pub fn contains(&self, id: &str) -> bool {
        self.rules.contains_key(id)
    }

    /// Returns the number of registered rules.
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Returns true if no rules are registered.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Returns an iterator over all registered rules.
    pub fn iter(&self) -> impl Iterator<Item = &dyn Rule> {
        self.rules.values().map(|r| r.as_ref())
    }

    /// Returns all registered rule IDs.
    pub fn rule_ids(&self) -> impl Iterator<Item = &String> {
        self.rules.keys()
    }

    /// Removes a rule by ID.
    pub fn remove(&mut self, id: &str) -> Option<Box<dyn Rule>> {
        self.rules.remove(id)
    }

    /// Clears all rules from the registry.
    pub fn clear(&mut self) {
        self.rules.clear();
    }
}

impl Clone for RuleRegistry {
    fn clone(&self) -> Self {
        let rules = self
            .rules
            .iter()
            .map(|(k, v)| (k.clone(), v.boxed_clone()))
            .collect();
        Self { rules }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_ordering() {
        // Test explicit ordering: Error > Warning > Info
        assert!(Severity::Error > Severity::Warning);
        assert!(Severity::Warning > Severity::Info);
        assert!(Severity::Error > Severity::Info);

        // Test equality
        assert!(Severity::Error == Severity::Error);
        assert!(Severity::Warning == Severity::Warning);
        assert!(Severity::Info == Severity::Info);

        // Test inequality
        assert!(Severity::Error != Severity::Warning);
        assert!(Severity::Warning != Severity::Info);

        // Test ordering chain
        let severities = vec![Severity::Info, Severity::Error, Severity::Warning];
        let mut sorted = severities;
        sorted.sort();
        assert_eq!(sorted, vec![Severity::Info, Severity::Warning, Severity::Error]);

        // Test reverse sorting
        sorted.sort_by(|a, b| b.cmp(a));
        assert_eq!(sorted, vec![Severity::Error, Severity::Warning, Severity::Info]);
    }

    #[test]
    fn severity_default() {
        assert_eq!(Severity::default(), Severity::Warning);
    }

    #[test]
    fn severity_display() {
        assert_eq!(Severity::Error.to_string(), "error");
        assert_eq!(Severity::Warning.to_string(), "warning");
        assert_eq!(Severity::Info.to_string(), "info");
    }

    #[test]
    fn severity_serialization() {
        // Test JSON serialization
        let error = serde_json::to_string(&Severity::Error).unwrap();
        assert_eq!(error, "\"error\"");

        let warning = serde_json::to_string(&Severity::Warning).unwrap();
        assert_eq!(warning, "\"warning\"");

        let info = serde_json::to_string(&Severity::Info).unwrap();
        assert_eq!(info, "\"info\"");
    }

    #[test]
    fn severity_deserialization() {
        let error: Severity = serde_json::from_str("\"error\"").unwrap();
        assert_eq!(error, Severity::Error);

        let warning: Severity = serde_json::from_str("\"warning\"").unwrap();
        assert_eq!(warning, Severity::Warning);

        let info: Severity = serde_json::from_str("\"info\"").unwrap();
        assert_eq!(info, Severity::Info);
    }

    #[test]
    fn rule_meta_builder() {
        let meta = RuleMeta::new("test-rule", "A test rule")
            .severity(Severity::Error)
            .schema(serde_json::json!({"type": "object"}));

        assert_eq!(meta.id, "test-rule");
        assert_eq!(meta.description, "A test rule");
        assert_eq!(meta.severity, Severity::Error);
        assert!(meta.schema.is_some());
    }

    #[test]
    fn rule_meta_serialization() {
        let meta = RuleMeta::new("test-rule", "A test rule")
            .severity(Severity::Error);

        let json = serde_json::to_string(&meta).unwrap();
        assert!(json.contains("test-rule"));
        assert!(json.contains("A test rule"));
        assert!(json.contains("error"));
        assert!(!json.contains("schema")); // Should be omitted when None

        let meta_with_schema = RuleMeta::new("test-rule", "A test rule")
            .schema(serde_json::json!({"type": "object"}));

        let json = serde_json::to_string(&meta_with_schema).unwrap();
        assert!(json.contains("schema"));
    }

    struct TestVisitor;

    impl RuleVisitor for TestVisitor {
        fn finish(self: Box<Self>) -> Vec<RuleViolation> {
            vec![]
        }
    }

    struct TestRule {
        meta: RuleMeta,
    }

    impl Rule for TestRule {
        fn meta(&self) -> &RuleMeta {
            &self.meta
        }

        fn create(&self, _context: RuleContext) -> Box<dyn RuleVisitor> {
            Box::new(TestVisitor)
        }

        fn boxed_clone(&self) -> Box<dyn Rule> {
            Box::new(TestRule {
                meta: self.meta.clone(),
            })
        }
    }

    #[test]
    fn rule_registry_basic() {
        let mut registry = RuleRegistry::new();

        let rule = TestRule {
            meta: RuleMeta::new("test-rule", "A test rule"),
        };

        registry.register(rule);

        assert!(registry.contains("test-rule"));
        assert!(!registry.contains("unknown-rule"));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn rule_registry_get() {
        let mut registry = RuleRegistry::new();

        let rule = TestRule {
            meta: RuleMeta::new("test-rule", "A test rule")
                .severity(Severity::Error),
        };

        registry.register(rule);

        let retrieved = registry.get("test-rule").unwrap();
        assert_eq!(retrieved.meta().id, "test-rule");
        assert_eq!(retrieved.meta().severity, Severity::Error);
    }

    #[test]
    fn rule_registry_replace() {
        let mut registry = RuleRegistry::new();

        let rule1 = TestRule {
            meta: RuleMeta::new("test-rule", "First rule"),
        };
        let rule2 = TestRule {
            meta: RuleMeta::new("test-rule", "Second rule"),
        };

        registry.register(rule1);
        let old = registry.register(rule2);

        assert!(old.is_some());
        assert_eq!(old.unwrap().meta().description, "First rule");
        assert_eq!(registry.get("test-rule").unwrap().meta().description, "Second rule");
    }

    #[test]
    fn rule_registry_remove() {
        let mut registry = RuleRegistry::new();

        let rule = TestRule {
            meta: RuleMeta::new("test-rule", "A test rule"),
        };

        registry.register(rule);
        assert_eq!(registry.len(), 1);

        let removed = registry.remove("test-rule");
        assert!(removed.is_some());
        assert_eq!(registry.len(), 0);
        assert!(!registry.contains("test-rule"));
    }

    #[test]
    fn rule_registry_clone() {
        let mut registry = RuleRegistry::new();

        let rule = TestRule {
            meta: RuleMeta::new("test-rule", "A test rule"),
        };

        registry.register(rule);
        let cloned = registry.clone();

        assert_eq!(cloned.len(), 1);
        assert!(cloned.contains("test-rule"));
    }

    #[test]
    fn rule_violation_creation() {
        let violation = RuleViolation::new("test-rule", Severity::Warning, "Test message");

        assert_eq!(violation.rule_id, "test-rule");
        assert_eq!(violation.severity, Severity::Warning);
        assert_eq!(violation.message, "Test message");
    }

    #[test]
    fn simple_rule_creation() {
        fn create_visitor(_ctx: RuleContext) -> Box<dyn RuleVisitor> {
            Box::new(TestVisitor)
        }

        let meta = RuleMeta::new("simple-test", "A simple test rule");
        let rule = SimpleRule::new(meta, create_visitor);

        assert_eq!(rule.meta().id, "simple-test");

        let visitor = rule.create(RuleContext::default());
        let violations = visitor.finish();
        assert!(violations.is_empty());
    }

    #[test]
    fn registry_iteration() {
        let mut registry = RuleRegistry::new();

        registry.register(TestRule {
            meta: RuleMeta::new("rule-1", "First rule"),
        });
        registry.register(TestRule {
            meta: RuleMeta::new("rule-2", "Second rule"),
        });

        let ids: Vec<_> = registry.rule_ids().collect();
        assert_eq!(ids.len(), 2);

        let rules: Vec<_> = registry.iter().collect();
        assert_eq!(rules.len(), 2);
    }
}
