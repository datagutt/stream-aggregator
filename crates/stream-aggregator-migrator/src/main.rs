use anyhow::Result;
use chrono::Utc;
use clap::Parser;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use std::collections::{HashMap, HashSet};
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

    /// Dry run mode - show what would be migrated without making changes
    #[arg(long)]
    dry_run: bool,

    /// Show detailed statistics about the migration
    #[arg(long)]
    stats: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OldStreamer {
    platform: String,
    #[serde(rename = "userId")]
    user_id: String,
    #[serde(rename = "featuredRank", default)]
    featured_rank: Option<String>,
    #[serde(default)]
    team: Option<String>,
    #[serde(rename = "customUsername", default)]
    custom_username: Option<String>,
}

#[derive(Default)]
struct MigrationStats {
    total_entries: usize,
    unique_streamers: usize,
    duplicates_in_source: usize,
    migrated: usize,
    skipped_existing: usize,
    platforms: HashMap<String, usize>,
    teams: HashMap<String, usize>,
    featured_count: usize,
    with_custom_name: usize,
}

impl MigrationStats {
    fn print(&self) {
        println!("\n--- Migration Statistics ---");
        println!("Source file analysis:");
        println!("  Total entries: {}", self.total_entries);
        println!("  Unique streamers: {}", self.unique_streamers);
        println!(
            "  Duplicate entries (skipped): {}",
            self.duplicates_in_source
        );
        println!("  With featured rank: {}", self.featured_count);
        println!("  With custom name: {}", self.with_custom_name);

        println!("\nPlatform distribution:");
        let mut platforms: Vec<_> = self.platforms.iter().collect();
        platforms.sort_by(|a, b| b.1.cmp(a.1));
        for (platform, count) in platforms {
            println!("  {}: {}", platform, count);
        }

        println!("\nTeam distribution:");
        let mut teams: Vec<_> = self.teams.iter().collect();
        teams.sort_by(|a, b| b.1.cmp(a.1));
        for (team, count) in &teams {
            println!("  {}: {}", team, count);
        }
        let with_team: usize = teams.iter().map(|(_, c)| *c).sum();
        println!(
            "  (no team): {}",
            self.unique_streamers.saturating_sub(with_team)
        );

        println!("\nMigration results:");
        println!("  Migrated: {}", self.migrated);
        println!("  Skipped (already in DB): {}", self.skipped_existing);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    if args.dry_run {
        println!("--- DRY RUN MODE ---");
        println!("No changes will be made to the database.\n");
    }

    println!("Reading old database from: {}", args.input);
    let json_content = fs::read_to_string(&args.input)?;
    let old_streamers: Vec<OldStreamer> = serde_json::from_str(&json_content)?;

    println!("Found {} entries in source file", old_streamers.len());

    // Deduplicate streamers and collect stats
    let mut stats = MigrationStats {
        total_entries: old_streamers.len(),
        ..Default::default()
    };

    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut unique_streamers: Vec<OldStreamer> = Vec::new();

    for streamer in old_streamers {
        let key = (
            streamer.platform.to_lowercase(),
            streamer.user_id.to_lowercase(),
        );

        // Track platform stats (before dedup)
        *stats.platforms.entry(streamer.platform.clone()).or_insert(0) += 1;

        if seen.contains(&key) {
            stats.duplicates_in_source += 1;
            println!(
                "Warning: Duplicate entry found: {} on {} (skipping)",
                streamer.user_id, streamer.platform
            );
            continue;
        }

        seen.insert(key);

        // Track other stats
        if streamer.featured_rank.is_some() {
            stats.featured_count += 1;
        }
        if streamer.custom_username.is_some() {
            stats.with_custom_name += 1;
        }
        if let Some(ref team) = streamer.team {
            *stats.teams.entry(team.clone()).or_insert(0) += 1;
        }

        unique_streamers.push(streamer);
    }

    stats.unique_streamers = unique_streamers.len();
    println!(
        "After deduplication: {} unique streamers",
        unique_streamers.len()
    );

    if args.dry_run {
        stats.migrated = unique_streamers.len();
        stats.print();
        println!("\n--- DRY RUN COMPLETE ---");
        println!("Run without --dry-run to perform the actual migration.");
        return Ok(());
    }

    println!("Connecting to database: {}", args.database_url);
    let pool = SqlitePool::connect(&args.database_url).await?;

    // Run migrations to ensure schema exists
    println!("Running migrations...");
    sqlx::migrate!("../stream-aggregator-store/migrations")
        .run(&pool)
        .await?;

    println!("Migrating streamers...");

    for old_streamer in unique_streamers {
        // Check if streamer already exists
        let exists: Option<(i32,)> =
            sqlx::query_as("SELECT 1 FROM tracked_streamers WHERE platform = ? AND user_id = ?")
                .bind(&old_streamer.platform)
                .bind(&old_streamer.user_id)
                .fetch_optional(&pool)
                .await?;

        if exists.is_some() {
            println!(
                "Skipping existing streamer: {} on {}",
                old_streamer.user_id, old_streamer.platform
            );
            stats.skipped_existing += 1;
            continue;
        }

        // Parse priority from featured_rank
        let priority = if let Some(rank) = &old_streamer.featured_rank {
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

        // Create labels map with all available metadata
        let mut labels: HashMap<String, serde_json::Value> = HashMap::new();
        if let Some(rank) = &old_streamer.featured_rank {
            labels.insert("featured_rank".to_string(), serde_json::json!(rank));
        }
        if let Some(team) = &old_streamer.team {
            // Also store team in labels for richer querying
            labels.insert("team".to_string(), serde_json::json!(team));
        }
        labels.insert("migrated_from".to_string(), serde_json::json!("lsnd"));

        let labels_json = serde_json::to_string(&labels)?;

        // Insert the streamer
        sqlx::query(
            r#"
            INSERT INTO tracked_streamers (
                platform, user_id, custom_name, group_name, priority,
                labels, source, discovery_rule_id, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, 'migrated', NULL, ?)
            "#,
        )
        .bind(&old_streamer.platform)
        .bind(&old_streamer.user_id)
        .bind(&old_streamer.custom_username)
        .bind(&old_streamer.team)
        .bind(&priority)
        .bind(&labels_json)
        .bind(Utc::now().to_rfc3339())
        .execute(&pool)
        .await?;

        println!(
            "Migrated: {} on {}",
            old_streamer.user_id, old_streamer.platform
        );
        stats.migrated += 1;
    }

    println!("\nMigration complete!");

    if args.stats {
        stats.print();
    } else {
        println!("Migrated: {}", stats.migrated);
        println!("Skipped (already exist): {}", stats.skipped_existing);
        println!("\nRun with --stats for detailed statistics.");
    }

    Ok(())
}
