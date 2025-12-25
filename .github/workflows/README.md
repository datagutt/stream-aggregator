# GitHub Actions Workflows

## docker-publish.yml

Automatically builds and publishes Docker images to GitHub Container Registry (GHCR).

### Triggers

- **Push to main**: Builds and tags as `latest`, `main`, and `main-{sha}`
- **Push to develop**: Builds and tags as `develop`, `develop-{sha}`
- **Version tags (v1.0.0)**: Builds and tags as `v1.0.0`, `v1.0`, `v1`, `latest`
- **Pull requests**: Builds but doesn't push (validation only)
- **Manual**: Can be triggered manually from Actions tab

### Features

- Multi-platform builds (linux/amd64, linux/arm64)
- Build caching for faster builds
- Semantic versioning support
- Provenance attestations for security
- Automatic tagging strategy

### Image Tags Generated

| Git Event | Tags Created |
|-----------|--------------|
| `main` branch push | `latest`, `main`, `main-abc123` |
| `develop` branch push | `develop`, `develop-abc123` |
| Tag `v1.2.3` | `v1.2.3`, `v1.2`, `v1`, `latest` |
| Pull request | No push (build only) |

### Permissions

The workflow uses GITHUB_TOKEN which is automatically provided by GitHub.
No additional secrets needed.

### Build Time

- First build: ~15-20 minutes (no cache)
- Subsequent builds: ~5-10 minutes (with cache)
- Multi-platform adds ~2-3 minutes overhead

### Output

Images are published to:
```
ghcr.io/OWNER/REPO:TAG
```

Example:
```
ghcr.io/datagutt/stream-aggregator:latest
```

### Local Testing

To test the workflow locally:

```bash
# Install act (GitHub Actions local runner)
brew install act  # macOS
# or
curl https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash

# Run workflow locally
act push -j build-and-push
```

### Troubleshooting

#### Build fails with "no space left on device"

GitHub Actions runners have limited disk space. The workflow uses cache cleanup.

#### Multi-platform build timeout

Reduce platforms or increase timeout:
```yaml
timeout-minutes: 60
```

#### Authentication errors

Ensure repository has package write permissions:
1. Go to repository Settings
2. Actions → General
3. Workflow permissions
4. Select "Read and write permissions"

### Maintenance

The workflow uses versioned actions (e.g., `@v4`). Update periodically:

```bash
# Check for updates
gh workflow list
```

### Cost

GitHub Actions is free for public repositories with generous limits:
- 2000 minutes/month for free accounts
- Unlimited for public repos

Multi-platform builds use ~15-20 minutes per build.
