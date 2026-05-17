import { listPlatforms, listStreamers } from "@/lib/api";
import { getAdminKey } from "@/lib/auth";
import { AddStreamerForm } from "@/components/admin/add-streamer-form";
import { StreamerTable } from "@/components/admin/streamer-table";

export default async function AdminStreamers() {
  const apiKey = (await getAdminKey()) ?? undefined;

  const [streamers, platforms] = await Promise.all([
    listStreamers({ noStore: true, apiKey }),
    listPlatforms({ revalidate: 60 }),
  ]);

  return (
    <div className="space-y-6">
      <header className="space-y-1">
        <p className="text-foreground-dim font-mono text-xs uppercase tracking-wider">
          Streamers
        </p>
        <h1 className="text-2xl font-semibold tracking-tight">
          Tracked streamers ({streamers.length})
        </h1>
      </header>

      <AddStreamerForm platforms={platforms} />
      <StreamerTable streamers={streamers} />
    </div>
  );
}
