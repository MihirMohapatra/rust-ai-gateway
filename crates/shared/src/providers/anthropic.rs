use serde::{Deserialize, Serialize};

use crate::models::chat::{ChatRequest, ChatResponse, Choice, Message, Usage};
use crate::providers::Provider;
use crate::GatewayError;
use async_trait::async_trait;
use reqwest::Client;

const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";
const DEFAULT_API_VERSION: &str = "2023-06-01";
const DEFAULT_MAX_TOKENS: u32 = 1024;

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    base_url: String,
    api_version: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self::with_version(api_key, base_url, DEFAULT_API_VERSION.to_string())
    }

    pub fn with_version(api_key: String, base_url: Option<String>, api_version: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
            api_version,
        }
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn supports_model(&self, model: &str) -> bool {
        let m = model.to_lowercase();
        m.starts_with("claude")
            || m.starts_with("anthropic/")
            || m == "claude"
            || m.starts_with("claude-3")
            || m.starts_with("claude-2")
    }

    async fn chat_completion(&self, request: &ChatRequest) -> Result<ChatResponse, GatewayError> {
        let url = format!("{}/v1/messages", self.base_url);

        let (system_prompt, user_messages) = split_system_message(&request.messages);

        if user_messages.is_empty() {
            return Err(GatewayError::BadRequest(
                "Anthropic requires at least one non-system message".to_string(),
            ));
        }

        let max_tokens = request.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS);

        let mut body = serde_json::json!({
            "model": request.model,
            "max_tokens": max_tokens,
            "messages": user_messages,
        });

        if let Some(system) = system_prompt {
            body["system"] = serde_json::Value::String(system);
        }

        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::Value::from(temp);
        }

        if let Some(true) = request.stream {
            return Err(GatewayError::BadRequest(
                "Streaming is not yet supported for Anthropic provider".to_string(),
            ));
        }

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", &self.api_version)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| GatewayError::ProviderError(format!("Anthropic request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(GatewayError::ProviderError(format!(
                "Anthropic returned {}: {}",
                status, body
            )));
        }

        let parsed: AnthropicResponse = response.json().await.map_err(|e| {
            GatewayError::ProviderError(format!("Failed to parse Anthropic response: {}", e))
        })?;

        Ok(parsed.into_chat_response())
    }
}

fn split_system_message(messages: &[Message]) -> (Option<String>, Vec<AnthropicMessage>) {
    let mut system_parts: Vec<String> = Vec::new();
    let mut converted: Vec<AnthropicMessage> = Vec::new();

    for m in messages {
        match m.role.as_str() {
            "system" | "developer" => system_parts.push(m.content.clone()),
            "assistant" => converted.push(AnthropicMessage {
                role: "assistant".to_string(),
                content: m.content.clone(),
            }),
            _ => converted.push(AnthropicMessage {
                role: "user".to_string(),
                content: m.content.clone(),
            }),
        }
    }

    let system = if system_parts.is_empty() {
        None
    } else {
        Some(system_parts.join("\n\n"))
    };

    (system, converted)
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    id: String,
    #[serde(default)]
    model: String,
    #[serde(default)]
    stop_reason: Option<String>,
    content: Vec<AnthropicContentBlock>,
    #[serde(default)]
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicContentBlock {
    Text { text: String },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
}

impl AnthropicResponse {
    fn into_chat_response(self) -> ChatResponse {
        let text: String = self
            .content
            .into_iter()
            .map(|block| match block {
                AnthropicContentBlock::Text { text } => text,
                AnthropicContentBlock::Other => String::new(),
            })
            .collect::<Vec<_>>()
            .join("");

        let created = chrono::Utc::now().timestamp();

        let usage = self.usage.map(|u| Usage {
            prompt_tokens: u.input_tokens,
            completion_tokens: u.output_tokens,
            total_tokens: u.input_tokens + u.output_tokens,
        });

        ChatResponse {
            id: self.id,
            object: "chat.completion".to_string(),
            created,
            model: self.model,
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: "assistant".to_string(),
                    content: text,
                },
                finish_reason: self.stop_reason,
            }],
            usage,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_system_message_with_system() {
        let messages = vec![
            Message {
                role: "system".to_string(),
                content: "Be concise".to_string(),
            },
            Message {
                role: "user".to_string(),
                content: "Hi".to_string(),
            },
        ];

        let (system, conv) = split_system_message(&messages);
        assert_eq!(system, Some("Be concise".to_string()));
        assert_eq!(conv.len(), 1);
        assert_eq!(conv[0].role, "user");
    }

    #[test]
    fn test_split_system_message_multiple_system() {
        let messages = vec![
            Message {
                role: "system".to_string(),
                content: "Rule 1".to_string(),
            },
            Message {
                role: "system".to_string(),
                content: "Rule 2".to_string(),
            },
            Message {
                role: "user".to_string(),
                content: "Hi".to_string(),
            },
        ];

        let (system, conv) = split_system_message(&messages);
        assert_eq!(system, Some("Rule 1\n\nRule 2".to_string()));
        assert_eq!(conv.len(), 1);
    }

    #[test]
    fn test_split_system_message_no_system() {
        let messages = vec![Message {
            role: "user".to_string(),
            content: "Hi".to_string(),
        }];

        let (system, conv) = split_system_message(&messages);
        assert_eq!(system, None);
        assert_eq!(conv.len(), 1);
    }

    #[test]
    fn test_supports_model() {
        let p = AnthropicProvider::new("k".to_string(), None);
        assert!(p.supports_model("claude-3-5-sonnet-20241022"));
        assert!(p.supports_model("claude-3-opus-20240229"));
        assert!(p.supports_model("claude-3-haiku-20240307"));
        assert!(p.supports_model("claude-2"));
        assert!(p.supports_model("Claude-3-Sonnet"));
        assert!(!p.supports_model("gpt-4"));
        assert!(!p.supports_model("llama3"));
    }
}
