import { notFound } from "next/navigation";

import { listStreams } from "@/lib/api";
import { getCommunity, communityToStreamFilter } from "@/lib/communities";
import { StreamGrid } from "@/components/stream-grid";

interface Props {
  params: Promise<{ slug: string }>;
}

export default async function CommunityHome({ params }: Props) {
  const { slug } = await params;
  const community = await getCommunity(slug);
  if (!community) notFound();

  const communityFilter = communityToStreamFilter(community.filter);
  const initialPage = await listStreams(
    {
      ...communityFilter,
      isLive: true,
      sort: "viewers",
      order: "desc",
      page: 0,
      pageSize: 60,
    },
    { revalidate: 30 },
  );

  return (
    <section aria-label="Live streams" className="space-y-6">
      <StreamGrid
        initialPage={initialPage}
        communityFilter={{ ...communityFilter, isLive: true }}
        userFilter={{ sort: "viewers", order: "desc" }}
        resetKey={`${slug}:live`}
      />
    </section>
  );
}
