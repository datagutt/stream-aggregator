# StreamAggregator - Project Overview

## Purpose
Multi-platform live stream aggregator service that fetches and tracks live stream information from multiple streaming platforms (Twitch, YouTube, Kick, TikTok, etc.).

## Tech Stack
- **Language**: Rust (edition 2021, rust-version 1.75+)
- **Async Runtime**: Tokio
- **Web Framework**: Axum with Tower middleware
- **HTTP Client**: wreq (reqwest fork with TLS fingerprinting for anti-bot bypass)
- **Storage**: DashMap (in-memory), with plans for SQLite/PostgreSQL
- **Serialization**: Serde (JSON, TOML)
- **CLI**: Clap with derive macros
- **Logging**: tracing + tracing-subscriber

## Architecture
Modular workspace with 6+ crates:
- `stream-aggregator` - Main binary with provider registry
- `stream-aggregator-core` - Core types and traits
- `stream-aggregator-api` - REST API (axum)
- `stream-aggregator-store` - Storage abstraction
- `stream-aggregator-scheduler` - Scraping scheduler (TO BE IMPLEMENTED)
- `stream-aggregator-provider-*` - Platform providers (Twitch implemented)

## Key Features
- Provider registry pattern for scalable provider management
- API key authentication with public read access
- Trait-based platform provider system
- OAuth2 token management for Twitch
- Batch fetching and discovery support
