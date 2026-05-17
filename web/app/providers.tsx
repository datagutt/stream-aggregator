"use client";

import { ThemeProvider } from "next-themes";
import type { ReactNode } from "react";

/**
 * Client-side providers wrapper. next-themes drives the dark/light class on
 * <html>. HeroUI v3 needs no provider. Brand tokens are injected by the
 * community layout as inline styles on <html>.
 */
export function Providers({ children }: { children: ReactNode }) {
  return (
    <ThemeProvider
      attribute="class"
      defaultTheme="dark"
      enableSystem={false}
      storageKey="theme"
    >
      {children}
    </ThemeProvider>
  );
}
