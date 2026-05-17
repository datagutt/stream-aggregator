import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Not configured",
  robots: { index: false, follow: false },
};

export default function NotConfigured() {
  return (
    <main className="mx-auto flex min-h-dvh max-w-xl flex-col justify-center gap-4 px-6 py-16">
      <p className="text-foreground-dim font-mono text-xs uppercase tracking-wider">
        Hostname not configured
      </p>
      <h1 className="text-3xl font-semibold tracking-tight">
        This domain isn&apos;t connected to a community.
      </h1>
      <p className="text-foreground-muted text-base">
        Visit the admin to attach this hostname to a community, or use one of
        the community&apos;s known domains.
      </p>
    </main>
  );
}
