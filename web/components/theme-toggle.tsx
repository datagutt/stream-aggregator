"use client";

import { useTheme } from "next-themes";
import { useEffect, useState } from "react";

/**
 * Simple icon-less, two-state theme toggle. Sets the `theme` cookie via
 * next-themes' built-in storageKey, so server components see the user
 * preference on the next request (no FOUC).
 */
export function ThemeToggle() {
  const { resolvedTheme, setTheme } = useTheme();
  const [mounted, setMounted] = useState(false);
  useEffect(() => setMounted(true), []);

  const isDark = resolvedTheme !== "light";
  const next = isDark ? "light" : "dark";

  return (
    <button
      type="button"
      aria-label={`Switch to ${next} theme`}
      onClick={() => setTheme(next)}
      className="text-foreground-muted hover:text-foreground border-border focus-visible:ring-brand h-8 rounded-md border px-2 font-mono text-[11px] uppercase tracking-wider transition-colors focus-visible:outline-none focus-visible:ring-2"
    >
      {mounted ? (isDark ? "Dark" : "Light") : "—"}
    </button>
  );
}
