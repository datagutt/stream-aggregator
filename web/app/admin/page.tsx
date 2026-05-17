import { listPlatforms, listStreamers, listStreams } from "@/lib/api";
import { getAdminKey } from "@/lib/auth";

interface StatCardProps {
  label: string;
  value: string | number;
  hint?: string;
}

function StatCard({ label, value, hint }: StatCardProps) {
  return (
    <div className="border-border bg-surface rounded-lg border p-4">
      <p className="text-foreground-dim font-mono text-[10px] uppercase tracking-wider">
        {label}
      </p>
      <p className="text-foreground mt-1 text-2xl font-semibold tabular-nums">
        {value}
      </p>
      {hint && <p className="text-foreground-muted mt-1 text-xs">{hint}</p>}
    </div>
  );
}

export default async function AdminOverview() {
  const apiKey = (await getAdminKey()) ?? undefined;

  // Parallel fetches: total live count, total streams count, all tracked
  // streamers (for per-platform counts), supported platforms. Pass apiKey
  // to listStreamers because GET /streamers requires auth when enabled.
  const [livePage, allPage, streamers, platforms] = await Promise.all([
    listStreams({ isLive: true, pageSize: 1 }, { noStore: true }),
    listStreams({ pageSize: 1 }, { noStore: true }),
    listStreamers({ noStore: true, apiKey }),
    listPlatforms({ revalidate: 60 }),
  ]);

  const trackedByPlatform = new Map<string, number>();
  for (const s of streamers) {
    trackedByPlatform.set(s.platform, (trackedByPlatform.get(s.platform) ?? 0) + 1);
  }

  return (
    <div className="space-y-8">
      <header className="space-y-1">
        <p className="text-foreground-dim font-mono text-xs uppercase tracking-wider">
          Overview
        </p>
        <h1 className="text-2xl font-semibold tracking-tight">Service health</h1>
      </header>

      <section className="grid grid-cols-1 gap-3 sm:grid-cols-3">
        <StatCard label="Tracked streamers" value={streamers.length} />
        <StatCard label="Live now" value={livePage.pagination.total} />
        <StatCard
          label="Known stream records"
          value={allPage.pagination.total}
          hint="includes offline"
        />
      </section>

      <section className="space-y-2">
        <h2 className="text-foreground-muted font-mono text-xs uppercase tracking-wider">
          By platform
        </h2>
        <div className="border-border bg-surface overflow-hidden rounded-lg border">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-foreground-dim border-border border-b text-left font-mono text-[11px] uppercase tracking-wider">
                <th className="px-4 py-2">Platform</th>
                <th className="px-4 py-2">Tracked</th>
                <th className="px-4 py-2">Supports discovery</th>
              </tr>
            </thead>
            <tbody>
              {platforms.map((p) => (
                <tr key={p.id} className="border-border border-b last:border-b-0">
                  <td className="px-4 py-2 font-medium">{p.name}</td>
                  <td className="px-4 py-2 tabular-nums">
                    {trackedByPlatform.get(p.id) ?? 0}
                  </td>
                  <td className="text-foreground-muted px-4 py-2">
                    {p.supportsDiscovery ? "yes" : "no"}
                  </td>
                </tr>
              ))}
              {platforms.length === 0 && (
                <tr>
                  <td colSpan={3} className="text-foreground-muted px-4 py-4 text-center">
                    No platforms configured.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </section>
    </div>
  );
}
