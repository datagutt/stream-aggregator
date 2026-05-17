/**
 * Community lookup layer with an in-process cache on top of the raw API.
 *
 * The middleware hits getCommunityByDomain on every request, so we cache
 * 60s per host to keep load minimal. Failures to refresh fall back to the
 * cached value when present (fail-open at the network layer); only a
 * successful "no match" response renders the unknown-host page.
 */

import type { Community, CommunityFilter, StreamQuery } from "./api-types";
import {
  getCommunityByDomainRaw,
  getCommunityRaw,
  listCommunitiesRaw,
} from "./api";

const CACHE_TTL_MS = 60_000;

interface CacheEntry<T> {
  value: T | null;
  fetchedAt: number;
}

const byDomain = new Map<string, CacheEntry<Community>>();
const bySlug = new Map<string, CacheEntry<Community>>();
let listCache: CacheEntry<Community[]> | null = null;

function fresh(entry: { fetchedAt: number } | null | undefined): boolean {
  return !!entry && Date.now() - entry.fetchedAt < CACHE_TTL_MS;
}

export async function listCommunities(): Promise<Community[]> {
  if (fresh(listCache) && listCache?.value) return listCache.value;
  try {
    const value = await listCommunitiesRaw();
    listCache = { value, fetchedAt: Date.now() };
    return value;
  } catch (err) {
    if (listCache?.value) return listCache.value;
    throw err;
  }
}

export async function getCommunity(slug: string): Promise<Community | null> {
  const cached = bySlug.get(slug);
  if (fresh(cached)) return cached!.value;
  try {
    const value = await getCommunityRaw(slug);
    bySlug.set(slug, { value, fetchedAt: Date.now() });
    return value;
  } catch (err) {
    if (cached) return cached.value;
    throw err;
  }
}

export async function getCommunityByDomain(host: string): Promise<Community | null> {
  const cached = byDomain.get(host);
  if (fresh(cached)) return cached!.value;
  try {
    const value = await getCommunityByDomainRaw(host);
    byDomain.set(host, { value, fetchedAt: Date.now() });
    return value;
  } catch (err) {
    if (cached) return cached.value;
    throw err;
  }
}

/**
 * Translate a CommunityFilter into the StreamQuery shape the API consumes.
 * Empty arrays map to "no constraint" downstream.
 */
export function communityToStreamFilter(filter: CommunityFilter): StreamQuery {
  return {
    platforms: filter.platforms,
    languages: filter.languages,
    categories: filter.categories,
    tags: filter.tags,
    labels: filter.labels,
    minViewers: filter.minViewers,
    // `groups` array is a backend gap: /streams supports one `group` scalar.
    // Pick the first if any are set so single-group communities still work.
    group: filter.groups && filter.groups.length ? filter.groups[0] : undefined,
  };
}

/** Test helper / admin invalidation: drop all cached entries. */
export function invalidateCommunityCache(): void {
  byDomain.clear();
  bySlug.clear();
  listCache = null;
}
