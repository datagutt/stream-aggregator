//! OAuth2 token management for Twitch

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info};
use wreq::Client;

use crate::models::{TokenResponse, TwitchConfig, TwitchError};

const TOKEN_URL: &str = "https://id.twitch.tv/oauth2/token";

/// Manages OAuth2 tokens for Twitch API
pub struct TokenManager {
    client: Client,
    config: TwitchConfig,
    token_data: Arc<RwLock<TokenData>>,
}

#[derive(Default)]
struct TokenData {
    access_token: Option<String>,
    expires_at: Option<Instant>,
}

impl TokenManager {
    pub fn new(client: Client, config: TwitchConfig) -> Self {
        Self {
            client,
            config,
            token_data: Arc::new(RwLock::new(TokenData::default())),
        }
    }

    /// Get a valid access token, refreshing if necessary
    pub async fn get_token(&self) -> Result<String, TwitchError> {
        // Check if we have a valid token
        {
            let token_data = self.token_data.read().await;
            if let Some(ref token) = token_data.access_token {
                if let Some(expires_at) = token_data.expires_at {
                    if expires_at > Instant::now() {
                        debug!("Using cached Twitch access token");
                        return Ok(token.clone());
                    }
                }
            }
        }

        // Need to refresh the token
        self.refresh_token().await
    }

    /// Refresh the OAuth2 token
    async fn refresh_token(&self) -> Result<String, TwitchError> {
        info!(
            "Refreshing Twitch OAuth2 token with client_id: {}",
            self.config.client_id.chars().take(10).collect::<String>() + "..."
        );

        let response = self
            .client
            .post(TOKEN_URL)
            .form(&[
                ("client_id", &self.config.client_id),
                ("client_secret", &self.config.client_secret),
                ("grant_type", &"client_credentials".to_string()),
            ])
            .send()
            .await
            .map_err(|e| {
                error!("Token request failed with network error: {}", e);
                TwitchError::AuthError(format!("Failed to request token: {}", e))
            })?;

        let status = response.status();
        debug!("Token request status: {}", status);

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            error!("Token request failed: {} - Body: {}", status, body);
            return Err(TwitchError::AuthError(format!(
                "Token request failed with status {}: {}",
                status, body
            )));
        }

        let token_response: TokenResponse = response.json().await.map_err(|e| {
            error!("Failed to parse token response: {}", e);
            TwitchError::ParseError(format!("Failed to parse token response: {}", e))
        })?;

        let access_token = token_response.access_token.clone();
        let expires_in = token_response.expires_in;

        // Store the token with a 60-second buffer before expiration
        let mut token_data = self.token_data.write().await;
        token_data.access_token = Some(access_token.clone());
        token_data.expires_at =
            Some(Instant::now() + Duration::from_secs(expires_in.saturating_sub(60)));

        info!(
            "Twitch OAuth2 token refreshed successfully, expires in {} seconds",
            expires_in
        );
        debug!(
            "New token (first 20 chars): {}...",
            access_token.chars().take(20).collect::<String>()
        );
        Ok(access_token)
    }

    /// Invalidate the current token (forces refresh on next request)
    pub async fn invalidate(&self) {
        let mut token_data = self.token_data.write().await;
        token_data.access_token = None;
        token_data.expires_at = None;
        debug!("Twitch token invalidated");
    }
}
