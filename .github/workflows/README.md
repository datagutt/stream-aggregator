# GitHub Actions Workflows

Optimized CI/CD workflows for maximum performance without additional costs.

## Workflows Overview

### 1. CI (`ci.yml`)
Fast validation with heavy caching and path filtering.

**Triggers:**
- Push/PR to main/develop (only when Rust files change)

**Jobs:**
- **check** (10 min): Format and clippy checks
- **test** (30 min): Build and run tests
- **audit** (10 min): Security audit (non-blocking)

**Optimizations:**
- Path filtering (skips doc-only changes)
- Concurrency control (cancels old runs)
- Heavy Cargo caching
- Job parallelization

### 2. Docker Build (`docker-publish.yml`)
Multi-platform Docker builds with advanced caching.

**Triggers:**
- Push to main/develop (only when Rust files change)
- Version tags (v1.0.0)
- Pull requests (build only, no push)
- Manual workflow dispatch

**Key Features:**
- Multi-platform builds (linux/amd64, linux/arm64)
- Two-stage dependency compilation (massive cache wins)
- Multi-layer caching (GHA + registry fallback)
- Path filtering (only builds when needed)
- Concurrency control (one build per branch)
- Shallow git clone

**Build Time:**
- First build: ~20 minutes (no cache)
- Cached build (deps unchanged): ~3 minutes
- Code-only change: ~5 minutes
- Doc-only change: **Skipped entirely**

**Image Tags Generated:**

| Git Event | Tags Created |
|-----------|--------------|
| `main` branch push | `latest`, `main`, `main-abc123` |
| `develop` branch push | `develop`, `develop-abc123` |
| Tag `v1.2.3` | `v1.2.3`, `v1.2`, `v1`, `latest` |
| Pull request | No push (build only) |

### 3. Merge Queue (`merge-queue.yml`)
Quick validation before merging to main.

**Triggers:** Merge queue validation

**Optimizations:**
- Fast validation (format, lint, test)
- Shared cache with CI workflow
- Prevents broken code from reaching main

### 4. Cleanup Caches (`cleanup-caches.yml`)
Automatic cache management to prevent hitting GitHub's 10GB limit.

**Schedule:** Weekly on Sundays

### 5. Dependabot (`dependabot.yml`)
Automatic dependency updates with batching.

**Schedule:**
- Rust dependencies: Weekly (Monday)
- GitHub Actions: Monthly
- Docker base images: Weekly

## Performance Optimizations

### Path Filtering
Only runs when these files change:
- `crates/**/*.rs` - Rust source
- `Cargo.toml`, `Cargo.lock` - Dependencies
- `Dockerfile` - Build config
- `.github/workflows/*.yml` - Workflows

**Skipped:** `*.md`, `docs/`, etc.

### Concurrency Control
```yaml
concurrency:
  group: <workflow>-${{ github.ref }}
  cancel-in-progress: true
```
Automatically cancels old runs when new commits arrive.

### Docker Layer Caching Strategy
1. **Planner stage**: Compiles dependencies only (heavily cached)
2. **Builder stage**: Compiles application code (fast rebuilds)

Cache sources (in priority order):
1. Branch-specific cache (`buildkit-<branch>`)
2. Main branch cache fallback
3. Registry cache (cross-runner reuse)

### Cargo Caching
Separate caches for:
1. Registry index (package metadata)
2. Registry cache (downloaded crates)
3. Git dependencies
4. Build artifacts (`target/`)

## Images Published To

```
ghcr.io/OWNER/REPO:TAG
```

Example:
```
ghcr.io/datagutt/stream-aggregator:latest
```

## Local Testing

### Run CI checks locally
```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

### Test Docker build with cache
```bash
docker buildx build \
  --cache-from type=gha \
  --cache-to type=gha,mode=max \
  -t stream-aggregator .
```

### Test workflows with act
```bash
# Install act
brew install act  # macOS

# Run CI workflow
act push -j check

# Run Docker workflow (requires Docker)
act push -j build-and-push
```

## Cache Management

### Monitor usage
```bash
# List all caches
gh actions-cache list

# Check cache size
gh actions-cache list --json | jq '[.[] | .size_in_bytes] | add'
```

### Manual cleanup
```bash
# Delete specific cache
gh actions-cache delete <cache-key>

# Delete all caches for a branch
gh actions-cache list -B <branch> | cut -f 1 | \
  xargs -I {} gh actions-cache delete {}
```

**Limits:**
- GitHub free tier: 10GB total
- Retention: 7 days (auto-cleanup)

## Troubleshooting

### Build fails with "no space left on device"
Cache cleanup runs weekly. For immediate fix:
```bash
gh workflow run cleanup-caches.yml
```

### "cmake not installed" error
Already fixed in Dockerfile. Ensure you're using latest image.

### Multi-platform build timeout
Increase timeout in workflow:
```yaml
timeout-minutes: 60
```

### Authentication errors
Ensure workflow permissions:
1. Repository Settings → Actions → General
2. Workflow permissions → Read and write
3. Save

### Low cache hit rate
Check if `Cargo.lock` is committed (it should be).

## Cost Optimization

### Free Tier Limits
- **Public repos**: Unlimited minutes
- **Private repos**: 2,000 minutes/month

### Current Usage (per push)
- CI (Rust changes): ~5-10 min
- Docker build (cached): ~5 min
- Docker build (uncached): ~20 min
- Doc-only changes: **0 min** (skipped)

### Strategies Used
1. Path filtering → Skip unnecessary builds
2. Concurrency control → Cancel duplicate runs
3. Heavy caching → Minimize rebuild time
4. Parallel jobs → Reduce wall-clock time
5. Timeout limits → Prevent runaway jobs
6. Shallow clones → Faster checkouts
7. Batched dependency updates → Fewer CI runs

## Monitoring

### Check workflow performance
```bash
# List recent runs
gh run list --workflow=ci.yml

# View run details
gh run view <run-id>

# Download logs
gh run download <run-id>
```

### Verify cache effectiveness
Check logs for:
```
Cache restored from key: cargo-deps-<hash>
```
Good hit rate: >80%

## Maintenance

### Weekly
- Review failed runs
- Merge Dependabot PRs
- Check cache usage

### Monthly
- Update GitHub Actions versions
- Review workflow performance
- Optimize slow jobs

## References

- [GitHub Actions Caching](https://docs.github.com/en/actions/using-workflows/caching-dependencies-to-speed-up-workflows)
- [Docker Buildx Caching](https://docs.docker.com/build/cache/)
- [Cargo Build Performance](https://doc.rust-lang.org/cargo/reference/profiles.html)
