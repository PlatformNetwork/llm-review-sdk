//! OpenAI provider implementation.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;

use crate::inference::{
    CompletionRequest, CompletionResponse, LlmClient, LlmConfig, LlmError, Provider, TokenUsage,
};

pub struct OpenAIClient {
    config: LlmConfig,
    client: Client,
}

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Serialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
    #[serde(default)]
    usage: Option<OpenAIUsage>,
}

#[derive(Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessageResponse,
}

#[derive(Deserialize)]
struct OpenAIMessageResponse {
    content: String,
}

#[derive(Deserialize)]
struct OpenAIUsage {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}

impl OpenAIClient {
    pub fn new(config: LlmConfig) -> Result<Self, LlmError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| LlmError::ConnectionFailed(format!("Failed to create HTTP client: {}", e)))?;
        
        Ok(Self { config, client })
    }

    fn build_request(&self, req: &CompletionRequest) -> OpenAIRequest {
        let messages: Vec<OpenAIMessage> = req
            .messages
            .iter()
            .map(|m| OpenAIMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        OpenAIRequest {
            model: self.config.model.clone(),
            messages,
            max_tokens: Some(self.config.max_tokens),
            temperature: Some(self.config.temperature),
        }
    }

    async fn perform_request(&self, request: &OpenAIRequest) -> Result<OpenAIResponse, LlmError> {
        let api_key = self.config.api_key.as_ref().ok_or_else(|| {
            LlmError::AuthenticationFailed("OpenAI API key required".to_string())
        })?;

        let url = format!("{}/chat/completions", self.config.base_url);
        
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
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
            .json::<OpenAIResponse>()
            .await
            .map_err(|e| LlmError::ParseError(format!("Failed to parse response: {}", e)))
    }
}

#[async_trait]
impl LlmClient for OpenAIClient {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let openai_request = self.build_request(&request);
        
        const MAX_RETRIES: u32 = 3;
        let mut attempt = 0u32;
        
        loop {
            match self.perform_request(&openai_request).await {
                Ok(response) => {
                    let choice = response.choices.first().ok_or_else(|| {
                        LlmError::ParseError("No choices in response".to_string())
                    })?;

                    let usage = response.usage.map(|u| TokenUsage {
                        prompt_tokens: u.prompt_tokens,
                        completion_tokens: u.completion_tokens,
                        total_tokens: u.total_tokens,
                    });

                    return Ok(CompletionResponse {
                        content: choice.message.content.clone(),
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
        Provider::OpenAI
    }

    async fn health_check(&self) -> Result<(), LlmError> {
        let api_key = self.config.api_key.as_ref().ok_or_else(|| {
            LlmError::AuthenticationFailed("OpenAI API key required".to_string())
        })?;

        let url = format!("{}/models", self.config.base_url);
        
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
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

pub const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
