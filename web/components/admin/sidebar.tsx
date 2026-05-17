"use client";

import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import { Button } from "@heroui/react";

const NAV = [
  { href: "/admin", label: "Overview" },
  { href: "/admin/streamers", label: "Streamers" },
  { href: "/admin/communities", label: "Communities" },
  { href: "/admin/platforms", label: "Platforms" },
];

export function Sidebar() {
  const pathname = usePathname();
  const router = useRouter();

  const signOut = async () => {
    await fetch("/api/admin-login", { method: "DELETE" });
    router.refresh();
  };

  return (
    <aside className="bg-surface border-border flex w-60 shrink-0 flex-col border-r">
      <div className="border-border border-b px-5 py-5">
        <p className="text-foreground-dim font-mono text-[10px] uppercase tracking-wider">
          Stream Aggregator
        </p>
        <p className="text-foreground text-base font-semibold tracking-tight">
          Admin
        </p>
      </div>
      <nav className="flex-1 space-y-0.5 px-2 py-3">
        {NAV.map((item) => {
          const active =
            item.href === "/admin"
              ? pathname === "/admin"
              : pathname.startsWith(item.href);
          return (
            <Link
              key={item.href}
              href={item.href}
              className={
                "block rounded-md px-3 py-2 text-sm transition-colors " +
                (active
                  ? "bg-brand/15 text-foreground"
                  : "text-foreground-muted hover:text-foreground hover:bg-surface-raised")
              }
            >
              {item.label}
            </Link>
          );
        })}
      </nav>
      <div className="border-border border-t p-3">
        <Button size="sm" variant="ghost" className="w-full" onPress={signOut}>
          Sign out
        </Button>
      </div>
    </aside>
  );
}
