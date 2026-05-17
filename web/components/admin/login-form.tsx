"use client";

import { useState, useTransition } from "react";
import { useRouter } from "next/navigation";
import { Button, Input, Label, TextField } from "@heroui/react";

/**
 * Single-field admin login. The user pastes the backend API key; we POST
 * it to /api/admin-login which validates it and sets the cookie.
 */
export function LoginForm() {
  const router = useRouter();
  const [key, setKey] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();

  const submit = (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    startTransition(async () => {
      const res = await fetch("/api/admin-login", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ key }),
      });
      if (res.ok) {
        router.refresh();
        return;
      }
      const body = (await res.json().catch(() => ({}))) as { error?: string };
      setError(body.error ?? `Login failed (${res.status})`);
    });
  };

  return (
    <main className="mx-auto flex min-h-dvh max-w-md flex-col justify-center gap-6 px-6 py-16">
      <header className="space-y-2">
        <p className="text-foreground-dim font-mono text-xs uppercase tracking-wider">
          Admin
        </p>
        <h1 className="text-3xl font-bold tracking-tight">Sign in</h1>
        <p className="text-foreground-muted text-sm">
          Paste a backend API key (from <code>API_KEYS</code> in the server
          config) to access the admin.
        </p>
      </header>

      <form onSubmit={submit} className="space-y-4">
        <TextField
          value={key}
          onChange={(v) => setKey(v)}
          isRequired
          autoFocus
        >
          <Label>API key</Label>
          <Input type="password" placeholder="paste your key" autoComplete="off" />
        </TextField>
        {error && (
          <p role="alert" className="text-sm text-[oklch(0.65_0.2_25)]">
            {error}
          </p>
        )}
        <Button type="submit" isDisabled={pending || !key.trim()} className="w-full">
          {pending ? "Validating…" : "Sign in"}
        </Button>
      </form>
    </main>
  );
}
