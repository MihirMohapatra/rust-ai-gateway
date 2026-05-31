use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub database_url: String,
    pub redis_url: String,
    pub host: String,
    pub port: u16,
    pub openai_api_key: Option<String>,
    pub ollama_base_url: Option<String>,
    pub jwt_secret: String,
    pub global_rate_limit: u32, // requests per minute globally
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgres://postgres:postgres@localhost:5432/ai_gateway".to_string()
            }),
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .unwrap_or(3000),
            openai_api_key: std::env::var("OPENAI_API_KEY").ok(),
            ollama_base_url: std::env::var("OLLAMA_BASE_URL").ok(),
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "dev-secret-change-in-production".to_string()),
            global_rate_limit: std::env::var("GLOBAL_RATE_LIMIT")
                .unwrap_or_else(|_| "1000".to_string())
                .parse()
                .unwrap_or(1000),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_config_defaults() {
        // Clear env vars to test defaults
        std::env::remove_var("DATABASE_URL");
        std::env::remove_var("REDIS_URL");
        std::env::remove_var("HOST");
        std::env::remove_var("PORT");
        std::env::remove_var("JWT_SECRET");
        std::env::remove_var("GLOBAL_RATE_LIMIT");
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("OLLAMA_BASE_URL");

        let config = AppConfig::from_env();

        assert_eq!(
            config.database_url,
            "postgres://postgres:postgres@localhost:5432/ai_gateway"
        );
        assert_eq!(config.redis_url, "redis://localhost:6379");
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 3000);
        assert_eq!(config.jwt_secret, "dev-secret-change-in-production");
        assert_eq!(config.global_rate_limit, 1000);
    }

    #[test]
    #[serial]
    fn test_config_from_env() {
        std::env::set_var("PORT", "8080");
        std::env::set_var("GLOBAL_RATE_LIMIT", "500");

        let config = AppConfig::from_env();

        assert_eq!(config.port, 8080);
        assert_eq!(config.global_rate_limit, 500);

        // Cleanup
        std::env::remove_var("PORT");
        std::env::remove_var("GLOBAL_RATE_LIMIT");
    }

    #[test]
    #[serial]
    fn test_config_invalid_port_uses_default() {
        std::env::set_var("PORT", "not_a_number");

        let config = AppConfig::from_env();
        assert_eq!(config.port, 3000);

        std::env::remove_var("PORT");
    }
}
