#[cfg(test)]
mod tests {
    use shared::providers::anthropic::AnthropicProvider;
    use shared::providers::ollama::OllamaProvider;
    use shared::providers::openai::OpenAiProvider;
    use shared::providers::Provider;

    #[test]
    fn test_openai_provider_supports_gpt_models() {
        let provider = OpenAiProvider::new("test-key".to_string(), None);

        assert!(provider.supports_model("gpt-4"));
        assert!(provider.supports_model("gpt-4o"));
        assert!(provider.supports_model("gpt-3.5-turbo"));
        assert!(provider.supports_model("o1-preview"));
        assert!(provider.supports_model("o3-mini"));
        assert!(!provider.supports_model("llama3"));
        assert!(!provider.supports_model("mistral"));
    }

    #[test]
    fn test_ollama_provider_supports_local_models() {
        let provider = OllamaProvider::new(None);

        assert!(provider.supports_model("llama3"));
        assert!(provider.supports_model("mistral"));
        assert!(provider.supports_model("codellama"));
        assert!(provider.supports_model("phi3"));
        assert!(provider.supports_model("qwen2.5"));
        assert!(!provider.supports_model("gpt-4"));
        assert!(!provider.supports_model("gpt-3.5-turbo"));
        assert!(!provider.supports_model("o1-preview"));
        // Ollama should NOT claim Claude models - those belong to Anthropic
        assert!(!provider.supports_model("claude-3-5-sonnet-20241022"));
        assert!(!provider.supports_model("claude-3-opus-20240229"));
    }

    #[test]
    fn test_openai_provider_name() {
        let provider = OpenAiProvider::new("key".to_string(), None);
        assert_eq!(provider.name(), "openai");
    }

    #[test]
    fn test_ollama_provider_name() {
        let provider = OllamaProvider::new(None);
        assert_eq!(provider.name(), "ollama");
    }

    #[test]
    fn test_openai_custom_base_url() {
        let provider = OpenAiProvider::new(
            "key".to_string(),
            Some("https://custom.openai.azure.com".to_string()),
        );
        assert_eq!(provider.name(), "openai");
    }

    #[test]
    fn test_ollama_custom_base_url() {
        let provider = OllamaProvider::new(Some("http://gpu-server:11434".to_string()));
        assert_eq!(provider.name(), "ollama");
    }

    #[test]
    fn test_anthropic_provider_name() {
        let provider = AnthropicProvider::new("sk-ant-test".to_string(), None);
        assert_eq!(provider.name(), "anthropic");
    }

    #[test]
    fn test_anthropic_provider_supports_claude_models() {
        let provider = AnthropicProvider::new("sk-ant-test".to_string(), None);

        assert!(provider.supports_model("claude-3-5-sonnet-20241022"));
        assert!(provider.supports_model("claude-3-5-sonnet-latest"));
        assert!(provider.supports_model("claude-3-opus-20240229"));
        assert!(provider.supports_model("claude-3-sonnet-20240229"));
        assert!(provider.supports_model("claude-3-haiku-20240307"));
        assert!(provider.supports_model("claude-2"));
        assert!(provider.supports_model("claude-2.1"));
        // Case insensitive
        assert!(provider.supports_model("Claude-3-Sonnet"));

        // Should NOT claim OpenAI or Ollama models
        assert!(!provider.supports_model("gpt-4"));
        assert!(!provider.supports_model("gpt-4o"));
        assert!(!provider.supports_model("o1-preview"));
        assert!(!provider.supports_model("llama3"));
        assert!(!provider.supports_model("mistral"));
    }

    #[test]
    fn test_anthropic_custom_base_url() {
        let provider = AnthropicProvider::new(
            "sk-ant-test".to_string(),
            Some("https://proxy.example.com".to_string()),
        );
        assert_eq!(provider.name(), "anthropic");
        assert!(provider.supports_model("claude-3-5-sonnet-20241022"));
    }

    #[test]
    fn test_anthropic_custom_api_version() {
        let provider = AnthropicProvider::with_version(
            "sk-ant-test".to_string(),
            None,
            "2024-01-01".to_string(),
        );
        assert_eq!(provider.name(), "anthropic");
    }
}
