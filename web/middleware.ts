import { NextResponse, type NextRequest } from "next/server";

import { getCommunityByDomainRaw } from "@/lib/api";

/**
 * Domain-based routing. Resolved hosts get rewritten to /c/[slug] so the
 * URL stays clean. Unknown hosts go to /_not-configured (fail-closed).
 *
 * Local dev (localhost, *.local, 127.*) falls through to path-based
 * routes so developers can navigate without configuring DNS or domains.
 *
 * The host lookup is cached for 60s per host in the module-level Map below
 * to keep load off the backend.
 */

const CACHE_TTL_MS = 60_000;

interface HostCacheEntry {
  slug: string | null;
  fetchedAt: number;
}

const hostCache = new Map<string, HostCacheEntry>();

function isLocalHost(host: string): boolean {
  return (
    host === "localhost" ||
    host.endsWith(".local") ||
    host.startsWith("127.") ||
    host.startsWith("0.0.0.0")
  );
}

async function lookupSlug(host: string): Promise<string | null> {
  const cached = hostCache.get(host);
  if (cached && Date.now() - cached.fetchedAt < CACHE_TTL_MS) {
    return cached.slug;
  }
  try {
    const community = await getCommunityByDomainRaw(host, { revalidate: 60 });
    const slug = community?.slug ?? null;
    hostCache.set(host, { slug, fetchedAt: Date.now() });
    return slug;
  } catch {
    // Fail-open at the network layer: keep serving the cached mapping if any.
    if (cached) return cached.slug;
    // No prior knowledge of this host: treat as unconfigured rather than 500.
    return null;
  }
}

export async function middleware(req: NextRequest) {
  const host = req.headers.get("host")?.split(":")[0]?.toLowerCase() ?? "";
  const url = req.nextUrl;

  // Skip internal paths and surfaces that already own their routing.
  if (
    url.pathname.startsWith("/_next") ||
    url.pathname.startsWith("/api") ||
    url.pathname.startsWith("/admin") ||
    url.pathname.startsWith("/c/") ||
    url.pathname === "/_not-configured"
  ) {
    return NextResponse.next();
  }

  // Dev convenience: fall through to the in-app picker / path-based routes.
  if (isLocalHost(host)) {
    return NextResponse.next();
  }

  const slug = await lookupSlug(host);
  if (slug) {
    const rewritten = url.clone();
    const tail = url.pathname === "/" ? "" : url.pathname;
    rewritten.pathname = `/c/${slug}${tail}`;
    return NextResponse.rewrite(rewritten);
  }

  // Unknown host: fail closed.
  const fallback = url.clone();
  fallback.pathname = "/_not-configured";
  return NextResponse.rewrite(fallback);
}

export const config = {
  matcher: ["/((?!_next/static|_next/image|favicon.ico).*)"],
};
