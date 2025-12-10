use anyhow::Result;
use chrono::Utc;
use clap::Parser;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use std::collections::HashMap;
use std::fs;

#[derive(Parser)]
#[command(name = "stream-aggregator-migrator")]
#[command(about = "Migrate old people.json database to new SQLite schema")]
struct Args {
    /// Path to the old people.json file
    #[arg(short, long, default_value = "lsnd/people.json")]
    input: String,

    /// SQLite database URL
    #[arg(short, long, default_value = "sqlite:stream_aggregator.db")]
    database_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OldStreamer {
    platform: String,
    userId: String,
    #[serde(default)]
    featuredRank: Option<String>,
    #[serde(default)]
    team: Option<String>,
    #[serde(default)]
    customUsername: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    println!("Reading old database from: {}", args.input);
    let json_content = fs::read_to_string(&args.input)?;
    let old_streamers: Vec<OldStreamer> = serde_json::from_str(&json_content)?;

    println!("Found {} streamers to migrate", old_streamers.len());

    println!("Connecting to database: {}", args.database_url);
    let pool = SqlitePool::connect(&args.database_url).await?;

    // Run migrations to ensure schema exists
    println!("Running migrations...");
    sqlx::migrate!("../stream-aggregator-store/migrations")
        .run(&pool)
        .await?;

    println!("Migrating streamers...");
    let mut migrated = 0;
    let mut skipped = 0;
    let total_count = old_streamers.len();

    for old_streamer in old_streamers {
        // Check if streamer already exists
        let exists: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM tracked_streamers WHERE platform = ? AND user_id = ?"
        )
        .bind(&old_streamer.platform)
        .bind(&old_streamer.userId)
        .fetch_optional(&pool)
        .await?;

        if exists.is_some() {
            println!("Skipping existing streamer: {} on {}", old_streamer.userId, old_streamer.platform);
            skipped += 1;
            continue;
        }

        // Parse priority from featuredRank
        let priority = if let Some(rank) = &old_streamer.featuredRank {
            match rank.parse::<i32>() {
                Ok(p) => Some(p),
                Err(_) => {
                    // Try to parse as float and convert to int
                    if let Ok(p) = rank.parse::<f32>() {
                        Some(p as i32)
                    } else {
                        None
                    }
                }
            }
        } else {
            None
        };

        // Create labels map
        let mut labels = HashMap::new();
        if let Some(rank) = &old_streamer.featuredRank {
            labels.insert("featured_rank".to_string(), rank.clone());
        }

        let labels_json = serde_json::to_string(&labels)?;

        // Insert the streamer
        sqlx::query(
            r#"
            INSERT INTO tracked_streamers (
                platform, user_id, custom_name, group_name, priority,
                labels, source, discovery_rule_id, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, 'manual', NULL, ?)
            "#
        )
        .bind(&old_streamer.platform)
        .bind(&old_streamer.userId)
        .bind(&old_streamer.customUsername)
        .bind(&old_streamer.team)
        .bind(&priority)
        .bind(&labels_json)
        .bind(Utc::now().to_rfc3339())
        .execute(&pool)
        .await?;

        println!("Migrated: {} on {}", old_streamer.userId, old_streamer.platform);
        migrated += 1;
    }

    println!("\nMigration complete!");
    println!("Migrated: {}", migrated);
    println!("Skipped (already exist): {}", skipped);
    println!("Total processed: {}", total_count);

    Ok(())
}