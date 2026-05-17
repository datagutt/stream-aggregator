/**
 * Typed search-params schema for community pages. Used by both:
 *
 * - Server components (page.tsx) via createSearchParamsCache, so parsing
 *   happens once at the root and any RSC can read the values.
 * - Client components (FilterBar) via useQueryStates, so the URL is the
 *   source of truth and back/forward navigation works for free.
 */

import {
  createSearchParamsCache,
  parseAsArrayOf,
  parseAsString,
  parseAsStringEnum,
} from "nuqs/server";

export const sortOptions = ["viewers", "started", "name"] as const;
export type SortOption = (typeof sortOptions)[number];

export const communityFilterParsers = {
  q: parseAsString.withDefault(""),
  category: parseAsString.withDefault(""),
  platform: parseAsArrayOf(parseAsString).withDefault([]),
  sort: parseAsStringEnum<SortOption>([...sortOptions]).withDefault("viewers"),
};

export const communityFiltersCache = createSearchParamsCache(communityFilterParsers);
