import { listPlatforms } from "@/lib/api";

export default async function AdminPlatforms() {
  const platforms = await listPlatforms({ noStore: true });

  return (
    <div className="space-y-6">
      <header className="space-y-1">
        <p className="text-foreground-dim font-mono text-xs uppercase tracking-wider">
          Platforms
        </p>
        <h1 className="text-2xl font-semibold tracking-tight">
          Supported platforms ({platforms.length})
        </h1>
        <p className="text-foreground-muted text-sm">
          Read-only. Configure which platforms run in <code>config.toml</code>{" "}
          on the backend. A detailed health endpoint will arrive in a future
          backend release.
        </p>
      </header>

      <div className="border-border bg-surface overflow-hidden rounded-lg border">
        <table className="w-full text-sm">
          <thead>
            <tr className="text-foreground-dim border-border border-b text-left font-mono text-[11px] uppercase tracking-wider">
              <th className="px-4 py-2">ID</th>
              <th className="px-4 py-2">Name</th>
              <th className="px-4 py-2">Base URL</th>
              <th className="px-4 py-2">Discovery</th>
            </tr>
          </thead>
          <tbody>
            {platforms.map((p) => (
              <tr key={p.id} className="border-border border-b last:border-b-0">
                <td className="px-4 py-2 font-mono text-xs">{p.id}</td>
                <td className="px-4 py-2 font-medium">{p.name}</td>
                <td className="text-foreground-muted px-4 py-2 truncate">
                  <a
                    href={p.baseUrl}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="hover:text-foreground underline"
                  >
                    {p.baseUrl}
                  </a>
                </td>
                <td className="text-foreground-muted px-4 py-2">
                  {p.supportsDiscovery ? "supported" : "—"}
                </td>
              </tr>
            ))}
            {platforms.length === 0 && (
              <tr>
                <td colSpan={4} className="text-foreground-muted px-4 py-6 text-center">
                  No platforms configured.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
