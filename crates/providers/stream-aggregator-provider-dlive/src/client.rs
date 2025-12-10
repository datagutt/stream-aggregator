//! DLive provider client implementation

use async_trait::async_trait;
use chrono::Utc;
use tracing::{debug, error};
use wreq::Client;

use stream_aggregator_core::{
    errors::ProviderError,
    models::*,
    traits::PlatformProvider,
};

use crate::models::{DLiveConfig, GraphQLRequest, GraphQLResponse};

const DLIVE_GRAPHQL_ENDPOINT: &str = "https://graphigo.prd.dlive.tv";

/// DLive platform provider using GraphQL
pub struct DLiveProvider {
    client: Client,
}

impl DLiveProvider {
    /// Create a new DLive provider
    pub fn new(_config: DLiveConfig) -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client }
    }

    /// Execute a GraphQL query
    async fn execute_query(&self, query: &str) -> Result<GraphQLResponse, ProviderError> {
        let request = GraphQLRequest {
            query: query.to_string(),
            variables: None,
        };

        let response = self.client
            .post(DLIVE_GRAPHQL_ENDPOINT)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| ProviderError::HttpError(format!("Failed to query DLive GraphQL: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            error!("DLive GraphQL error {}: {}", status, body);
            return Err(ProviderError::HttpError(format!("DLive GraphQL error {}", status)));
        }

        let gql_response: GraphQLResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::ParseError(format!("Failed to parse DLive response: {}", e)))?;

        // Check for GraphQL errors
        if !gql_response.errors.is_empty() {
            let error_msg = gql_response.errors
                .iter()
                .map(|e| e.message.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(ProviderError::ParseError(format!("DLive GraphQL errors: {}", error_msg)));
        }

        Ok(gql_response)
    }

    /// Fetch user by display name
    async fn fetch_user(&self, display_name: &str) -> Result<GraphQLResponse, ProviderError> {
        debug!("Fetching DLive user: {}", display_name);

        let query = format!(
            r#"{{
                userByDisplayName(displayname: "{}") {{
                    username
                    displayname
                    avatar
                    livestream {{
                        id
                        title
                        totalReward
                        watchingCount
                        language {{
                            language
                        }}
                        category {{
                            title
                        }}
                    }}
                }}
            }}"#,
            display_name
        );

        self.execute_query(&query).await
    }
}

#[async_trait]
impl PlatformProvider for DLiveProvider {
    fn platform_id(&self) -> &'static str {
        "dlive"
    }

    fn display_name(&self) -> &'static str {
        "DLive"
    }

    fn base_url(&self) -> &'static str {
        "https://dlive.tv"
    }

    async fn resolve_user_id(&self, username_or_id: &str) -> Result<String, ProviderError> {
        // DLive uses display names
        Ok(username_or_id.to_string())
    }

    async fn fetch_stream(&self, user_id: &str) -> Result<StreamInfo, ProviderError> {
        let response = self.fetch_user(user_id).await?;

        let user = response
            .data
            .and_then(|d| d.user_by_display_name)
            .ok_or_else(|| ProviderError::StreamerNotFound(user_id.to_string()))?;

        let mut stream_info = StreamInfo::new("dlive", &user.username, &user.display_name);
        stream_info.avatar_url = Some(user.avatar);

        if let Some(livestream) = user.livestream {
            stream_info.is_live = true;
            stream_info.title = Some(livestream.title);
            stream_info.viewer_count = Some(livestream.watching_count);

            if let Some(language) = livestream.language {
                stream_info.language = Some(language.language);
            }

            if let Some(category) = livestream.category {
                stream_info.category = Some(category.title);
            }
        } else {
            stream_info.is_live = false;
        }

        stream_info.last_updated = Utc::now();

        Ok(stream_info)
    }

    async fn fetch_streams_batch(&self, user_ids: &[String]) -> Vec<Result<StreamInfo, ProviderError>> {
        // No batch API, fetch sequentially
        let mut results = Vec::with_capacity(user_ids.len());
        
        for user_id in user_ids {
            results.push(self.fetch_stream(user_id).await);
        }
        
        results
    }

    fn supports_discovery(&self) -> bool {
        false
    }

    async fn discover_streamers(
        &self,
        _filters: &DiscoveryFilters,
    ) -> Result<Vec<DiscoveredStreamer>, ProviderError> {
        Err(ProviderError::DiscoveryNotSupported)
    }

    fn rate_limit_config(&self) -> RateLimitConfig {
        RateLimitConfig {
            requests_per_minute: 60,
            burst_size: 10,
        }
    }

    async fn health_check(&self) -> HealthStatus {
        match self.client.get("https://dlive.tv").send().await {
            Ok(response) if response.status().is_success() => HealthStatus::Healthy,
            _ => HealthStatus::Unhealthy,
        }
    }
}
