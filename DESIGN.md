# DESIGN.md

## Stack

- Next.js 15 (app router, React 19)
- Tailwind CSS v4 (CSS-first, no plugin config)
- HeroUI v3 beta (`@heroui/react@beta`, `@heroui/styles@beta`) — built on Tailwind v4 + React Aria
- TypeScript strict
- Bun as package manager and runtime
- `next-themes` for theme switching (per-community default theme, user override allowed)

No Tailwind plugin for HeroUI v3. Styles are imported via CSS: `@import "tailwindcss"; @import "@heroui/styles";`. Theme tokens are CSS variables on `:root`, overridden per community at the layout level.

## Theme posture

**Scene sentence (public, default community):** *A Norwegian on their phone at 8pm, idly browsing to see if any of their favorite Twitch streamers are live before they pick something to watch.* That sentence forces dark mode: thumbnails are the content, dark surrounds them best, and 8pm phone glances want low glare. Public default theme = **dark**.

**Scene sentence (admin):** *A curator on a desktop at noon adding fifteen streamers from a Discord list, glancing at platform health.* That sentence forces light mode: dense forms, tabular data, daytime context. Admin default theme = **light**.

Both directions are overridable by user toggle and by community config. The category-reflex check survives this because:
- First-order trap for "live streaming directory" is dark purple. We avoid purple.
- Second-order trap is dark + neon green/blue. We avoid neon. Default accent is warm.

## Color strategy

**Committed** for community pages. One brand color carries ~30% of the surface (header, accents, live pill, primary buttons). Restrained for admin (tinted neutrals + one accent ≤10%).

All colors are OKLCH. Neutrals tinted toward the community accent's hue (chroma 0.005–0.008). Never `#000` or `#fff`.

### Default community palette (the "neutral" brand when no community.json overrides)

```css
:root {
  /* Backgrounds, dark default */
  --color-background:        oklch(0.16 0.005 270);
  --color-surface:           oklch(0.20 0.006 270);
  --color-surface-raised:    oklch(0.24 0.007 270);
  --color-border:            oklch(0.30 0.008 270);

  /* Foregrounds */
  --color-foreground:        oklch(0.96 0.003 270);
  --color-foreground-muted:  oklch(0.72 0.006 270);
  --color-foreground-dim:    oklch(0.55 0.006 270);

  /* Brand accent (overridden per community) */
  --color-brand:             oklch(0.68 0.16 25);   /* warm coral, not gaming-purple */
  --color-brand-contrast:    oklch(0.98 0.01 25);

  /* Live indicator — always saturated red, never branded */
  --color-live:              oklch(0.62 0.22 25);

  /* Platform pill colors (semantic, fixed) */
  --color-platform-twitch:   oklch(0.55 0.20 295);
  --color-platform-youtube:  oklch(0.60 0.22 25);
  --color-platform-kick:     oklch(0.78 0.18 145);
  --color-platform-tiktok:   oklch(0.55 0.01 270);
}
```

### Per-community override (loaded from community config)

A community sets only the brand variables; everything else inherits.

```css
[data-community="livestreamnorge"] {
  --color-brand: oklch(0.58 0.20 250);          /* Nordic blue */
  --color-brand-contrast: oklch(0.98 0.01 250);
}

[data-community="swedishstreamers"] {
  --color-brand: oklch(0.78 0.16 95);           /* gold */
  --color-brand-contrast: oklch(0.18 0.01 95);
}
```

## Typography

- Body: **Geist Sans** (variable). Loaded via `next/font/google`.
- Display (community name in header, page titles): same family, weight 700, tracking tight (`-0.02em`) at >32px.
- Mono (admin, viewer counts when fixed-width matters): **Geist Mono**.

Scale (ratio ≈ 1.25):
- xs 12 / sm 14 / base 16 / lg 18 / xl 20 / 2xl 26 / 3xl 32 / 4xl 42 / 5xl 56

Body line-length capped at 65ch in long-form (`/c/[slug]/about`). Stream titles truncated to 2 lines with `text-overflow: ellipsis`.

