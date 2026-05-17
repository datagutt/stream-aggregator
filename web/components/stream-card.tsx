import type { StreamInfo } from "@/lib/api-types";
import { formatViewersCompact, initials, streamUrl, timeSince } from "@/lib/format";
import { LivePill } from "./live-pill";
import { PlatformPill } from "./platform-pill";

interface Props {
  stream: StreamInfo;
}

export function StreamCard({ stream }: Props) {
  const live = stream.isLive;
  return (
    <a
      href={streamUrl(stream.platform, stream.userId)}
      target="_blank"
      rel="noopener noreferrer"
      aria-label={`${stream.displayName} — ${stream.title ?? "offline"} on ${stream.platform}`}
      className="group focus-visible:ring-brand block overflow-hidden rounded-lg border border-border bg-surface transition-colors duration-200 hover:border-foreground-dim focus-visible:outline-none focus-visible:ring-2"
    >
      <div className="relative aspect-video w-full overflow-hidden bg-surface-raised">
        {live && stream.thumbnailUrl ? (
          // Plain <img> per DESIGN.md — no next/image allowlist to maintain.
          // eslint-disable-next-line @next/next/no-img-element
          <img
            src={stream.thumbnailUrl}
            alt=""
            loading="lazy"
            decoding="async"
            className="size-full object-cover transition duration-200 group-hover:scale-[1.03] group-hover:brightness-[1.05] motion-reduce:transition-none motion-reduce:group-hover:scale-100 motion-reduce:group-hover:brightness-100"
          />
        ) : (
          <div className="flex size-full items-center justify-center">
            <span className="text-foreground-dim font-mono text-[10px] uppercase tracking-wider">
              {live ? "no thumbnail" : `offline${stream.lastLiveAt ? ` · last live ${timeSince(stream.lastLiveAt)}` : ""}`}
            </span>
          </div>
        )}

        {live && (
          <>
            <LivePill className="absolute left-2 top-2" />
            {typeof stream.viewerCount === "number" && (
              <span className="absolute right-2 top-2 inline-flex items-center rounded-sm bg-black/65 px-1.5 py-0.5 font-mono text-[11px] tabular-nums text-white backdrop-blur-sm">
                {formatViewersCompact(stream.viewerCount)}
              </span>
            )}
          </>
        )}
      </div>

      <div className="flex gap-3 p-3">
        {stream.avatarUrl ? (
          // eslint-disable-next-line @next/next/no-img-element
          <img
            src={stream.avatarUrl}
            alt=""
            loading="lazy"
            decoding="async"
            className="size-9 shrink-0 rounded-full object-cover"
          />
        ) : (
          <div
            aria-hidden
            className="bg-surface-raised flex size-9 shrink-0 items-center justify-center rounded-full text-xs font-semibold text-foreground-muted"
          >
            {initials(stream.displayName)}
          </div>
        )}

        <div className="min-w-0 flex-1">
          <p className="truncate text-sm font-medium text-foreground">
            {stream.title ?? stream.displayName}
          </p>
          <p className="truncate text-xs text-foreground-muted">
            {stream.displayName}
            {stream.category ? <> · <span>{stream.category}</span></> : null}
          </p>
          <div className="mt-1.5">
            <PlatformPill platform={stream.platform} />
          </div>
        </div>
      </div>
    </a>
  );
}
