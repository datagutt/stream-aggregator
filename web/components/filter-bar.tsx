"use client";

import { throttle, useQueryStates } from "nuqs";
import {
  Input,
  Label,
  ListBox,
  SearchField,
  Select,
  TextField,
  ToggleButton,
} from "@heroui/react";
import type { Key } from "react-aria-components";

import { communityFilterParsers, sortOptions, type SortOption } from "@/lib/community-search-params";
import type { PlatformInfo } from "@/lib/api-types";

const SORT_LABELS: Record<SortOption, string> = {
  viewers: "Viewers",
  started: "Recently started",
  name: "Name (A-Z)",
};

interface Props {
  /** Platforms the user can pick. The community-level lock has already been
      applied upstream so this list is the allowed superset. */
  availablePlatforms: PlatformInfo[];
}

export function FilterBar({ availablePlatforms }: Props) {
  // shallow:false makes the URL change re-fetch the server component, which
  // re-renders the grid with the new filters. throttle keeps text-input edits
  // from spamming the backend during typing.
  const [filters, setFilters] = useQueryStates(communityFilterParsers, {
    shallow: false,
    history: "replace",
    limitUrlUpdates: throttle(300),
    clearOnDefault: true,
  });

  return (
    <div className="border-border bg-surface flex flex-wrap items-end gap-3 rounded-lg border p-3">
      <SearchField
        className="min-w-[220px] flex-1"
        value={filters.q}
        onChange={(v) => setFilters({ q: v })}
        aria-label="Search streams"
      >
        <Label>Search</Label>
        <SearchField.Group>
          <SearchField.SearchIcon />
          <SearchField.Input placeholder="Title or streamer" />
          <SearchField.ClearButton />
        </SearchField.Group>
      </SearchField>

      <TextField
        className="w-44"
        value={filters.category}
        onChange={(v) => setFilters({ category: v })}
        aria-label="Category"
      >
        <Label>Category</Label>
        <Input placeholder="e.g. Just Chatting" />
      </TextField>

      <Select
        className="w-48"
        selectionMode="single"
        selectedKey={filters.sort}
        onSelectionChange={(key: Key | null) => {
          if (key !== null) setFilters({ sort: String(key) as SortOption });
        }}
      >
        <Label>Sort</Label>
        <Select.Trigger>
          <Select.Value />
          <Select.Indicator />
        </Select.Trigger>
        <Select.Popover>
          <ListBox>
            {sortOptions.map((opt) => (
              <ListBox.Item key={opt} id={opt} textValue={SORT_LABELS[opt]}>
                {SORT_LABELS[opt]}
                <ListBox.ItemIndicator />
              </ListBox.Item>
            ))}
          </ListBox>
        </Select.Popover>
      </Select>

      {availablePlatforms.length > 1 && (
        <fieldset className="flex min-w-0 flex-1 flex-col gap-1.5">
          <legend className="text-foreground-dim font-mono text-[10px] uppercase tracking-wider">
            Platform
          </legend>
          <div className="flex flex-wrap items-center gap-1.5">
            {availablePlatforms.map((p) => {
              const active = filters.platform.includes(p.id);
              return (
                <ToggleButton
                  key={p.id}
                  size="sm"
                  variant={active ? "default" : "ghost"}
                  isSelected={active}
                  onChange={(selected) => {
                    const next = selected
                      ? [...filters.platform, p.id]
                      : filters.platform.filter((id) => id !== p.id);
                    setFilters({ platform: next });
                  }}
                >
                  {p.name}
                </ToggleButton>
              );
            })}
          </div>
        </fieldset>
      )}
    </div>
  );
}
