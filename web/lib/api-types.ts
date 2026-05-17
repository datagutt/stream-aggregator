/**
 * TypeScript mirrors of the Rust backend's serialized models.
 *
 * The backend uses snake_case throughout. We keep the same casing on the wire
 * (via the `Raw*` types) and provide camelCase domain types for ergonomic use
 * inside the frontend, with `from*` adapters in api.ts doing the conversion.
 */

export type ThemeMode = "dark" | "light";

// ──────────────────────────────────────────────────────────────────────────
// Streams
// ──────────────────────────────────────────────────────────────────────────

export interface StreamInfo {
  id: string;
  platform: string;
  userId: string;
  /** URL-safe handle/login on the platform (e.g. Twitch user_login). Use
      this — not userId — when building external URLs. May be absent for
      historical rows from before the provider populated it. */
  login: string | null;
  displayName: string;
  avatarUrl: string | null;
  isLive: boolean;
  title: string | null;
  viewerCount: number | null;
  thumbnailUrl: string | null;
  category: string | null;
  tags: string[];
  language: string | null;
  startedAt: string | null;
  lastFetchedAt: string;
  lastLiveAt: string | null;
  metadata: Record<string, unknown>;
}

export interface RawStreamInfo {
  id: string;
  platform: string;
  user_id: string;
  login: string | null;
  display_name: string;
  avatar_url: string | null;
  is_live: boolean;
  title: string | null;
  viewer_count: number | null;
  thumbnail_url: string | null;
  category: string | null;
  tags: string[];
  language: string | null;
  started_at: string | null;
  last_fetched_at: string;
  last_live_at: string | null;
  metadata: Record<string, unknown>;
}

export interface StreamPage {
  data: StreamInfo[];
  pagination: {
    page: number;
    pageSize: number;
    total: number;
    totalPages: number;
  };
}

export interface RawStreamPage {
  data: RawStreamInfo[];
  pagination: {
    page: number;
    page_size: number;
    total: number;
    total_pages: number;
  };
}

export interface StreamQuery {
  /** Empty arrays mean "no filter on this dimension". */
  platforms?: string[];
  languages?: string[];
  categories?: string[];
  tags?: string[];
  group?: string;
  labels?: Record<string, string>;
  isLive?: boolean;
  search?: string;
  minViewers?: number;
  maxViewers?: number;
  sort?: "viewers" | "name" | "platform" | "fetched" | "live";
  order?: "asc" | "desc";
  page?: number;
  pageSize?: number;
}

// ──────────────────────────────────────────────────────────────────────────
// Tracked streamers
// ──────────────────────────────────────────────────────────────────────────

export interface TrackedStreamer {
  platform: string;
  userId: string;
  customName: string | null;
  group: string | null;
  priority: number | null;
  labels: Record<string, string>;
  source: "manual" | "discovery";
  discoveryRuleId: string | null;
  createdAt: string;
}

export interface RawTrackedStreamer {
  platform: string;
  user_id: string;
  custom_name: string | null;
  group: string | null;
  priority: number | null;
  labels: Record<string, string>;
  source: "manual" | "discovery";
  discovery_rule_id: string | null;
  created_at: string;
}

export interface AddStreamerInput {
  platform: string;
  /** Either user_id or username must be set, not both. */
  userId?: string;
  username?: string;
  customName?: string;
  group?: string;
  priority?: number;
  labels?: Record<string, string>;
}

// ──────────────────────────────────────────────────────────────────────────
// Communities
// ──────────────────────────────────────────────────────────────────────────

export interface CommunityFilter {
  platforms?: string[];
  languages?: string[];
  categories?: string[];
  tags?: string[];
  groups?: string[];
  labels?: Record<string, string>;
  minViewers?: number;
}

export interface RawCommunityFilter {
  platforms?: string[];
  languages?: string[];
  categories?: string[];
  tags?: string[];
  groups?: string[];
  labels?: Record<string, string>;
  min_viewers?: number | null;
}

export interface Community {
  slug: string;
  name: string;
  tagline: string | null;
  accent: string;
  accentContrast: string | null;
  logoUrl: string | null;
  defaultTheme: ThemeMode;
  domains: string[];
  filter: CommunityFilter;
  aboutMd: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface RawCommunity {
  slug: string;
  name: string;
  tagline: string | null;
  accent: string;
  accent_contrast: string | null;
  logo_url: string | null;
  default_theme: ThemeMode;
  domains: string[];
  filter: RawCommunityFilter;
  about_md: string | null;
  created_at: string;
  updated_at: string;
}

export interface UpsertCommunityInput {
  slug: string;
  name: string;
  tagline?: string | null;
  accent: string;
  accentContrast?: string | null;
  logoUrl?: string | null;
  defaultTheme: ThemeMode;
  domains: string[];
  filter: CommunityFilter;
  aboutMd?: string | null;
}

// ──────────────────────────────────────────────────────────────────────────
// Platforms
// ──────────────────────────────────────────────────────────────────────────

export interface PlatformInfo {
  id: string;
  name: string;
  baseUrl: string;
  supportsDiscovery: boolean;
}

export interface RawPlatformInfo {
  id: string;
  name: string;
  base_url: string;
  supports_discovery: boolean;
}

// ──────────────────────────────────────────────────────────────────────────
// Envelopes
// ──────────────────────────────────────────────────────────────────────────

export interface ApiResponse<T> {
  data: T;
}

export interface ApiError {
  error: {
    code: string;
    message: string;
  };
}
