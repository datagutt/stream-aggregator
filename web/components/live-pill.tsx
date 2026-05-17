export function LivePill({ className = "" }: { className?: string }) {
  return (
    <span
      role="img"
      aria-label="Live"
      className={
        "inline-flex items-center gap-1.5 rounded-sm bg-black/65 px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wider text-white backdrop-blur-sm " +
        className
      }
    >
      <span
        className="live-pulse inline-block size-1.5 rounded-full"
        style={{ background: "var(--color-live)", animation: "live-pulse 2s ease-in-out infinite" }}
      />
      Live
    </span>
  );
}
