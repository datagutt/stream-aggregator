/**
 * Display formatters. All locale-aware, English by default.
 */

const viewerFormatter = new Intl.NumberFormat("en-US");
const viewerCompactFormatter = new Intl.NumberFormat("en-US", {
  notation: "compact",
  maximumFractionDigits: 1,
});

/** "12,453" — used in dense surfaces. */
export function formatViewers(n: number | null | undefined): string {
  if (n == null) return "0";
  return viewerFormatter.format(n);
}

/** "12.4K" — used inside stream-card overlays. */
export function formatViewersCompact(n: number | null | undefined): string {
  if (n == null) return "0";
  return viewerCompactFormatter.format(n);
}

/**
 * "3m ago", "2h ago", "Mar 14". Cap relative form at 24h; older falls back
 * to a short absolute date.
 */
export function timeSince(iso: string | null | undefined, now: Date = new Date()): string {
  if (!iso) return "";
  const then = new Date(iso);
  if (Number.isNaN(then.getTime())) return "";
  const delta = Math.max(0, Math.floor((now.getTime() - then.getTime()) / 1000));
  if (delta < 60) return `${delta}s ago`;
  if (delta < 3600) return `${Math.floor(delta / 60)}m ago`;
  if (delta < 86_400) return `${Math.floor(delta / 3600)}h ago`;
  return then.toLocaleDateString("en-US", { month: "short", day: "numeric" });
}

/** Initials for a missing avatar. */
export function initials(name: string): string {
  return name
    .split(/\s+/)
    .filter(Boolean)
    .map((w) => w[0]?.toUpperCase() ?? "")
    .slice(0, 2)
    .join("");
}

/**
 * Wrap an OKLCH triplet ("L C H") into an oklch() CSS function, with a
 * fallback that's safe to drop into `style={{ ... }}`.
 */
export function oklchTriplet(triplet: string | null | undefined, fallback: string): string {
  if (!triplet) return `oklch(${fallback})`;
  return `oklch(${triplet})`;
}

/**
 * Build the platform's canonical stream URL.
 *
 * `login` is the URL-safe handle the provider populates. We require it for
 * every platform whose `user_id` is a non-URL identifier (Twitch numeric ID,
 * YouTube would technically work either way, etc). When login is missing
 * (historical rows from before the column was added), return `null` so the
 * caller can render the card as non-clickable rather than producing a broken
 * URL pointing at the wrong page.
 */
export function streamUrl(platform: string, login: string | null): string | null {
  if (!login) return null;
  const safe = encodeURIComponent(login);
  switch (platform) {
    case "twitch":
      return `https://www.twitch.tv/${safe}`;
    case "youtube":
      // login is the UC... channel ID; the canonical URL surfaces "live"
      // when the channel is live and the latest video otherwise.
      return `https://www.youtube.com/channel/${safe}/live`;
    case "kick":
      return `https://kick.com/${safe}`;
    case "tiktok":
      return `https://www.tiktok.com/@${safe}/live`;
    case "guac":
      return `https://guac.live/${safe}`;
    case "angelthump":
      return `https://angelthump.com/${safe}`;
    case "robotstreamer":
      return `https://robotstreamer.com/robot/${safe}`;
    default:
      return null;
  }
}
