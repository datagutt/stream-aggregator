# web/

Brandable Next.js frontend for [StreamAggregator](../README.md). One backend, many fronts: each "community" is a curated slice of the global stream pool with its own brand and domains.

See `../PRODUCT.md` and `../DESIGN.md` for the framing and visual system.

## Stack

- Next.js 16 (app router, Turbopack, React 19)
- Tailwind CSS v4 (CSS-first, no plugin config)
- HeroUI v3 (`@heroui/react@^3` + `@heroui/styles@^3`)
- nuqs for URL state (typed search params, throttled, server-aware)
- next-themes for dark/light toggle (class strategy, global cookie)
- culori for color conversion (hex ↔ OKLCH)
- marked for the /about markdown render

Bun is the package manager and dev runner.

## Local development

```bash
# Backend (from the repo root)
API_KEYS=dev-key cargo run --features stream-aggregator-store/all

# Frontend (in this directory)
bun install
bun dev
```

Visit `http://localhost:3000/admin`, paste `dev-key`, and create your first community. Then visit `/c/<slug>` to see the public side.

## Env vars

| Var | Default | Notes |
|---|---|---|
| `STREAM_AGGREGATOR_API_URL` | `http://localhost:8080` | Server-side URL for the Rust API |
| `NEXT_PUBLIC_API_URL` | (unset) | Browser-visible URL; only needed if any client component bypasses server actions |
| `DEFAULT_COMMUNITY` | (unset) | Optional slug — when set, the local-dev `/` picker auto-redirects |

The admin API key is never an env var. It's set per-session in the `admin_key` httpOnly cookie via `/admin`.

## Architecture

```
proxy.ts                 # Host -> /c/[slug] rewrite (Next.js 16 file convention)
app/
  page.tsx               # dev picker + APEX fallback
  _not-configured/       # unknown-host fail-closed page
  c/[slug]/              # public community surface
    page.tsx             # live grid + filter bar (nuqs)
    all/                 # live + offline grouped
    about/               # admin-authored markdown
  admin/                 # API-key gated
    page.tsx             # overview (counts)
    streamers/           # add / list / remove
    communities/         # CRUD (brand + filter recipe + domains)
    platforms/           # read-only
  api/admin-login/       # cookie set/clear handler
lib/
  api.ts                 # typed fetch client
  api-types.ts           # camelCase domain types + snake_case wire types
  communities.ts         # community lookups with 60s in-process cache
  community-search-params.ts # nuqs schema shared between server + client
  auth.ts                # admin cookie helpers
  format.ts              # viewer counts, streamUrl, etc
  color.ts               # hex <-> oklch
components/
  stream-card.tsx        # single <a> per card, plain <img>, brand-aware
  stream-grid.tsx        # SSR initial page + IntersectionObserver infinite scroll + 60s poll on focus
  filter-bar.tsx         # SearchField + TextField + Select + ToggleButton chips, URL-driven
  community-header.tsx
  live-pill.tsx
  platform-pill.tsx
  theme-toggle.tsx
  admin/
    login-form.tsx
    sidebar.tsx
    add-streamer-form.tsx
    streamer-table.tsx
    community-editor.tsx
```

## Domain routing

`proxy.ts` runs on every request:

1. Internal paths (`/_next`, `/api`, `/admin`, `/c/`, `/_not-configured`) pass through unchanged.
2. Local hosts (`localhost`, `*.local`, `127.*`) pass through to path-based routes.
3. Anything else gets a 60s-cached `GET /api/v1/communities/by-domain/{host}` lookup:
   - match → rewrite to `/c/<slug>` (URL bar stays clean)
   - no match → rewrite to `/_not-configured` (fail-closed)

For local development you can add `127.0.0.1 lsn.local` to `/etc/hosts` and the proxy will fall through to `/c/<slug>` once `lsn.local` is in the community's domains list.

## Deployment

Vercel handles the Next.js side natively. The backend runs separately (Coolify/Fly per `../docs/DEPLOYMENT.md`). Cross-origin requests are already allowed by the backend's `CorsLayer::permissive()`.

To attach a community domain:
1. Point DNS at the Vercel deployment.
2. Add the domain in the Vercel project settings.
3. In `/admin/communities/<slug>`, add the same hostname to the domains list.

The proxy resolves the rest.

## Out of scope for v1

- Embedded video playback (cards link out)
- WebSocket live updates (we poll page 0 every 60s)
- Discovery rule editor (backend endpoint not implemented yet)
- A detailed /stats dashboard (depends on backend exposing it)
- A per-community theme override (the theme cookie is global)

## Build

```bash
bun run build
bun run start
```
