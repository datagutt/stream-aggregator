const LABELS: Record<string, string> = {
  twitch: "Twitch",
  youtube: "YouTube",
  kick: "Kick",
  tiktok: "TikTok",
  guac: "Guac",
  angelthump: "AngelThump",
  robotstreamer: "RobotStreamer",
};

export function PlatformPill({ platform, className = "" }: { platform: string; className?: string }) {
  const label = LABELS[platform] ?? platform;
  // Lookup is a CSS variable, generated for known platforms in globals.css.
  const swatch = `var(--color-platform-${platform}, var(--color-foreground-dim))`;
  return (
    <span
      className={
        "inline-flex items-center gap-1.5 text-[11px] font-medium text-foreground-muted " + className
      }
    >
      <span
        aria-hidden
        className="inline-block size-1.5 rounded-full"
        style={{ background: swatch }}
      />
      {label}
    </span>
  );
}
