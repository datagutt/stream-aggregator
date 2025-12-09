//! HTTP request handlers

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error};

use stream_aggregator_core::{errors::*, models::*, traits::StreamStore};

use crate::responses::*;

// Type alias for API handlers result
type ApiResult<T> = Result<T, ApiErrorResponse>;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub store: Arc<dyn StreamStore>,
}

/// Query parameters for GET /streams
#[derive(Debug, Deserialize)]
pub struct StreamsQuery {
    pub platform: Option<String>,
    #[serde(rename = "live")]
    pub is_live: Option<bool>,
    pub language: Option<String>,
    pub category: Option<String>,
    pub tag: Option<String>,
    pub min_viewers: Option<u64>,
    pub page: Option<usize>,
    #[serde(rename = "per_page")]
    pub page_size: Option<usize>,
}

/// GET /streams - List all streams
pub async fn list_streams(
    State(state): State<AppState>,
    Query(query): Query<StreamsQuery>,
) -> ApiResult<Json<PaginatedResponse<StreamInfo>>> {
    debug!(?query, "Listing streams");

    let stream_query = StreamQuery {
        platform: query.platform,
        is_live: query.is_live,
        language: query.language,
        category: query.category,
        tag: query.tag,
        min_viewers: query.min_viewers,
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
}

/// GET /streamers - List tracked streamers
pub async fn list_streamers(
    State(state): State<AppState>,
    Query(query): Query<StreamersQuery>,
) -> ApiResult<Json<ApiResponse<Vec<TrackedStreamer>>>> {
    debug!(?query, "Listing tracked streamers");

    let streamer_query = TrackedStreamerQuery {
        platform: query.platform,
        group: query.group,
        source: query.source,
        labels: HashMap::new(),
    };

    let streamers = state.store.get_tracked_streamers(&streamer_query).await?;
    Ok(Json(ApiResponse::new(streamers)))
}

/// POST /streamers - Add a streamer to track
#[derive(Debug, Deserialize)]
pub struct AddStreamerRequest {
    pub platform: String,
    pub user_id: String,
    pub custom_name: Option<String>,
    pub group: Option<String>,
    pub priority: Option<i32>,
    pub labels: Option<HashMap<String, String>>,
}

pub async fn add_streamer(
    State(state): State<AppState>,
    Json(req): Json<AddStreamerRequest>,
) -> ApiResult<Json<ApiResponse<TrackedStreamer>>> {
    debug!(platform = %req.platform, user_id = %req.user_id, "Adding streamer");

    let mut streamer = TrackedStreamer::new_manual(req.platform, req.user_id);
    streamer.custom_name = req.custom_name;
    streamer.group = req.group;
    streamer.priority = req.priority;
    if let Some(labels) = req.labels {
        streamer.labels = labels;
    }

    state.store.add_tracked_streamer(&streamer).await?;
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
pub async fn list_platforms() -> Json<ApiResponse<Vec<PlatformInfo>>> {
    // TODO: Get this from registered providers
    let platforms = vec![
        PlatformInfo {
            id: "twitch".to_string(),
            name: "Twitch".to_string(),
            base_url: "https://twitch.tv".to_string(),
            supports_discovery: true,
        },
    ];

    Json(ApiResponse::new(platforms))
}

#[derive(Debug, serde::Serialize)]
pub struct PlatformInfo {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub supports_discovery: bool,
}
