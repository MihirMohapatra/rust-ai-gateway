#[cfg(test)]
mod tests {
    use shared::providers::Provider;
    use shared::providers::openai::OpenAiProvider;
    use shared::providers::ollama::OllamaProvider;

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
        assert!(!provider.supports_model("gpt-4"));
        assert!(!provider.supports_model("gpt-3.5-turbo"));
        assert!(!provider.supports_model("o1-preview"));
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
}

