//! Ollama provider implementation.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;

use crate::inference::{
    CompletionRequest, CompletionResponse, LlmClient, LlmConfig, LlmError, Provider, TokenUsage,
};

pub struct OllamaClient {
    config: LlmConfig,
    client: Client,
}

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

#[derive(Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<usize>,
}

#[derive(Deserialize)]
struct OllamaResponse {
    message: OllamaResponseMessage,
    #[serde(default)]
    prompt_eval_count: Option<usize>,
    #[serde(default)]
    eval_count: Option<usize>,
}

#[derive(Deserialize)]
struct OllamaResponseMessage {
    content: String,
}

impl OllamaClient {
    pub fn new(config: LlmConfig) -> Result<Self, LlmError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| LlmError::ConnectionFailed(format!("Failed to create HTTP client: {}", e)))?;
        
        Ok(Self { config, client })
    }

    fn build_request(&self, req: &CompletionRequest) -> OllamaRequest {
        let messages: Vec<OllamaMessage> = req
            .messages
            .iter()
            .map(|m| OllamaMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        OllamaRequest {
            model: self.config.model.clone(),
            messages,
            stream: false,
            options: Some(OllamaOptions {
                temperature: Some(self.config.temperature),
                num_predict: Some(self.config.max_tokens),
            }),
        }
    }

    async fn perform_request(&self, request: &OllamaRequest) -> Result<OllamaResponse, LlmError> {
        let url = format!("{}/api/chat", self.config.base_url);
        
        let response = self
            .client
            .post(&url)
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
            .json::<OllamaResponse>()
            .await
            .map_err(|e| LlmError::ParseError(format!("Failed to parse response: {}", e)))
    }
}

#[async_trait]
impl LlmClient for OllamaClient {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let ollama_request = self.build_request(&request);
        
        const MAX_RETRIES: u32 = 3;
        let mut attempt = 0u32;
        
        loop {
            match self.perform_request(&ollama_request).await {
                Ok(response) => {
                    let usage = response.prompt_eval_count.map(|prompt| TokenUsage {
                        prompt_tokens: prompt,
                        completion_tokens: response.eval_count.unwrap_or(0),
                        total_tokens: prompt + response.eval_count.unwrap_or(0),
                    });

                    return Ok(CompletionResponse {
                        content: response.message.content,
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
        Provider::Ollama
    }

    async fn health_check(&self) -> Result<(), LlmError> {
        let url = format!("{}/api/tags", self.config.base_url);
        
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| LlmError::ConnectionFailed(e.to_string()))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(LlmError::ConnectionFailed(format!(
                "Health check failed with status {}",
                response.status()
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

pub const DEFAULT_BASE_URL: &str = "http://localhost:11434";
