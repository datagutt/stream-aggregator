/**
 * Typed fetch client for the StreamAggregator Rust API.
 *
 * Server components call these directly with `next: { revalidate: ... }`.
 * Admin write paths pass the API key via `apiKey` so it never leaks to the
 * browser — they're meant to be called from server actions or route handlers
 * that resolved the key from the httpOnly cookie.
 */

import type {
  AddStreamerInput,
  ApiResponse,
  Community,
  PlatformInfo,
  RawCommunity,
  RawPlatformInfo,
  RawStreamInfo,
  RawStreamPage,
  RawTrackedStreamer,
  StreamInfo,
  StreamPage,
  StreamQuery,
  TrackedStreamer,
  UpsertCommunityInput,
} from "./api-types";

const API_URL =
  process.env.STREAM_AGGREGATOR_API_URL ??
  process.env.NEXT_PUBLIC_API_URL ??
  "http://localhost:8080";

export interface FetchOpts {
  /** Next.js cache config. */
  revalidate?: number;
  /** Force no-store (e.g. for admin mutations). */
  noStore?: boolean;
  /** Abort signal. */
  signal?: AbortSignal;
  /** Tags for on-demand revalidation. */
  tags?: string[];
  /** Optional API key. When provided, sent as X-API-Key. Used by admin
      server actions that resolved the key from the httpOnly cookie. */
  apiKey?: string;
}

export interface AuthedOpts extends FetchOpts {
  apiKey: string;
}

class HttpError extends Error {
  constructor(public status: number, public code: string, message: string) {
    super(message);
    this.name = "HttpError";
  }
}

async function request<T>(
  path: string,
  init: RequestInit & { revalidate?: number; noStore?: boolean; tags?: string[] },
): Promise<T> {
  const { revalidate, noStore, tags, ...rest } = init;
  const url = `${API_URL}${path}`;
  // noStore wins: Next.js complains if both `cache: no-store` and
  // `next.revalidate` are present.
  const next: { revalidate?: number | false; tags?: string[] } = {};
  if (!noStore && typeof revalidate === "number") next.revalidate = revalidate;
  if (tags && tags.length) next.tags = tags;

  const res = await fetch(url, {
    ...rest,
    cache: noStore ? "no-store" : undefined,
    next: Object.keys(next).length ? next : undefined,
  });

  if (!res.ok) {
    let code = "HTTP_ERROR";
    let message = res.statusText;
    try {
      const body = (await res.json()) as { error?: { code?: string; message?: string } };
      if (body.error) {
        code = body.error.code ?? code;
        message = body.error.message ?? message;
      }
    } catch {
      // body was not JSON, keep statusText
    }
    throw new HttpError(res.status, code, message);
  }

  if (res.status === 204) return undefined as T;
  return (await res.json()) as T;
}

function authHeaders(apiKey: string): HeadersInit {
  return { "X-API-Key": apiKey };
}

// ──────────────────────────────────────────────────────────────────────────
// Streams
// ──────────────────────────────────────────────────────────────────────────

export function buildStreamsQueryString(q: StreamQuery): string {
  const sp = new URLSearchParams();

  const arrayParam = (key: string, values?: string[]) => {
    if (!values) return;
    for (const v of values) {
      if (v !== undefined && v !== "") sp.append(`${key}[]`, v);
    }
  };

  arrayParam("platform", q.platforms);
  arrayParam("language", q.languages);
  arrayParam("category", q.categories);
  arrayParam("tag", q.tags);
  if (q.group) sp.set("group", q.group);
  if (q.labels) {
    for (const [k, v] of Object.entries(q.labels)) {
      sp.append(`labels[${k}]`, v);
    }
  }
  if (typeof q.isLive === "boolean") sp.set("live", String(q.isLive));
  if (q.search) sp.set("search", q.search);
  if (typeof q.minViewers === "number") sp.set("min_viewers", String(q.minViewers));
  if (typeof q.maxViewers === "number") sp.set("max_viewers", String(q.maxViewers));
  if (q.sort) sp.set("sort", q.sort);
  if (q.order) sp.set("order", q.order);
  if (typeof q.page === "number") sp.set("page", String(q.page));
  if (typeof q.pageSize === "number") sp.set("per_page", String(q.pageSize));

  const qs = sp.toString();
  return qs ? `?${qs}` : "";
}

