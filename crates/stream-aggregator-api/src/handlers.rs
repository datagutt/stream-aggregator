//! HTTP request handlers

use axum::{
    async_trait,
    extract::{FromRequestParts, Path, State},
    http::{request::Parts, StatusCode},
    Json,
};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;

use stream_aggregator_core::{errors::*, models::*, traits::{StreamStore, PlatformProvider}};

use crate::responses::*;

// Type alias for API handlers result
type ApiResult<T> = Result<T, ApiErrorResponse>;

/// Generic query string extractor that supports bracket notation
/// 
/// This extractor uses `serde_qs` to properly parse query strings with
/// bracket notation like `?labels[key]=value` into nested structures.
pub struct QsQuery<T>(pub T);

#[async_trait]
impl<T, S> FromRequestParts<S> for QsQuery<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let query = parts.uri.query().unwrap_or_default();
        let value = serde_qs::from_str(query)
            .map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("Failed to parse query string: {}", e),
                )
            })?;
        Ok(QsQuery(value))
    }
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub store: Arc<dyn StreamStore>,
    /// Provider registry for resolving usernames to IDs
    pub providers: Arc<HashMap<String, Arc<dyn PlatformProvider>>>,
}

/// Query parameters for GET /streams
#[derive(Debug, Deserialize)]
pub struct StreamsQuery {
    pub platform: Option<String>,
    #[serde(rename = "live")]
    pub is_live: Option<bool>,
    pub group: Option<String>,
    #[serde(default)]
    pub labels: HashMap<String, String>,
    pub search: Option<String>,
    pub language: Option<String>,
    pub category: Option<String>,
    pub tag: Option<String>,
    pub min_viewers: Option<u64>,
    pub max_viewers: Option<u64>,
    pub sort: Option<String>,
    pub order: Option<String>,
    pub page: Option<usize>,
    #[serde(rename = "per_page")]
    pub page_size: Option<usize>,
}

/// GET /streams - List all streams
pub async fn list_streams(
    State(state): State<AppState>,
    QsQuery(query): QsQuery<StreamsQuery>,
) -> ApiResult<Json<PaginatedResponse<StreamInfo>>> {
    debug!(?query, "Listing streams");

    let stream_query = StreamQuery {
        platform: query.platform,
        is_live: query.is_live,
        group: query.group,
        labels: query.labels,
        search: query.search,
        language: query.language,
        category: query.category,
        tag: query.tag,
        min_viewers: query.min_viewers,
        max_viewers: query.max_viewers,
        sort: query.sort,
        order: query.order,
        page: query.page,
        page_size: query.page_size,
    };

    let page = state.store.get_streams(&stream_query).await?;
    Ok(Json(page.into()))
}

