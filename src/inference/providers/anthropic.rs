//! Anthropic provider implementation.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;

use crate::inference::{
    CompletionRequest, CompletionResponse, LlmClient, LlmConfig, LlmError, Provider, TokenUsage,
};

pub struct AnthropicClient {
    config: LlmConfig,
    client: Client,
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: usize,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
    #[serde(default)]
    usage: Option<AnthropicUsage>,
}

#[derive(Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: usize,
    output_tokens: usize,
}

impl AnthropicClient {
    pub fn new(config: LlmConfig) -> Result<Self, LlmError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| LlmError::ConnectionFailed(format!("Failed to create HTTP client: {}", e)))?;
        
        Ok(Self { config, client })
    }

    fn build_request(&self, req: &CompletionRequest) -> AnthropicRequest {
        let messages: Vec<AnthropicMessage> = req
            .messages
            .iter()
            .filter(|m| m.role != "system")
            .map(|m| AnthropicMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        AnthropicRequest {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            messages,
            temperature: Some(self.config.temperature),
        }
    }

    async fn perform_request(&self, request: &AnthropicRequest) -> Result<AnthropicResponse, LlmError> {
        let api_key = self.config.api_key.as_ref().ok_or_else(|| {
            LlmError::AuthenticationFailed("Anthropic API key required".to_string())
        })?;

        let url = format!("{}/messages", self.config.base_url);
        
        let response = self
            .client
            .post(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(request)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    LlmError::Timeout(120_000)
                } else {
                    LlmError::ConnectionFailed(e.to_string())
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(match status.as_u16() {
                401 => LlmError::AuthenticationFailed(error_text),
                404 => LlmError::ModelNotFound(self.config.model.clone()),
                429 => LlmError::RateLimited(error_text),
                _ => LlmError::ApiError(format!("HTTP {}: {}", status, error_text)),
            });
        }

        response
            .json::<AnthropicResponse>()
            .await
            .map_err(|e| LlmError::ParseError(format!("Failed to parse response: {}", e)))
    }
}

#[async_trait]
impl LlmClient for AnthropicClient {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let anthropic_request = self.build_request(&request);
        
        const MAX_RETRIES: u32 = 3;
        let mut attempt = 0u32;
        
        loop {
            match self.perform_request(&anthropic_request).await {
                Ok(response) => {
                    let content = response
                        .content
                        .iter()
                        .filter_map(|c| {
                            if c.content_type == "text" {
                                c.text.clone()
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n");

                    let usage = response.usage.map(|u| TokenUsage {
                        prompt_tokens: u.input_tokens,
                        completion_tokens: u.output_tokens,
                        total_tokens: u.input_tokens + u.output_tokens,
                    });

                    return Ok(CompletionResponse {
                        content,
                        usage,
                    });
                }
                Err(LlmError::RateLimited(_)) if attempt < MAX_RETRIES => {
                    attempt += 1;
                    sleep(Duration::from_secs(2u64.pow(attempt))).await;
                    continue;
                }
                Err(LlmError::ConnectionFailed(_)) if attempt < MAX_RETRIES => {
                    attempt += 1;
                    sleep(Duration::from_secs(2u64.pow(attempt))).await;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

    fn config(&self) -> &LlmConfig {
        &self.config
    }

    fn provider(&self) -> Provider {
        Provider::Anthropic
    }

    async fn health_check(&self) -> Result<(), LlmError> {
        let api_key = self.config.api_key.as_ref().ok_or_else(|| {
            LlmError::AuthenticationFailed("Anthropic API key required".to_string())
        })?;

        let url = format!("{}/messages", self.config.base_url);
        
        let response = self
            .client
            .post(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": self.config.model,
                "max_tokens": 1,
                "messages": [{"role": "user", "content": "hi"}]
            }))
            .send()
            .await
            .map_err(|e| LlmError::ConnectionFailed(e.to_string()))?;

        let status = response.status();
        if status.is_success() || status.as_u16() == 400 {
            Ok(())
        } else if status.as_u16() == 401 {
            Err(LlmError::AuthenticationFailed("Invalid API key".to_string()))
        } else {
            Err(LlmError::ConnectionFailed(format!(
                "Health check failed with status {}",
                status
            )))
        }
    }

    fn boxed_clone(&self) -> Box<dyn LlmClient> {
        Box::new(Self {
            config: self.config.clone(),
            client: self.client.clone(),
        })
    }
}

pub const DEFAULT_BASE_URL: &str = "https://api.anthropic.com/v1";