function streamFromRaw(r: RawStreamInfo): StreamInfo {
  return {
    id: r.id,
    platform: r.platform,
    userId: r.user_id,
    login: r.login,
    displayName: r.display_name,
    avatarUrl: r.avatar_url,
    isLive: r.is_live,
    title: r.title,
    viewerCount: r.viewer_count,
    thumbnailUrl: r.thumbnail_url,
    category: r.category,
    tags: r.tags,
    language: r.language,
    startedAt: r.started_at,
    lastFetchedAt: r.last_fetched_at,
    lastLiveAt: r.last_live_at,
    metadata: r.metadata,
  };
}

function pageFromRaw(r: RawStreamPage): StreamPage {
  return {
    data: r.data.map(streamFromRaw),
    pagination: {
      page: r.pagination.page,
      pageSize: r.pagination.page_size,
      total: r.pagination.total,
      totalPages: r.pagination.total_pages,
    },
  };
}

export async function listStreams(
  q: StreamQuery = {},
  opts: FetchOpts = {},
): Promise<StreamPage> {
  const raw = await request<RawStreamPage>(
    `/api/v1/streams${buildStreamsQueryString(q)}`,
    { method: "GET", revalidate: opts.revalidate ?? 30, signal: opts.signal, tags: opts.tags },
  );
  return pageFromRaw(raw);
}

export async function getStream(id: string, opts: FetchOpts = {}): Promise<StreamInfo> {
  const raw = await request<ApiResponse<RawStreamInfo>>(`/api/v1/streams/${encodeURIComponent(id)}`, {
    method: "GET",
    revalidate: opts.revalidate ?? 30,
    signal: opts.signal,
  });
  return streamFromRaw(raw.data);
}

// ──────────────────────────────────────────────────────────────────────────
// Tracked streamers
// ──────────────────────────────────────────────────────────────────────────

function streamerFromRaw(r: RawTrackedStreamer): TrackedStreamer {
  return {
    platform: r.platform,
    userId: r.user_id,
    customName: r.custom_name,
    group: r.group,
    priority: r.priority,
    labels: r.labels,
    source: r.source,
    discoveryRuleId: r.discovery_rule_id,
    createdAt: r.created_at,
  };
}

export async function listStreamers(opts: FetchOpts = {}): Promise<TrackedStreamer[]> {
  const raw = await request<ApiResponse<RawTrackedStreamer[]>>(`/api/v1/streamers`, {
    method: "GET",
    revalidate: opts.revalidate ?? 30,
    signal: opts.signal,
    headers: opts.apiKey ? authHeaders(opts.apiKey) : undefined,
    noStore: opts.noStore,
  });
  return raw.data.map(streamerFromRaw);
}

export async function addStreamer(
  input: AddStreamerInput,
  opts: AuthedOpts,
): Promise<TrackedStreamer> {
  const body = {
    platform: input.platform,
    user_id: input.userId,
    username: input.username,
    custom_name: input.customName,
    group: input.group,
    priority: input.priority,
    labels: input.labels,
  };
  const raw = await request<ApiResponse<RawTrackedStreamer>>(`/api/v1/streamers`, {
    method: "POST",
    headers: { "Content-Type": "application/json", ...authHeaders(opts.apiKey) },
    body: JSON.stringify(body),
    noStore: true,
    signal: opts.signal,
  });
  return streamerFromRaw(raw.data);
}

export async function removeStreamer(
  platform: string,
  userId: string,
  opts: AuthedOpts,
): Promise<void> {
  await request<void>(
    `/api/v1/streamers/${encodeURIComponent(platform)}/${encodeURIComponent(userId)}`,
    {
      method: "DELETE",
      headers: authHeaders(opts.apiKey),
      noStore: true,
      signal: opts.signal,
    },
  );
}

// ──────────────────────────────────────────────────────────────────────────
// Platforms
// ──────────────────────────────────────────────────────────────────────────

function platformFromRaw(r: RawPlatformInfo): PlatformInfo {
  return {
    id: r.id,
    name: r.name,
    baseUrl: r.base_url,
    supportsDiscovery: r.supports_discovery,
  };
}

export async function listPlatforms(opts: FetchOpts = {}): Promise<PlatformInfo[]> {
  const raw = await request<ApiResponse<RawPlatformInfo[]>>(`/api/v1/platforms`, {
    method: "GET",
    revalidate: opts.revalidate ?? 60,
    signal: opts.signal,
  });
  return raw.data.map(platformFromRaw);
}

// ──────────────────────────────────────────────────────────────────────────
// Communities (raw HTTP; the typed wrapper with caching is in lib/communities.ts)
// ──────────────────────────────────────────────────────────────────────────

