import { notFound } from "next/navigation";

import { getCommunityRaw, listPlatforms } from "@/lib/api";
import { CommunityEditor } from "@/components/admin/community-editor";

interface Props {
  params: Promise<{ slug: string }>;
}

export default async function EditCommunityPage({ params }: Props) {
  const { slug } = await params;
  const [community, platforms] = await Promise.all([
    getCommunityRaw(slug, { noStore: true }),
    listPlatforms({ revalidate: 60 }),
  ]);

  if (!community) notFound();

  return (
    <div className="space-y-6">
      <header className="space-y-1">
        <p className="text-foreground-dim font-mono text-xs uppercase tracking-wider">
          Edit · {slug}
        </p>
        <h1 className="text-2xl font-semibold tracking-tight">{community.name}</h1>
      </header>

      <CommunityEditor
        initial={community}
        platformIds={platforms.map((p) => p.id)}
      />
    </div>
  );
}
