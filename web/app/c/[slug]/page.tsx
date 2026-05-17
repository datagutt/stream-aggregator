import { notFound } from "next/navigation";
import type { SearchParams } from "nuqs/server";

import { listPlatforms, listStreams } from "@/lib/api";
import { getCommunity, communityToStreamFilter } from "@/lib/communities";
import { StreamGrid } from "@/components/stream-grid";
import { FilterBar } from "@/components/filter-bar";
import { communityFiltersCache, type SortOption } from "@/lib/community-search-params";
import type { StreamQuery } from "@/lib/api-types";

interface Props {
  params: Promise<{ slug: string }>;
  searchParams: Promise<SearchParams>;
}

function sortToApi(sort: SortOption): { sort: StreamQuery["sort"]; order: StreamQuery["order"] } {
  if (sort === "name") return { sort: "name", order: "asc" };
  if (sort === "started") return { sort: "live", order: "desc" };
  return { sort: "viewers", order: "desc" };
}

export default async function CommunityHome({ params, searchParams }: Props) {
  const { slug } = await params;
  const community = await getCommunity(slug);
  if (!community) notFound();

  // Parse search params once at the root via the typed nuqs cache.
  const filters = await communityFiltersCache.parse(searchParams);

  const communityFilter = communityToStreamFilter(community.filter);
  const { sort, order } = sortToApi(filters.sort);

  // The user's platform pick can only narrow within the community's allow-list.
  const userPlatforms = filters.platform;
  const mergedPlatforms = communityFilter.platforms?.length
    ? userPlatforms.length
      ? userPlatforms.filter((p) =>
          (communityFilter.platforms ?? []).includes(p),
        )
      : communityFilter.platforms
    : userPlatforms.length
      ? userPlatforms
      : undefined;

  const mergedCategories = filters.category
    ? [filters.category]
    : communityFilter.categories;

  const initialPage = await listStreams(
    {
      ...communityFilter,
      platforms: mergedPlatforms,
      categories: mergedCategories,
      isLive: true,
      search: filters.q || undefined,
      sort,
      order,
      page: 0,
      pageSize: 60,
    },
    { revalidate: 30 },
  );

  // Build the platform multi-select source list.
  const platforms = await listPlatforms({ revalidate: 60 });
  const allowed =
    communityFilter.platforms && communityFilter.platforms.length
      ? platforms.filter((p) => (communityFilter.platforms ?? []).includes(p.id))
      : platforms;

  const resetKey = [
    slug,
    "live",
    filters.q,
    filters.category,
    [...filters.platform].sort().join(","),
    filters.sort,
  ].join("|");

  return (
    <section aria-label="Live streams" className="space-y-6">
      <FilterBar availablePlatforms={allowed} />
      <StreamGrid
        initialPage={initialPage}
        communityFilter={{ ...communityFilter, isLive: true }}
        userFilter={{
          platforms: userPlatforms.length ? userPlatforms : undefined,
          categories: filters.category ? [filters.category] : undefined,
          search: filters.q || undefined,
          sort,
          order,
        }}
        resetKey={resetKey}
      />
    </section>
  );
}
