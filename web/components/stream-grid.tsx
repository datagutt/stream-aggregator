"use client";

import { useCallback, useEffect, useRef, useState } from "react";

import type { StreamInfo, StreamPage, StreamQuery } from "@/lib/api-types";
import { listStreams } from "@/lib/api";
import { StreamCard } from "./stream-card";

interface Props {
  /** Server-rendered first page. */
  initialPage: StreamPage;
  /** Locked community-level filters (not user-editable). */
  communityFilter: StreamQuery;
  /** User filters from URL search params. */
  userFilter: StreamQuery;
  /** Treated as a stable key — when it changes, the grid resets. */
  resetKey: string;
}

const POLL_INTERVAL_MS = 60_000;
const PAGE_SIZE = 60;

function mergeFilters(community: StreamQuery, user: StreamQuery): StreamQuery {
  return {
    ...community,
    ...user,
    // The community's platforms/languages/etc are the locked superset; the user
    // can narrow within them. We intersect if user picked any, otherwise we
    // keep the community's set.
    platforms: intersect(community.platforms, user.platforms),
    languages: intersect(community.languages, user.languages),
    categories: intersect(community.categories, user.categories),
    tags: intersect(community.tags, user.tags),
  };
}

function intersect(a?: string[], b?: string[]): string[] | undefined {
  if (!a || a.length === 0) return b;
  if (!b || b.length === 0) return a;
  const lower = new Set(a.map((s) => s.toLowerCase()));
  return b.filter((v) => lower.has(v.toLowerCase()));
}

export function StreamGrid({ initialPage, communityFilter, userFilter, resetKey }: Props) {
  const [items, setItems] = useState<StreamInfo[]>(initialPage.data);
  const [page, setPage] = useState(initialPage.pagination.page);
  const [totalPages, setTotalPages] = useState(initialPage.pagination.totalPages);
  const [loadingMore, setLoadingMore] = useState(false);
  const sentinelRef = useRef<HTMLDivElement>(null);

  // Skip the first run — the server already gave us page 0 on mount.
  // Subsequent resetKey changes (user changes filters) trigger a fresh fetch.
  const isInitial = useRef(true);

  useEffect(() => {
    if (isInitial.current) {
      isInitial.current = false;
      return;
    }
    let cancelled = false;
    setItems([]);
    setPage(0);
    setTotalPages(0);
    setLoadingMore(true);
    void (async () => {
      try {
        const fresh = await listStreams(
          { ...mergeFilters(communityFilter, userFilter), page: 0, pageSize: PAGE_SIZE },
          { revalidate: 0, noStore: true },
        );
        if (cancelled) return;
        setItems(fresh.data);
        setPage(fresh.pagination.page);
        setTotalPages(fresh.pagination.totalPages);
      } finally {
        if (!cancelled) setLoadingMore(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [resetKey, communityFilter, userFilter]);

  // Background polling of page 0 to keep the live grid fresh.
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    let cancelled = false;

    const refresh = async () => {
      if (document.visibilityState !== "visible") return;
      try {
        const fresh = await listStreams(
          { ...mergeFilters(communityFilter, userFilter), page: 0, pageSize: PAGE_SIZE },
          { revalidate: 0, noStore: true },
        );
        if (cancelled) return;
        setItems((prev) => {
          // Keep any beyond-first-page items the user already loaded.
          const firstIds = new Set(fresh.data.map((s) => s.id));
          const tail = prev.slice(fresh.data.length).filter((s) => !firstIds.has(s.id));
          return [...fresh.data, ...tail];
        });
        setTotalPages(fresh.pagination.totalPages);
      } catch {
        // Polling failures are silent; next tick will retry.
      }
    };

    const schedule = () => {
      timer = setTimeout(async () => {
        await refresh();
        if (!cancelled) schedule();
      }, POLL_INTERVAL_MS);
    };

    const onVisibility = () => {
      if (document.visibilityState === "visible") void refresh();
    };

    schedule();
    document.addEventListener("visibilitychange", onVisibility);
    window.addEventListener("focus", onVisibility);

    return () => {
      cancelled = true;
      if (timer) clearTimeout(timer);
      document.removeEventListener("visibilitychange", onVisibility);
      window.removeEventListener("focus", onVisibility);
    };
  }, [communityFilter, userFilter]);

  // Infinite scroll via IntersectionObserver on the sentinel.
  const loadMore = useCallback(async () => {
    if (loadingMore) return;
    if (totalPages > 0 && page + 1 >= totalPages) return;
    setLoadingMore(true);
    try {
      const next = await listStreams(
        { ...mergeFilters(communityFilter, userFilter), page: page + 1, pageSize: PAGE_SIZE },
        { revalidate: 0, noStore: true },
      );
      setItems((prev) => {
        const known = new Set(prev.map((s) => s.id));
        return [...prev, ...next.data.filter((s) => !known.has(s.id))];
      });
      setPage(next.pagination.page);
      setTotalPages(next.pagination.totalPages);
    } finally {
      setLoadingMore(false);
    }
  }, [communityFilter, userFilter, loadingMore, page, totalPages]);

  useEffect(() => {
    const node = sentinelRef.current;
    if (!node) return;
    const io = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting)) void loadMore();
      },
      { rootMargin: "400px 0px" },
    );
    io.observe(node);
    return () => io.disconnect();
  }, [loadMore]);

  if (items.length === 0 && !loadingMore) {
    return (
      <div className="bg-surface flex flex-col items-center gap-2 rounded-lg border border-border px-8 py-16 text-center">
        <p className="text-foreground text-lg font-medium">No streams matching these filters.</p>
        <p className="text-foreground-muted text-sm">
          Try clearing a filter, or check back in a few minutes.
        </p>
      </div>
    );
  }

  return (
    <>
      <ul className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5">
        {items.map((s) => (
          <li key={s.id}>
            <StreamCard stream={s} />
          </li>
        ))}
      </ul>
      <div ref={sentinelRef} aria-hidden className="h-1 w-full" />
      {loadingMore && (
        <p className="text-foreground-dim mt-4 text-center font-mono text-xs uppercase tracking-wider">
          Loading…
        </p>
      )}
    </>
  );
}
