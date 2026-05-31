#[cfg(test)]
mod tests {
    use shared::models::chat::{ChatRequest, ChatResponse, Choice, Message, Usage};
    use shared::GatewayError;

    #[test]
    fn test_chat_request_serialization() {
        let request = ChatRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: "You are a helpful assistant.".to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: "Hello!".to_string(),
                },
            ],
            temperature: Some(0.7),
            max_tokens: Some(100),
            stream: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: ChatRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.model, "gpt-4");
        assert_eq!(deserialized.messages.len(), 2);
        assert_eq!(deserialized.messages[0].role, "system");
        assert_eq!(deserialized.temperature, Some(0.7));
        assert_eq!(deserialized.max_tokens, Some(100));
    }

    #[test]
    fn test_chat_response_deserialization() {
        let json = r#"{
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "created": 1700000000,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "Hello! How can I help?"},
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 8,
                "total_tokens": 18
            }
        }"#;

        let response: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.id, "chatcmpl-123");
        assert_eq!(response.model, "gpt-4");
        assert_eq!(response.choices.len(), 1);
        assert_eq!(response.choices[0].message.content, "Hello! How can I help?");
        assert_eq!(response.usage.as_ref().unwrap().total_tokens, 18);
    }

    #[test]
    fn test_chat_request_minimal() {
        let json = r#"{"model": "llama3", "messages": [{"role": "user", "content": "Hi"}]}"#;
        let request: ChatRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.model, "llama3");
        assert_eq!(request.temperature, None);
        assert_eq!(request.max_tokens, None);
        assert_eq!(request.stream, None);
    }

    #[test]
    fn test_gateway_error_status_codes() {
        assert_eq!(GatewayError::AuthError("test".into()).status_code(), 401);
        assert_eq!(GatewayError::RateLimited.status_code(), 429);
        assert_eq!(GatewayError::BadRequest("test".into()).status_code(), 400);
        assert_eq!(GatewayError::ProviderError("test".into()).status_code(), 502);
        assert_eq!(GatewayError::Internal("test".into()).status_code(), 500);
    }

    #[test]
    fn test_gateway_error_display() {
        let err = GatewayError::RateLimited;
        assert_eq!(err.to_string(), "Rate limit exceeded");

        let err = GatewayError::AuthError("bad token".into());
        assert_eq!(err.to_string(), "Authentication failed: bad token");
    }
}