## Layout primitives

- Grid breakpoints: 1 col <640, 2 col 640–960, 3 col 960–1280, 4 col 1280–1600, 5 col >1600.
- Outer padding: clamp(16px, 4vw, 48px).
- Stream card aspect: 16:9 thumbnail + 64px metadata bar below. Never nested in another card.
- Filter bar: horizontal at top on desktop, drawer (`HeroUI Drawer`) on mobile.
- Admin: 240px fixed sidebar nav on desktop, top tabs on mobile.

## Motion

- Transitions ≤200ms, ease-out-quart `cubic-bezier(0.165, 0.84, 0.44, 1)`.
- Live pill pulses (2s, opacity 0.5 → 1, infinite).
- Viewer count animates on update: number tween 600ms when value changes by >5%.
- Thumbnail hover: scale 1.03, brightness 1.05, 180ms. Mobile: no hover, swap to subtle saturation bump on intersection.
- No bounce. No elastic. No layout-property animations.

## Stream card spec

```
┌─────────────────────────────────┐
│                                 │
│         THUMBNAIL 16:9          │   ← live pill top-left, viewers top-right
│                                 │
├─────────────────────────────────┤
│ AVATAR  Streamer Display Name   │
│   24    Stream title here trunc │   ← title is the headline, streamer second
│         #category · platform    │
└─────────────────────────────────┘
```

Refinements:
- Live pill: red dot + "LIVE" text in `Geist Mono`, 11px, uppercase.
- Viewer count: top-right, dark scrim, monospace, comma-grouped.
- Whole card is a single anchor to the platform stream URL (`target="_blank"`).
- Offline streamers (only on `/all` and admin): no thumbnail, just the metadata bar with grey avatar and "offline" pill.

## Filter UX

The frontend filter bar maps 1:1 onto the backend's `GET /api/v1/streams` query params. No client-side filtering — every change refetches.

Filter chips (toggleable):
- Platform (multi)
- Language (multi)
- Category (autocomplete)
- Tags (multi, from selected platform's tag list)
- Viewer range (slider)

Search input is debounced 300ms.

Active filters render as removable chips below the bar. "Clear all" link appears when >0 active.

## Anti-patterns to actively reject

From the global shared design laws:
- No side-stripe borders on stream cards or alerts.
- No gradient text anywhere (no "LIVE" gradients, no community name gradients).
- No glassmorphism on cards; reserved for the optional sticky filter bar's `backdrop-blur` on scroll.
- No hero-metric template on admin home.
- No modal as first thought. Add Streamer is an inline form, not a dialog.

## Accessibility (non-negotiable)

- All HeroUI components inherit React Aria a11y; do not break it with custom click handlers on non-button elements.
- Color contrast: text ≥ 4.5:1 against its surface. Brand color may not be the sole signal for live state — always paired with the "LIVE" text.
- Focus ring visible on every interactive element, tinted with `--color-brand`.
- `prefers-reduced-motion` disables the pulse, viewer tween, and hover scale.
- Page titles + `<main>` landmarks on every route.

## File / folder conventions

```
web/
  app/
    layout.tsx                 # root, fonts, HeroUIProvider, theme provider
    page.tsx                   # community picker / redirect
    c/[slug]/
      layout.tsx               # community context, brand CSS vars, header
      page.tsx                 # live grid
      all/page.tsx             # live + offline
      about/page.tsx
    admin/
      layout.tsx               # API-key gate, sidebar
      page.tsx                 # overview
      streamers/page.tsx
      communities/page.tsx
      platforms/page.tsx
  components/
    stream-card.tsx
    filter-bar.tsx
    live-pill.tsx
    platform-pill.tsx
    community-header.tsx
    admin/
  lib/
    api.ts                     # typed fetch client for the Rust API
    communities.ts             # community config loader + types
    types.ts                   # mirror of backend models
  communities/
    livestreamnorge.json       # example community config
    default.json
  public/
  styles/
    globals.css                # @import tailwind, heroui, CSS vars
```
