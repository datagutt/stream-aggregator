"use client";

import { ThemeProvider } from "next-themes";
import { NuqsAdapter } from "nuqs/adapters/next/app";
import type { ReactNode } from "react";

/**
 * Client-side providers wrapper.
 *
 * - next-themes drives the dark/light class on <html>.
 * - NuqsAdapter wires URL query state for client components.
 * - HeroUI v3 needs no provider.
 *
 * Brand tokens are injected by the community layout as inline styles.
 */
export function Providers({ children }: { children: ReactNode }) {
  return (
    <ThemeProvider
      attribute="class"
      defaultTheme="dark"
      enableSystem={false}
      storageKey="theme"
    >
      <NuqsAdapter>{children}</NuqsAdapter>
    </ThemeProvider>
  );
}