/// GET /streams/:id - Get a single stream
pub async fn get_stream(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiResponse<StreamInfo>>> {
    debug!(stream_id = %id, "Getting stream");

    let stream_id = StreamId(id);
    let stream = state
        .store
        .get_stream(&stream_id)
        .await?
        .ok_or(ApiErrorResponse(ApiError::NotFound))?;

    Ok(Json(ApiResponse::new(stream)))
}

/// Query parameters for GET /streamers
#[derive(Debug, Deserialize)]
pub struct StreamersQuery {
    pub platform: Option<String>,
    pub group: Option<String>,
    pub source: Option<StreamerSource>,
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

/// GET /streamers - List tracked streamers
pub async fn list_streamers(
    State(state): State<AppState>,
    QsQuery(query): QsQuery<StreamersQuery>,
) -> ApiResult<Json<ApiResponse<Vec<TrackedStreamer>>>> {
    debug!(?query, "Listing tracked streamers");

    let streamer_query = TrackedStreamerQuery {
        platform: query.platform,
        group: query.group,
        source: query.source,
        labels: query.labels,
    };

    let streamers = state.store.get_tracked_streamers(&streamer_query).await?;
    Ok(Json(ApiResponse::new(streamers)))
}

/// POST /streamers - Add a streamer to track
#[derive(Debug, Deserialize)]
pub struct AddStreamerRequest {
    pub platform: String,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub custom_name: Option<String>,
    pub group: Option<String>,
    pub priority: Option<i32>,
    pub labels: Option<HashMap<String, String>>,
}

pub async fn add_streamer(
    State(state): State<AppState>,
    Json(req): Json<AddStreamerRequest>,
) -> ApiResult<Json<ApiResponse<TrackedStreamer>>> {
    match (&req.user_id, &req.username) {
        (None, None) => {
            return Err(ApiErrorResponse(ApiError::BadRequest(
                "Must provide either 'user_id' or 'username'".to_string()
            )));
        }
        (Some(_), Some(_)) => {
            return Err(ApiErrorResponse(ApiError::BadRequest(
                "Cannot provide both 'user_id' and 'username'".to_string()
            )));
        }
        _ => {}
    }
    
    let provider = state.providers.get(&req.platform).ok_or_else(|| {
        ApiErrorResponse(ApiError::BadRequest(format!(
            "Unsupported platform: {}",
            req.platform
        )))
    })?;

    let final_user_id = if let Some(username) = req.username {
        provider.resolve_user_id(&username).await?
    } else {
        req.user_id.unwrap()
    };

    let mut streamer = TrackedStreamer::new_manual(req.platform.clone(), final_user_id.clone());
    streamer.custom_name = req.custom_name;
    streamer.group = req.group;
    streamer.priority = req.priority;
    if let Some(labels) = req.labels {
        streamer.labels = labels;
    }

    state.store.add_tracked_streamer(&streamer).await?;

    // Perform initial data scrape
    match provider.fetch_stream(&final_user_id).await {
        Ok(stream_info) => {
            // Store the initial stream data
            if let Err(e) = state.store.upsert_stream(&stream_info).await {
                tracing::warn!(platform = %req.platform, user_id = %final_user_id, "Failed to store initial stream data: {}", e);
                // Don't fail the request if storing fails, just log
            }
        }
        Err(e) => {
            // Scrape failed, remove the streamer
            tracing::warn!(platform = %req.platform, user_id = %final_user_id, "Initial scrape failed, removing streamer: {}", e);
            if let Err(remove_err) = state.store.remove_tracked_streamer(&req.platform, &final_user_id).await {
                tracing::error!(platform = %req.platform, user_id = %final_user_id, "Failed to remove streamer after scrape failure: {}", remove_err);
            }
            return Err(ApiErrorResponse(ApiError::BadRequest(format!(
                "Failed to fetch initial stream data: {}", e
            ))));
        }
    }

    Ok(Json(ApiResponse::new(streamer)))
}

/// DELETE /streamers/:platform/:user_id - Remove a tracked streamer
pub async fn remove_streamer(
    State(state): State<AppState>,
    Path((platform, user_id)): Path<(String, String)>,
) -> ApiResult<Json<ApiResponse<()>>> {
    debug!(platform = %platform, user_id = %user_id, "Removing streamer");

    state.store.remove_tracked_streamer(&platform, &user_id).await?;
    Ok(Json(ApiResponse::new(())))
}

/// GET /health - Health check
pub async fn health_check() -> Json<HealthCheckResponse> {
    Json(HealthCheckResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// GET /platforms - List supported platforms
pub async fn list_platforms(
    State(state): State<AppState>,
) -> Json<ApiResponse<Vec<PlatformInfo>>> {
    let platforms = state
        .providers
        .values()
        .map(|provider| PlatformInfo {
            id: provider.platform_id().to_string(),
            name: provider.display_name().to_string(),
            base_url: provider.base_url().to_string(),
            supports_discovery: provider.supports_discovery(),
        })
        .collect();

    Json(ApiResponse::new(platforms))
}

#[derive(Debug, serde::Serialize)]
pub struct PlatformInfo {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub supports_discovery: bool,
}
