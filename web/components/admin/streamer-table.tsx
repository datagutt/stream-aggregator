"use client";

import { useMemo, useState, useTransition } from "react";
import { useRouter } from "next/navigation";
import {
  Button,
  Input,
  Label,
  Popover,
  TextField,
} from "@heroui/react";

import type { TrackedStreamer } from "@/lib/api-types";
import { removeStreamerAction } from "@/app/admin/streamers/actions";

interface Props {
  streamers: TrackedStreamer[];
}

export function StreamerTable({ streamers }: Props) {
  const router = useRouter();
  const [query, setQuery] = useState("");
  const [removing, setRemoving] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return streamers;
    return streamers.filter(
      (s) =>
        s.userId.toLowerCase().includes(q) ||
        s.customName?.toLowerCase().includes(q) ||
        s.platform.toLowerCase().includes(q) ||
        s.group?.toLowerCase().includes(q),
    );
  }, [streamers, query]);

  const remove = (platform: string, userId: string) => {
    const key = `${platform}/${userId}`;
    setRemoving(key);
    startTransition(async () => {
      const res = await removeStreamerAction(platform, userId);
      setRemoving(null);
      if (res.ok) router.refresh();
      else alert(res.error);
    });
  };

  return (
    <div className="space-y-3">
      <TextField className="w-full max-w-sm" value={query} onChange={(v) => setQuery(v)}>
        <Label>Filter</Label>
        <Input placeholder="search this list" />
      </TextField>

      <div className="border-border bg-surface overflow-hidden rounded-lg border">
        <table className="w-full text-sm">
          <thead>
            <tr className="text-foreground-dim border-border border-b text-left font-mono text-[11px] uppercase tracking-wider">
              <th className="px-4 py-2">Platform</th>
              <th className="px-4 py-2">User ID</th>
              <th className="px-4 py-2">Custom name</th>
              <th className="px-4 py-2">Group</th>
              <th className="px-4 py-2">Source</th>
              <th className="px-4 py-2 text-right">Actions</th>
            </tr>
          </thead>
          <tbody>
            {filtered.length === 0 && (
              <tr>
                <td colSpan={6} className="text-foreground-muted px-4 py-6 text-center">
                  {streamers.length === 0
                    ? "No tracked streamers yet."
                    : "No matches."}
                </td>
              </tr>
            )}
            {filtered.map((s) => {
              const key = `${s.platform}/${s.userId}`;
              const isRemoving = removing === key && pending;
              return (
                <tr key={key} className="border-border border-b last:border-b-0">
                  <td className="px-4 py-2 font-medium">{s.platform}</td>
                  <td className="text-foreground-muted px-4 py-2 font-mono text-xs">
                    {s.userId}
                  </td>
                  <td className="px-4 py-2">{s.customName ?? ""}</td>
                  <td className="text-foreground-muted px-4 py-2">{s.group ?? ""}</td>
                  <td className="text-foreground-muted px-4 py-2">{s.source}</td>
                  <td className="px-4 py-2 text-right">
                    <Popover>
                      <Popover.Trigger>
                        <Button size="sm" variant="ghost" isDisabled={isRemoving}>
                          {isRemoving ? "Removing…" : "Remove"}
                        </Button>
                      </Popover.Trigger>
                      <Popover.Content className="w-56 p-3">
                        <p className="text-sm">
                          Remove <strong>{s.customName ?? s.userId}</strong>?
                        </p>
                        <p className="text-foreground-muted mt-1 text-xs">
                          This stops tracking on the backend.
                        </p>
                        <div className="mt-3 flex justify-end gap-2">
                          <Button
                            size="sm"
                            variant="danger"
                            onPress={() => remove(s.platform, s.userId)}
                          >
                            Remove
                          </Button>
                        </div>
                      </Popover.Content>
                    </Popover>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}
