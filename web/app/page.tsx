/**
 * Local-dev community picker. In production, middleware rewrites
 * recognized hostnames to /c/[slug] before this route is ever reached, and
 * unknown hosts go to /_not-configured. This page is mostly useful for
 * developers running on localhost.
 */
export default function Home() {
  return (
    <main className="mx-auto flex min-h-dvh max-w-2xl flex-col justify-center gap-6 px-6 py-16">
      <h1 className="text-4xl font-bold tracking-tight">Stream Directory</h1>
      <p className="text-foreground-muted text-base">
        No communities have been configured yet. Visit{" "}
        <a className="underline" href="/admin">
          /admin
        </a>{" "}
        and set one up to get started.
      </p>
    </main>
  );
}
