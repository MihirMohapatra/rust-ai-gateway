pub mod openai;
pub mod ollama;

use async_trait::async_trait;
use crate::models::chat::{ChatRequest, ChatResponse};
use crate::GatewayError;

#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    fn supports_model(&self, model: &str) -> bool;
    async fn chat_completion(&self, request: &ChatRequest) -> Result<ChatResponse, GatewayError>;
}

