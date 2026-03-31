//! LLM provider implementations.

pub mod anthropic;
pub mod ollama;
pub mod openai;

pub use anthropic::AnthropicClient;
pub use ollama::OllamaClient;
pub use openai::OpenAIClient;

pub use anthropic::DEFAULT_BASE_URL as ANTHROPIC_DEFAULT_URL;
pub use ollama::DEFAULT_BASE_URL as OLLAMA_DEFAULT_URL;
pub use openai::DEFAULT_BASE_URL as OPENAI_DEFAULT_URL;
