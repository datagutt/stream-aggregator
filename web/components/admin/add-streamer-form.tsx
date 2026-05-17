"use client";

import { useState, useTransition } from "react";
import { useRouter } from "next/navigation";
import {
  Button,
  Input,
  Label,
  ListBox,
  Select,
  TextField,
} from "@heroui/react";
import type { Key } from "react-aria-components";

import type { PlatformInfo } from "@/lib/api-types";
import { addStreamerAction } from "@/app/admin/streamers/actions";

interface Props {
  platforms: PlatformInfo[];
}

export function AddStreamerForm({ platforms }: Props) {
  const router = useRouter();
  const [platform, setPlatform] = useState(platforms[0]?.id ?? "twitch");
  const [identifier, setIdentifier] = useState("");
  const [group, setGroup] = useState("");
  const [customName, setCustomName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();

  const submit = (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    if (!identifier.trim()) {
      setError("Username or user ID is required");
      return;
    }
    startTransition(async () => {
      const res = await addStreamerAction({
        platform,
        identifier: identifier.trim(),
        customName: customName.trim() || undefined,
        group: group.trim() || undefined,
      });
      if (res.ok) {
        setIdentifier("");
        setCustomName("");
        setGroup("");
        router.refresh();
      } else {
        setError(res.error);
      }
    });
  };

  return (
    <form
      onSubmit={submit}
      className="border-border bg-surface flex flex-wrap items-end gap-3 rounded-lg border p-4"
    >
      <Select
        className="w-40"
        selectionMode="single"
        selectedKey={platform}
        onSelectionChange={(key: Key | null) => {
          if (key) setPlatform(String(key));
        }}
      >
        <Label>Platform</Label>
        <Select.Trigger>
          <Select.Value />
          <Select.Indicator />
        </Select.Trigger>
        <Select.Popover>
          <ListBox>
            {platforms.map((p) => (
              <ListBox.Item key={p.id} id={p.id} textValue={p.name}>
                {p.name}
                <ListBox.ItemIndicator />
              </ListBox.Item>
            ))}
          </ListBox>
        </Select.Popover>
      </Select>

      <TextField
        className="min-w-[200px] flex-1"
        value={identifier}
        onChange={(v) => setIdentifier(v)}
        isRequired
      >
        <Label>Username or user ID</Label>
        <Input placeholder="e.g. shroud" autoComplete="off" />
      </TextField>

      <TextField className="w-40" value={group} onChange={(v) => setGroup(v)}>
        <Label>Group (optional)</Label>
        <Input placeholder="e.g. norwegian" />
      </TextField>

      <TextField
        className="w-44"
        value={customName}
        onChange={(v) => setCustomName(v)}
      >
        <Label>Display override</Label>
        <Input placeholder="optional" />
      </TextField>

      <Button type="submit" isDisabled={pending || !identifier.trim()}>
        {pending ? "Adding…" : "Add"}
      </Button>

      {error && (
        <p role="alert" className="basis-full text-sm text-[oklch(0.65_0.2_25)]">
          {error}
        </p>
      )}
    </form>
  );
}
