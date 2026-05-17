//! Integration tests for the /api/v1/communities endpoints.

use std::collections::HashMap;
use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

use stream_aggregator_api::{create_router_with_auth, AuthConfig};
use stream_aggregator_core::traits::StreamStore;
use stream_aggregator_store::MemoryStore;

fn router(auth: AuthConfig) -> axum::Router {
    let store: Arc<dyn StreamStore> = Arc::new(MemoryStore::new());
    create_router_with_auth(store, HashMap::new(), auth)
}

async fn body_json(body: Body) -> Value {
    let bytes = to_bytes(body, usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn payload(slug: &str, domains: &[&str]) -> Value {
    json!({
        "slug": slug,
        "name": format!("Community {slug}"),
        "accent": "0.58 0.20 250",
        "default_theme": "dark",
        "domains": domains,
        "filter": { "languages": ["no"] }
    })
}

#[tokio::test]
async fn list_get_create_update_delete_unauthenticated() {
    let app = router(AuthConfig::default());

    // List on empty store
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/communities")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res.into_body()).await;
    assert_eq!(v["data"].as_array().unwrap().len(), 0);

    // Create
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/communities")
                .header("content-type", "application/json")
                .body(Body::from(payload("lsn", &["lsn.local"]).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let v = body_json(res.into_body()).await;
    assert_eq!(v["data"]["slug"], "lsn");
    assert_eq!(v["data"]["domains"][0], "lsn.local");
    // Response shape: snake_case throughout (existing API convention).
    assert_eq!(v["data"]["default_theme"], "dark");
    assert!(v["data"]["created_at"].is_string());

    // Conflict: same slug twice
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/communities")
                .header("content-type", "application/json")
                .body(Body::from(payload("lsn", &[]).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Get by slug
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/communities/lsn")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Get by domain
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/communities/by-domain/lsn.local")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res.into_body()).await;
    assert_eq!(v["data"]["slug"], "lsn");

    // Unknown domain -> 404
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/communities/by-domain/missing.test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // Update via PUT (replace domains)
    let mut payload_v = payload("lsn", &["lsn.local", "extra.test"]);
    payload_v["tagline"] = json!("Norske strømmere live nå");
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri("/api/v1/communities/lsn")
                .header("content-type", "application/json")
                .body(Body::from(payload_v.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res.into_body()).await;
    assert_eq!(v["data"]["tagline"], "Norske strømmere live nå");
    assert_eq!(v["data"]["domains"].as_array().unwrap().len(), 2);

    // Delete
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/api/v1/communities/lsn")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // Second delete -> 404
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/api/v1/communities/lsn")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn writes_require_api_key_when_auth_enabled() {
    let app = router(AuthConfig::new(vec!["secret".into()]));

    // GET stays public
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/communities")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // POST without key -> 401
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/communities")
                .header("content-type", "application/json")
                .body(Body::from(payload("lsn", &[]).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // POST with key -> 201
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/communities")
                .header("content-type", "application/json")
                .header("X-API-Key", "secret")
                .body(Body::from(payload("lsn", &[]).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
}
