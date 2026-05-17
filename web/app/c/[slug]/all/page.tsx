import { notFound } from "next/navigation";

import { listStreams } from "@/lib/api";
import { getCommunity, communityToStreamFilter } from "@/lib/communities";
import { StreamCard } from "@/components/stream-card";

interface Props {
  params: Promise<{ slug: string }>;
}

/**
 * /all — live streams first, then a separate section for tracked-but-offline
 * streamers. No filter bar, no infinite scroll: this is the "see everyone we
 * cover" surface for power viewers.
 */
export default async function CommunityAll({ params }: Props) {
  const { slug } = await params;
  const community = await getCommunity(slug);
  if (!community) notFound();

  const communityFilter = communityToStreamFilter(community.filter);

  // Two parallel fetches: live (page 0, by viewers) and offline (page 0, by name).
  const [livePage, offlinePage] = await Promise.all([
    listStreams(
      {
        ...communityFilter,
        isLive: true,
        sort: "viewers",
        order: "desc",
        page: 0,
        pageSize: 100,
      },
      { revalidate: 30 },
    ),
    listStreams(
      {
        ...communityFilter,
        isLive: false,
        sort: "name",
        order: "asc",
        page: 0,
        pageSize: 100,
      },
      { revalidate: 60 },
    ),
  ]);

  return (
    <div className="space-y-10">
      <section aria-labelledby="live-heading" className="space-y-3">
        <h2
          id="live-heading"
          className="text-foreground-muted font-mono text-xs uppercase tracking-wider"
        >
          Live now · {livePage.pagination.total}
        </h2>
        {livePage.data.length === 0 ? (
          <p className="text-foreground-muted text-sm">No one is live right now.</p>
        ) : (
          <ul className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5">
            {livePage.data.map((s) => (
              <li key={s.id}>
                <StreamCard stream={s} />
              </li>
            ))}
          </ul>
        )}
      </section>

      <section aria-labelledby="offline-heading" className="space-y-3">
        <h2
          id="offline-heading"
          className="text-foreground-muted font-mono text-xs uppercase tracking-wider"
        >
          Offline · {offlinePage.pagination.total}
        </h2>
        {offlinePage.data.length === 0 ? (
          <p className="text-foreground-muted text-sm">No tracked streamers are offline.</p>
        ) : (
          <ul className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5">
            {offlinePage.data.map((s) => (
              <li key={s.id}>
                <StreamCard stream={s} />
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
