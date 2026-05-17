# PRODUCT.md

## What this is

A brandable frontend for the StreamAggregator Rust service. One backend, many fronts. Each "front" is a curated community (e.g. *LiveStreamNorge*, *SwedishStreamers*, *VTubersDE*) defined by a filter recipe over the aggregated stream pool: language + tags + labels + group + platform + category + viewer range.

The Rust backend already exposes everything we need on `GET /api/v1/streams` with rich filter query params. Communities are not a backend concept; they are saved filter presets on the frontend that select a slice of the global stream pool.

## Register

- **Public-facing (`/c/[slug]` and root):** register = **brand**. Design IS the product. Each community has its own identity, palette, tone. People go there to watch, browse, discover. The directory is the experience.
- **Admin (`/admin/*`):** register = **product**. Design SERVES the product. Dense, fast, functional. Admins manage tracked streamers, communities, and inspect platform health.

Both registers ship from the same Next.js app, gated by route.

## Users

1. **Casual viewer.** Lands on a community page (e.g. `livestreamnorge.example.com` or `lsn.example.com/c/no`). Wants to see "who's live right now, in my community, ranked by something useful." Will leave in <8 seconds if it looks generic or slow.
2. **Power viewer.** Same intent, plus wants filters: by category, language, search by streamer name, sort by viewers.
3. **Curator/admin.** Maintains the directory. Adds streamers manually, defines discovery rules (when backend supports them), tweaks the community brand and filter recipe, monitors platform health.

Visitor traffic dominates by ~3 orders of magnitude. The public side must be perfect; the admin side must be sufficient.

## Tone

Community-driven and slightly hand-made, not corporate. Each community sets its own voice (Norwegian directory feels different from a global gaming directory) but the shared chrome stays restrained so brands can lead.

The aggregator itself is opinionless. The communities are opinionated.

## Anti-references

- Not a Twitch clone. We are not a streaming site. We are a *directory* — a lobby, a starting page.
- Not a SaaS dashboard. No KPI cards. No "stat tiles with arrows."
- Not a generic "card grid with icons." Stream thumbnails are the visual content, not decoration around it.
- Not generic dark-purple-gaming. Brand register's reflex-reject zone.

## Strategic principles

1. **Live first.** A streamer who is offline is invisible by default. Only live broadcasters show on the main grid. Offline streamers exist in admin views and on /all routes.
2. **Communities are filter presets.** A community is a typed config object (slug, brand, filter recipe). Adding a community is a config edit, not a deploy of a new app.
3. **Brand differs, layout doesn't.** Communities reskin colors, logo, name, tagline, optional accent. They do NOT reskin layout. Consistency makes the admin easier and reduces design drift.
4. **The frontend never owns truth.** All data comes from the Rust API. The frontend caches and presents. No client-side database, no parallel state.
5. **Degrade gracefully.** Not every documented backend route is implemented yet (discovery rules, websockets, stats are TBD). Build pages that hide cleanly when an endpoint is missing.

## What ships in v1

Public:
- `/c/[slug]` — community grid (live streams matching the community's filter recipe)
- `/c/[slug]/all` — same with offline streamers shown
- `/c/[slug]/about` — what this community is, who curates it
- `/` — community picker (or auto-redirect to default community via env)

Admin (API-key gated):
- `/admin` — overview (counts, platform health snapshot)
- `/admin/streamers` — list, search, add (single + bulk), remove
- `/admin/communities` — list, edit filter recipe and brand
- `/admin/platforms` — read-only platform status

## What does NOT ship in v1

- Embedded video playback (we link out to the platform)
- User accounts / favoriting / notifications
- Multi-region or multi-tenant deployment (single-instance is fine; community brand alone differentiates)
- WebSocket live updates (poll every 60s instead; switch to WS when backend exposes it)
- Discovery rule editor (backend route documented but not implemented; admin shows "coming soon" placeholder)
