import Link from "next/link";
import { redirect } from "next/navigation";

import { listCommunities } from "@/lib/communities";
import { oklchTriplet } from "@/lib/format";
import type { Community } from "@/lib/api-types";

/**
 * Local-dev / unmatched-host community picker. In production, recognized
 * hosts are rewritten to /c/[slug] by proxy.ts before this route renders,
 * and unknown hosts go to /_not-configured. This page is reached only when
 * a developer hits localhost.
 *
 * If DEFAULT_COMMUNITY is set, we auto-redirect into it.
 */
export default async function Home() {
  const fallback = process.env.DEFAULT_COMMUNITY;
  if (fallback) redirect(`/c/${fallback}`);

  let communities: Community[];
  try {
    communities = await listCommunities();
  } catch {
    communities = [];
  }

  if (communities.length === 1) {
    redirect(`/c/${communities[0].slug}`);
  }

  return (
    <main className="mx-auto flex min-h-dvh max-w-3xl flex-col gap-8 px-6 py-16">
      <header className="space-y-2">
        <p className="text-foreground-dim font-mono text-xs uppercase tracking-wider">
          Stream Directory
        </p>
        <h1 className="text-3xl font-bold tracking-tight">Pick a community</h1>
      </header>

      {communities.length === 0 ? (
        <section className="border-border bg-surface rounded-lg border p-6">
          <p className="text-foreground-muted text-sm">
            No communities configured yet.{" "}
            <Link href="/admin" className="underline">
              Set one up in the admin
            </Link>{" "}
            to get started.
          </p>
        </section>
      ) : (
        <ul className="grid grid-cols-1 gap-3 sm:grid-cols-2">
          {communities.map((c) => (
            <li key={c.slug}>
              <Link
                href={`/c/${c.slug}`}
                className="border-border bg-surface hover:border-foreground-dim flex items-center gap-3 rounded-lg border p-4 transition-colors"
              >
                {c.logoUrl ? (
                  // eslint-disable-next-line @next/next/no-img-element
                  <img
                    src={c.logoUrl}
                    alt=""
                    className="size-10 shrink-0 rounded-md object-cover"
                  />
                ) : (
                  <span
                    aria-hidden
                    className="size-10 shrink-0 rounded-md"
                    style={{ background: oklchTriplet(c.accent, "0.68 0.16 25") }}
                  />
                )}
                <div className="min-w-0 flex-1">
                  <p className="truncate text-base font-semibold tracking-tight">
                    {c.name}
                  </p>
                  {c.tagline && (
                    <p className="text-foreground-muted truncate text-sm">
                      {c.tagline}
                    </p>
                  )}
                </div>
              </Link>
            </li>
          ))}
        </ul>
      )}
    </main>
  );
}
