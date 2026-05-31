//! Unit tests for provider HTTP calls using wiremock (mock HTTP server).
//! These tests do NOT require any external services.

use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use shared::models::chat::{ChatRequest, Message};
use shared::providers::ollama::OllamaProvider;
use shared::providers::openai::OpenAiProvider;
use shared::providers::Provider;

fn sample_chat_request() -> ChatRequest {
    ChatRequest {
        model: "gpt-4".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: "Hello".to_string(),
        }],
        temperature: None,
        max_tokens: None,
        stream: None,
    }
}

fn mock_openai_response() -> serde_json::Value {
    serde_json::json!({
        "id": "chatcmpl-test123",
        "object": "chat.completion",
        "created": 1700000000,
        "model": "gpt-4",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "Hello! How can I help you?"},
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 5,
            "completion_tokens": 7,
            "total_tokens": 12
        }
    })
}

#[tokio::test]
async fn test_openai_provider_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("Authorization", "Bearer test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_openai_response()))
        .mount(&mock_server)
        .await;

    let provider = OpenAiProvider::new("test-api-key".to_string(), Some(mock_server.uri()));

    let request = sample_chat_request();
    let response = provider.chat_completion(&request).await.unwrap();

    assert_eq!(response.id, "chatcmpl-test123");
    assert_eq!(response.model, "gpt-4");
    assert_eq!(
        response.choices[0].message.content,
        "Hello! How can I help you?"
    );
    assert_eq!(response.usage.as_ref().unwrap().total_tokens, 12);
}

#[tokio::test]
async fn test_openai_provider_error_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
            "error": {"message": "Rate limit exceeded", "type": "rate_limit_error"}
        })))
        .mount(&mock_server)
        .await;

    let provider = OpenAiProvider::new("test-api-key".to_string(), Some(mock_server.uri()));

    let request = sample_chat_request();
    let result = provider.chat_completion(&request).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.status_code(), 502); // Provider errors map to 502
    assert!(err.to_string().contains("429"));
}

#[tokio::test]
async fn test_ollama_provider_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "ollama-123",
            "object": "chat.completion",
            "created": 1700000000,
            "model": "llama3",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "Hi there!"},
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 3,
                "completion_tokens": 4,
                "total_tokens": 7
            }
        })))
        .mount(&mock_server)
        .await;

    let provider = OllamaProvider::new(Some(mock_server.uri()));

    let mut request = sample_chat_request();
    request.model = "llama3".to_string();

    let response = provider.chat_completion(&request).await.unwrap();

    assert_eq!(response.model, "llama3");
    assert_eq!(response.choices[0].message.content, "Hi there!");
}

#[tokio::test]
async fn test_ollama_provider_connection_refused() {
    // Use a port that nothing is listening on
    let provider = OllamaProvider::new(Some("http://127.0.0.1:19999".to_string()));

    let request = sample_chat_request();
    let result = provider.chat_completion(&request).await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().status_code(), 502);
}

#[tokio::test]
async fn test_openai_provider_invalid_json_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not valid json {{{"))
        .mount(&mock_server)
        .await;

    let provider = OpenAiProvider::new("test-key".to_string(), Some(mock_server.uri()));

    let request = sample_chat_request();
    let result = provider.chat_completion(&request).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Failed to parse"));
}
