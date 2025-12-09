//! Tests for Twitch provider

#[cfg(test)]
mod tests {
    use super::super::*;

    // Note: Integration tests require valid Twitch credentials
    // These should be run separately with environment variables set

    #[test]
    fn test_twitch_config_creation() {
        let config = TwitchConfig::new("test_client_id", "test_secret");
        assert_eq!(config.client_id, "test_client_id");
        assert_eq!(config.client_secret, "test_secret");
    }

    // TODO: Add wiremock tests for API responses
}
