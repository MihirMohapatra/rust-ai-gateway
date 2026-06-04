use crate::models::chat::{ChatRequest, ChatResponse};
use crate::providers::Provider;
use crate::GatewayError;
use async_trait::async_trait;
use reqwest::Client;

pub struct OllamaProvider {
    client: Client,
    base_url: String,
}

impl OllamaProvider {
    pub fn new(base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.unwrap_or_else(|| "http://localhost:11434".to_string()),
        }
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    fn supports_model(&self, model: &str) -> bool {
        // Ollama supports local models like llama3, mistral, codellama, phi3, gemma, etc.
        // Explicit allowlist for known local families - we no longer act as a catch-all
        // so that other providers (e.g. Anthropic) can claim their own model families.
        let m = model.to_lowercase();
        m.starts_with("llama")
            || m.starts_with("mistral")
            || m.starts_with("mixtral")
            || m.starts_with("codellama")
            || m.starts_with("phi")
            || m.starts_with("gemma")
            || m.starts_with("qwen")
            || m.starts_with("deepseek")
            || m.starts_with("vicuna")
            || m.starts_with("dolphin")
            || m.starts_with("nous")
            || m.starts_with("orca")
    }

    async fn chat_completion(&self, request: &ChatRequest) -> Result<ChatResponse, GatewayError> {
        // Ollama has an OpenAI-compatible endpoint
        let url = format!("{}/v1/chat/completions", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await
            .map_err(|e| GatewayError::ProviderError(format!("Ollama request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(GatewayError::ProviderError(format!(
                "Ollama returned {}: {}",
                status, body
            )));
        }

        response.json::<ChatResponse>().await.map_err(|e| {
            GatewayError::ProviderError(format!("Failed to parse Ollama response: {}", e))
        })
    }
}