function communityFromRaw(r: RawCommunity): Community {
  return {
    slug: r.slug,
    name: r.name,
    tagline: r.tagline,
    accent: r.accent,
    accentContrast: r.accent_contrast,
    logoUrl: r.logo_url,
    defaultTheme: r.default_theme,
    domains: r.domains,
    filter: {
      platforms: r.filter.platforms ?? [],
      languages: r.filter.languages ?? [],
      categories: r.filter.categories ?? [],
      tags: r.filter.tags ?? [],
      groups: r.filter.groups ?? [],
      labels: r.filter.labels ?? {},
      minViewers: r.filter.min_viewers ?? undefined,
    },
    aboutMd: r.about_md,
    createdAt: r.created_at,
    updatedAt: r.updated_at,
  };
}

export async function listCommunitiesRaw(opts: FetchOpts = {}): Promise<Community[]> {
  const raw = await request<ApiResponse<RawCommunity[]>>(`/api/v1/communities`, {
    method: "GET",
    revalidate: opts.revalidate ?? 60,
    signal: opts.signal,
    tags: opts.tags,
  });
  return raw.data.map(communityFromRaw);
}

export async function getCommunityRaw(
  slug: string,
  opts: FetchOpts = {},
): Promise<Community | null> {
  try {
    const raw = await request<ApiResponse<RawCommunity>>(
      `/api/v1/communities/${encodeURIComponent(slug)}`,
      {
        method: "GET",
        revalidate: opts.revalidate ?? 60,
        signal: opts.signal,
        tags: opts.tags,
      },
    );
    return communityFromRaw(raw.data);
  } catch (e) {
    if (e instanceof HttpError && e.status === 404) return null;
    throw e;
  }
}

export async function getCommunityByDomainRaw(
  host: string,
  opts: FetchOpts = {},
): Promise<Community | null> {
  try {
    const raw = await request<ApiResponse<RawCommunity>>(
      `/api/v1/communities/by-domain/${encodeURIComponent(host)}`,
      {
        method: "GET",
        revalidate: opts.revalidate ?? 60,
        signal: opts.signal,
        tags: opts.tags,
      },
    );
    return communityFromRaw(raw.data);
  } catch (e) {
    if (e instanceof HttpError && e.status === 404) return null;
    throw e;
  }
}

function upsertBody(input: UpsertCommunityInput) {
  return {
    slug: input.slug,
    name: input.name,
    tagline: input.tagline ?? null,
    accent: input.accent,
    accent_contrast: input.accentContrast ?? null,
    logo_url: input.logoUrl ?? null,
    default_theme: input.defaultTheme,
    domains: input.domains,
    filter: {
      platforms: input.filter.platforms ?? [],
      languages: input.filter.languages ?? [],
      categories: input.filter.categories ?? [],
      tags: input.filter.tags ?? [],
      groups: input.filter.groups ?? [],
      labels: input.filter.labels ?? {},
      min_viewers: input.filter.minViewers ?? null,
    },
    about_md: input.aboutMd ?? null,
  };
}

export async function createCommunity(
  input: UpsertCommunityInput,
  opts: AuthedOpts,
): Promise<Community> {
  const raw = await request<ApiResponse<RawCommunity>>(`/api/v1/communities`, {
    method: "POST",
    headers: { "Content-Type": "application/json", ...authHeaders(opts.apiKey) },
    body: JSON.stringify(upsertBody(input)),
    noStore: true,
    signal: opts.signal,
  });
  return communityFromRaw(raw.data);
}

export async function updateCommunity(
  slug: string,
  input: UpsertCommunityInput,
  opts: AuthedOpts,
): Promise<Community> {
  const raw = await request<ApiResponse<RawCommunity>>(
    `/api/v1/communities/${encodeURIComponent(slug)}`,
    {
      method: "PUT",
      headers: { "Content-Type": "application/json", ...authHeaders(opts.apiKey) },
      body: JSON.stringify(upsertBody({ ...input, slug })),
      noStore: true,
      signal: opts.signal,
    },
  );
  return communityFromRaw(raw.data);
}

export async function deleteCommunity(slug: string, opts: AuthedOpts): Promise<void> {
  await request<void>(`/api/v1/communities/${encodeURIComponent(slug)}`, {
    method: "DELETE",
    headers: authHeaders(opts.apiKey),
    noStore: true,
    signal: opts.signal,
  });
}

export { HttpError };
