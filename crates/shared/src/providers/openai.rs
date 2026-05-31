use async_trait::async_trait;
use reqwest::Client;
use crate::models::chat::{ChatRequest, ChatResponse};
use crate::providers::Provider;
use crate::GatewayError;

pub struct OpenAiProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl OpenAiProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com".to_string()),
        }
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn supports_model(&self, model: &str) -> bool {
        model.starts_with("gpt-") || model.starts_with("o1") || model.starts_with("o3")
    }

    async fn chat_completion(&self, request: &ChatRequest) -> Result<ChatResponse, GatewayError> {
        let url = format!("{}/v1/chat/completions", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await
            .map_err(|e| GatewayError::ProviderError(format!("OpenAI request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(GatewayError::ProviderError(format!(
                "OpenAI returned {}: {}",
                status, body
            )));
        }

        response
            .json::<ChatResponse>()
            .await
            .map_err(|e| GatewayError::ProviderError(format!("Failed to parse OpenAI response: {}", e)))
    }
}

