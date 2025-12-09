//! API response types

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use stream_aggregator_core::{errors::*, models::*};

/// Standard API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub data: T,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

/// Paginated API response
#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationInfo,
}

#[derive(Debug, Serialize)]
pub struct PaginationInfo {
    pub page: usize,
    pub per_page: usize,
    pub total: usize,
    pub total_pages: usize,
}

impl From<StreamPage> for PaginatedResponse<StreamInfo> {
    fn from(page: StreamPage) -> Self {
        Self {
            data: page.items,
            pagination: PaginationInfo {
                page: page.page,
                per_page: page.page_size,
                total: page.total,
                total_pages: page.total_pages,
            },
        }
    }
}

/// Error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
}

impl ErrorResponse {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: ErrorDetail {
                code: code.into(),
                message: message.into(),
            },
        }
    }
}

/// API error wrapper that implements IntoResponse
pub struct ApiErrorResponse(pub ApiError);

impl From<ApiError> for ApiErrorResponse {
    fn from(err: ApiError) -> Self {
        ApiErrorResponse(err)
    }
}

impl From<StoreError> for ApiErrorResponse {
    fn from(err: StoreError) -> Self {
        ApiErrorResponse(ApiError::StoreError(err))
    }
}

/// Convert ApiError to HTTP response
impl IntoResponse for ApiErrorResponse {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self.0 {
            ApiError::StoreError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "STORE_ERROR",
                e.to_string(),
            ),
            ApiError::InvalidRequest(msg) => (StatusCode::BAD_REQUEST, "INVALID_REQUEST", msg.clone()),
            ApiError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "UNAUTHORIZED",
                "Unauthorized".to_string(),
            ),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "FORBIDDEN", "Forbidden".to_string()),
            ApiError::NotFound => (
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                "Resource not found".to_string(),
            ),
            ApiError::RateLimitExceeded => (
                StatusCode::TOO_MANY_REQUESTS,
                "RATE_LIMIT_EXCEEDED",
                "Rate limit exceeded".to_string(),
            ),
            ApiError::InternalError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                msg.clone(),
            ),
        };

        let body = Json(ErrorResponse::new(code, message));
        (status, body).into_response()
    }
}

/// Health check response
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    pub status: String,
    pub version: String,
}
