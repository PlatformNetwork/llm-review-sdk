# LLM Review SDK

SDK for LLM-powered code review in Platform Network challenges. Provides infrastructure for defining review rules, LLM-powered agents, and HTTP server APIs for miner and validator integration.

## Features

- **Rules Engine**: Configurable review rules following ESLint's meta + create pattern
- **Agents**: LLM-powered review agents with function calling capabilities  
- **Inference**: Multi-provider LLM abstraction (Ollama, OpenAI, Anthropic)
- **Workflow**: Orchestration layer for coordinating review pipelines
- **Server**: HTTP server for exposing review functionality

## Feature Flags

| Flag | Default | Description |
|------|---------|-------------|
| `std` | Yes | Enables full standard library support |
| `async` | No | Enables async runtime support (tokio, reqwest) |
| `plagiarism` | No | Enables plagiarism detection integration |

### Adding to Cargo.toml

```toml
[dependencies]
llm-review-sdk = "0.1"

# With specific features
llm-review-sdk = { version = "0.1", features = ["async", "plagiarism"] }
```

## Quick Start

### Setting Up ExecutorServer (Miner Side)

```rust
use llm_review_sdk::server::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = Server::bind(8080);
    println!("Server listening on {}", server.addr());
    server.serve().await?;
    Ok(())
}
```

### Using ValidatorClient (Validator Side)

```rust
use llm_review_sdk::server::{InferenceRequest, RuleConfig};
use llm_review_sdk::agents::AgentConfig;

let request = InferenceRequest {
    agent_name: "code-reviewer".to_string(),
    input_code: code.to_string(),
    comparison_code: None,
    rules: vec![RuleConfig {
        rule_id: "no-unused-vars".to_string(),
        options: serde_json::Value::Null,
    }],
    config: AgentConfig::default(),
    request_id: uuid::Uuid::new_v4().to_string(),
};
```

### Defining Custom Rules

```rust
use llm_review_sdk::rules::{Rule, RuleMeta, RuleContext, RuleVisitor, Severity, SimpleRule};

let rule = SimpleRule::new(
    RuleMeta::new("no-debug", "Disallow debug statements")
        .severity(Severity::Warning),
    |ctx: RuleContext| -> Box<dyn RuleVisitor> {
        // Return visitor implementation
        todo!()
    },
);
```

## HTTP API

### POST /api/v1/inference

Submit code for LLM-based review.

**Request:**
```json
{
  "agent_name": "code-reviewer",
  "input_code": "fn main() { let x = 1; }",
  "rules": [{"rule_id": "no-unused-vars", "options": null}],
  "config": {"max_iterations": 10, "timeout_ms": 30000},
  "request_id": "uuid"
}
```

**Response:**
```json
{
  "request_id": "uuid",
  "violations": [{"rule_id": "no-unused-vars", "severity": "warning", "message": "..."}],
  "summary": "1 warning found",
  "confidence": 0.95,
  "duration_ms": 150
}
```

### GET /health

Health check endpoint.

## LLM Configuration

```rust
use llm_review_sdk::inference::{LlmConfig, Provider};

// Ollama (local)
let ollama = LlmConfig {
    provider: Provider::Ollama,
    base_url: "http://localhost:11434".to_string(),
    model: "llama2".to_string(),
    ..Default::default()
};

// OpenAI
let openai = LlmConfig {
    provider: Provider::OpenAI,
    base_url: "https://api.openai.com/v1".to_string(),
    api_key: Some(env::var("OPENAI_API_KEY").unwrap()),
    model: "gpt-4".to_string(),
    ..Default::default()
};

// Anthropic  
let anthropic = LlmConfig {
    provider: Provider::Anthropic,
    base_url: "https://api.anthropic.com/v1".to_string(),
    api_key: Some(env::var("ANTHROPIC_API_KEY").unwrap()),
    model: "claude-3-sonnet".to_string(),
    ..Default::default()
};
```

## Module Reference

| Module | Description |
|--------|-------------|
| `rules` | Rule DSL (always available) |
| `agents` | ReviewAgent implementations (requires `std`) |
| `inference` | LLM providers (requires `std`) |
| `workflow` | Review orchestration (requires `std`) |
| `server` | HTTP server/client (requires `std`) |
| `integration` | Plagiarism SDK integration (requires `plagiarism`) |

## License

Apache-2.0
