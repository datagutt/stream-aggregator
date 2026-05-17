import Link from "next/link";

import { listCommunitiesRaw } from "@/lib/api";
import { Button } from "@heroui/react";
import { oklchTriplet } from "@/lib/format";
import type { Community } from "@/lib/api-types";

export default async function AdminCommunitiesIndex() {
  let communities: Community[];
  try {
    communities = await listCommunitiesRaw({ noStore: true });
  } catch {
    communities = [];
  }

  return (
    <div className="space-y-6">
      <header className="flex items-center justify-between">
        <div className="space-y-1">
          <p className="text-foreground-dim font-mono text-xs uppercase tracking-wider">
            Communities
          </p>
          <h1 className="text-2xl font-semibold tracking-tight">
            Communities ({communities.length})
          </h1>
        </div>
        <Link href="/admin/communities/new">
          <Button>New community</Button>
        </Link>
      </header>

      {communities.length === 0 ? (
        <div className="border-border bg-surface space-y-2 rounded-lg border p-8 text-center">
          <p className="text-foreground text-base">No communities yet.</p>
          <p className="text-foreground-muted text-sm">
            Set up your first community to start curating streams under your
            own brand.
          </p>
          <div>
            <Link href="/admin/communities/new">
              <Button className="mt-2">Create your first community</Button>
            </Link>
          </div>
        </div>
      ) : (
        <ul className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3">
          {communities.map((c) => (
            <li key={c.slug}>
              <Link
                href={`/admin/communities/${c.slug}`}
                className="border-border bg-surface hover:border-foreground-dim flex items-start gap-3 rounded-lg border p-4 transition-colors"
              >
                <span
                  aria-hidden
                  className="mt-0.5 size-8 shrink-0 rounded-md"
                  style={{ background: oklchTriplet(c.accent, "0.68 0.16 25") }}
                />
                <div className="min-w-0 flex-1">
                  <p className="truncate text-base font-semibold tracking-tight">
                    {c.name}
                  </p>
                  <p className="text-foreground-muted truncate font-mono text-xs">
                    /c/{c.slug}
                  </p>
                  <p className="text-foreground-muted mt-1 text-xs">
                    {c.domains.length} domain{c.domains.length === 1 ? "" : "s"}
                    {c.filter.languages?.length
                      ? ` · ${c.filter.languages.join(", ")}`
                      : ""}
                  </p>
                </div>
              </Link>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
