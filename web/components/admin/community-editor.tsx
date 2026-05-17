"use client";

import { useMemo, useState, useTransition } from "react";
import { useRouter } from "next/navigation";
import {
  Button,
  Input,
  Label,
  ListBox,
  Popover,
  Select,
  TextField,
  ToggleButton,
} from "@heroui/react";
import type { Key } from "react-aria-components";

import type { Community, UpsertCommunityInput, ThemeMode } from "@/lib/api-types";
import {
  CURATED_SWATCHES,
  hexToOklchTriplet,
  inferAccentContrast,
  oklchTripletToHex,
} from "@/lib/color";
import {
  createCommunityAction,
  deleteCommunityAction,
  updateCommunityAction,
} from "@/app/admin/communities/actions";

interface Props {
  /** Provided when editing an existing community. */
  initial?: Community;
  /** All platform IDs the backend knows about; used for the chip multi-select. */
  platformIds: string[];
}

// Minimal ISO 639-1 set; admin can add more by typing in the language chip
// input (it accepts any 2-letter code).
const KNOWN_LANGUAGES = [
  { code: "en", label: "English" },
  { code: "no", label: "Norwegian" },
  { code: "sv", label: "Swedish" },
  { code: "da", label: "Danish" },
  { code: "fi", label: "Finnish" },
  { code: "de", label: "German" },
  { code: "fr", label: "French" },
  { code: "es", label: "Spanish" },
  { code: "ja", label: "Japanese" },
  { code: "ko", label: "Korean" },
  { code: "pt", label: "Portuguese" },
  { code: "nl", label: "Dutch" },
];

export function CommunityEditor({ initial, platformIds }: Props) {
  const router = useRouter();
  const editing = !!initial;

  const initialAccentHex = initial ? oklchTripletToHex(initial.accent) ?? "#3b82f6" : "#3b82f6";

  const [slug, setSlug] = useState(initial?.slug ?? "");
  const [name, setName] = useState(initial?.name ?? "");
  const [tagline, setTagline] = useState(initial?.tagline ?? "");
  const [accentHex, setAccentHex] = useState(initialAccentHex);
  const [logoUrl, setLogoUrl] = useState(initial?.logoUrl ?? "");
  const [defaultTheme, setDefaultTheme] = useState<ThemeMode>(initial?.defaultTheme ?? "dark");
  const [domains, setDomains] = useState<string[]>(initial?.domains ?? []);
  const [domainDraft, setDomainDraft] = useState("");

  const [platforms, setPlatforms] = useState<string[]>(initial?.filter.platforms ?? []);
  const [languages, setLanguages] = useState<string[]>(initial?.filter.languages ?? []);
  const [languageDraft, setLanguageDraft] = useState("");
  const [categories, setCategories] = useState<string[]>(initial?.filter.categories ?? []);
  const [categoryDraft, setCategoryDraft] = useState("");
  const [tags, setTags] = useState<string[]>(initial?.filter.tags ?? []);
  const [tagDraft, setTagDraft] = useState("");
  const [labels, setLabels] = useState<Record<string, string>>(
    initial?.filter.labels ?? {},
  );
  const [labelK, setLabelK] = useState("");
  const [labelV, setLabelV] = useState("");

  const [aboutMd, setAboutMd] = useState(initial?.aboutMd ?? "");
  const [error, setError] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();

  const accentTriplet = useMemo(
    () => hexToOklchTriplet(accentHex) ?? "0.68 0.16 25",
    [accentHex],
  );

  const buildPayload = (): UpsertCommunityInput => ({
    slug: slug.trim(),
    name: name.trim(),
    tagline: tagline.trim() || null,
    accent: accentTriplet,
    accentContrast: inferAccentContrast(accentTriplet),
    logoUrl: logoUrl.trim() || null,
    defaultTheme,
    domains: domains.filter((d) => d.trim() !== ""),
    filter: {
      platforms,
      languages,
      categories,
      tags,
      labels,
    },
    aboutMd: aboutMd.trim() || null,
  });

  const submit = (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    if (!slug.trim() || !/^[a-z0-9-]+$/.test(slug.trim())) {
      setError("Slug must be lowercase letters, numbers, and dashes only.");
      return;
    }
    if (!name.trim()) {
      setError("Name is required.");
      return;
    }
    startTransition(async () => {
      const payload = buildPayload();
      const res = editing
        ? await updateCommunityAction(initial!.slug, payload)
        : await createCommunityAction(payload);
      if (res.ok) {
        router.push(`/admin/communities/${res.data.slug}`);
        router.refresh();
      } else {
        setError(res.error);
      }
    });
  };

  const remove = () => {
    if (!editing) return;
    startTransition(async () => {
      const res = await deleteCommunityAction(initial!.slug);
      if (res.ok) {
        router.push("/admin/communities");
        router.refresh();
      } else {
        setError(res.error);
      }
    });
  };

  const addChip = (
    list: string[],
    set: (n: string[]) => void,
    draft: string,
    setDraft: (s: string) => void,
  ) => {
    const v = draft.trim();
    if (!v || list.includes(v)) {
      setDraft("");
      return;
    }
    set([...list, v]);
    setDraft("");
  };

  return (
    <form onSubmit={submit} className="space-y-8">
      {/* Brand block ───────────────────────────────────────────────── */}
      <section className="space-y-4">
        <h2 className="text-foreground-muted font-mono text-xs uppercase tracking-wider">
          Brand
        </h2>

        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
          <TextField value={slug} onChange={(v) => setSlug(v)} isDisabled={editing} isRequired>
            <Label>Slug</Label>
            <Input placeholder="livestreamnorge" autoComplete="off" />
          </TextField>
          <TextField value={name} onChange={(v) => setName(v)} isRequired>
            <Label>Name</Label>
            <Input placeholder="LiveStreamNorge" autoComplete="off" />
          </TextField>
        </div>

        <TextField value={tagline} onChange={(v) => setTagline(v)}>
          <Label>Tagline</Label>
          <Input placeholder="Norske strømmere live nå" />
        </TextField>

        <TextField value={logoUrl} onChange={(v) => setLogoUrl(v)}>
          <Label>Logo URL (optional)</Label>
          <Input placeholder="https://…" autoComplete="off" />
        </TextField>

        <div className="space-y-2">
          <Label>Accent color</Label>
          <div className="flex flex-wrap items-center gap-2">
            <input
              type="color"
              value={accentHex}
              onChange={(e) => setAccentHex(e.target.value)}
              aria-label="Pick a color"
              className="border-border h-9 w-14 cursor-pointer rounded-md border bg-transparent"
            />
            <input
              type="text"
              value={accentHex}
              onChange={(e) => setAccentHex(e.target.value)}
              aria-label="Hex"
              className="border-border bg-background text-foreground focus-visible:ring-brand h-9 w-28 rounded-md border px-2 font-mono text-sm focus-visible:outline-none focus-visible:ring-2"
            />
            <span className="text-foreground-dim font-mono text-xs">
              → oklch({accentTriplet})
            </span>
          </div>
          <div className="flex flex-wrap gap-1.5">
            {CURATED_SWATCHES.map((s) => (
              <button
                key={s.hex}
                type="button"
                aria-label={s.label}
                onClick={() => setAccentHex(s.hex)}
                className="size-6 rounded-full ring-1 ring-inset ring-black/10 transition-transform hover:scale-110"
                style={{ background: s.hex }}
              />
            ))}
          </div>
        </div>

        <Select
          className="w-48"
          selectionMode="single"
          selectedKey={defaultTheme}
          onSelectionChange={(k: Key | null) => {
            if (k) setDefaultTheme(String(k) as ThemeMode);
          }}
        >
          <Label>Default theme</Label>
          <Select.Trigger>
            <Select.Value />
            <Select.Indicator />
          </Select.Trigger>
          <Select.Popover>
            <ListBox>
              <ListBox.Item id="dark" textValue="Dark">
                Dark
                <ListBox.ItemIndicator />
              </ListBox.Item>
              <ListBox.Item id="light" textValue="Light">
                Light
                <ListBox.ItemIndicator />
              </ListBox.Item>
            </ListBox>
          </Select.Popover>
        </Select>
      </section>

      {/* Domains ───────────────────────────────────────────────────── */}
      <section className="space-y-3">
        <h2 className="text-foreground-muted font-mono text-xs uppercase tracking-wider">
          Domains
        </h2>
        <p className="text-foreground-muted text-sm">
          Hostnames that map to this community. The proxy resolves Host →
          slug from this list.
        </p>
        <div className="flex flex-wrap items-center gap-1.5">
          {domains.map((d) => (
            <span
              key={d}
              className="border-border bg-surface-raised inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-xs"
            >
              <code>{d}</code>
              <button
                type="button"
                aria-label={`Remove ${d}`}
                onClick={() => setDomains(domains.filter((x) => x !== d))}
                className="text-foreground-dim hover:text-foreground"
              >
                ×
              </button>
            </span>
          ))}
          <input
            type="text"
            value={domainDraft}
            onChange={(e) => setDomainDraft(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === ",") {
                e.preventDefault();
                addChip(domains, setDomains, domainDraft, setDomainDraft);
              }
            }}
            placeholder="add hostname, press enter"
            className="border-border bg-background text-foreground placeholder:text-foreground-dim focus-visible:ring-brand h-7 min-w-[180px] flex-1 rounded-md border px-2 text-xs focus-visible:outline-none focus-visible:ring-2"
          />
        </div>
      </section>

      {/* Filter recipe ─────────────────────────────────────────────── */}
      <section className="space-y-4">
        <h2 className="text-foreground-muted font-mono text-xs uppercase tracking-wider">
          Filter recipe
        </h2>

        <div className="space-y-2">
          <Label>Platforms (empty = all)</Label>
          <div className="flex flex-wrap gap-1.5">
            {platformIds.map((id) => {
              const active = platforms.includes(id);
              return (
                <ToggleButton
                  key={id}
                  size="sm"
                  variant={active ? "default" : "ghost"}
                  isSelected={active}
                  onChange={(selected) =>
                    setPlatforms(
                      selected ? [...platforms, id] : platforms.filter((p) => p !== id),
                    )
                  }
                >
                  {id}
                </ToggleButton>
              );
            })}
          </div>
        </div>

        <ChipField
          label="Languages (ISO 639-1, e.g. no, sv, en)"
          chips={languages}
          setChips={setLanguages}
          draft={languageDraft}
          setDraft={setLanguageDraft}
          suggestions={KNOWN_LANGUAGES.map((l) => l.code)}
        />

        <ChipField
          label="Categories"
          chips={categories}
          setChips={setCategories}
          draft={categoryDraft}
          setDraft={setCategoryDraft}
          placeholder='e.g. "Just Chatting"'
        />

        <ChipField
          label="Tags"
          chips={tags}
          setChips={setTags}
          draft={tagDraft}
          setDraft={setTagDraft}
        />

        <div className="space-y-2">
          <Label>Labels (key = value)</Label>
          <div className="flex flex-wrap gap-1.5">
            {Object.entries(labels).map(([k, v]) => (
              <span
                key={k}
                className="border-border bg-surface-raised inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-xs"
              >
                <code>
                  {k}={v}
                </code>
                <button
                  type="button"
                  aria-label={`Remove ${k}`}
                  onClick={() => {
                    const next = { ...labels };
                    delete next[k];
                    setLabels(next);
                  }}
                  className="text-foreground-dim hover:text-foreground"
                >
                  ×
                </button>
              </span>
            ))}
          </div>
          <div className="flex gap-2">
            <input
              type="text"
              value={labelK}
              onChange={(e) => setLabelK(e.target.value)}
              placeholder="key"
              className="border-border bg-background text-foreground placeholder:text-foreground-dim focus-visible:ring-brand h-9 w-32 rounded-md border px-2 text-sm focus-visible:outline-none focus-visible:ring-2"
            />
            <input
              type="text"
              value={labelV}
              onChange={(e) => setLabelV(e.target.value)}
              placeholder="value"
              className="border-border bg-background text-foreground placeholder:text-foreground-dim focus-visible:ring-brand h-9 w-40 rounded-md border px-2 text-sm focus-visible:outline-none focus-visible:ring-2"
            />
            <Button
              size="sm"
              variant="ghost"
              onPress={() => {
                const k = labelK.trim();
                const v = labelV.trim();
                if (!k || !v) return;
                setLabels({ ...labels, [k]: v });
                setLabelK("");
                setLabelV("");
              }}
            >
              Add label
            </Button>
          </div>
        </div>
      </section>

      {/* About ─────────────────────────────────────────────────────── */}
      <section className="space-y-2">
        <Label>About (markdown)</Label>
        <textarea
          value={aboutMd}
          onChange={(e) => setAboutMd(e.target.value)}
          rows={6}
          placeholder="Optional. Shown on /c/[slug]/about."
          className="border-border bg-background text-foreground placeholder:text-foreground-dim focus-visible:ring-brand w-full rounded-md border px-3 py-2 text-sm focus-visible:outline-none focus-visible:ring-2"
        />
      </section>

      {error && (
        <p role="alert" className="text-sm text-[oklch(0.65_0.2_25)]">
          {error}
        </p>
      )}

      <div className="flex items-center gap-3">
        <Button type="submit" isDisabled={pending}>
          {pending ? "Saving…" : editing ? "Save changes" : "Create community"}
        </Button>
        {editing && (
          <Popover>
            <Popover.Trigger>
              <Button variant="ghost" isDisabled={pending}>
                Delete…
              </Button>
            </Popover.Trigger>
            <Popover.Content className="w-64 p-3">
              <p className="text-sm">
                Delete community <strong>{initial?.slug}</strong>?
              </p>
              <p className="text-foreground-muted mt-1 text-xs">
                Removes the community and its domain mappings. Cannot be undone.
              </p>
              <div className="mt-3 flex justify-end">
                <Button size="sm" variant="danger" onPress={remove}>
                  Delete
                </Button>
              </div>
            </Popover.Content>
          </Popover>
        )}
      </div>
    </form>
  );
}

interface ChipFieldProps {
  label: string;
  chips: string[];
  setChips: (next: string[]) => void;
  draft: string;
  setDraft: (s: string) => void;
  placeholder?: string;
  suggestions?: string[];
}

function ChipField({
  label,
  chips,
  setChips,
  draft,
  setDraft,
  placeholder,
  suggestions,
}: ChipFieldProps) {
  const commit = () => {
    const v = draft.trim();
    if (!v || chips.includes(v)) {
      setDraft("");
      return;
    }
    setChips([...chips, v]);
    setDraft("");
  };
  return (
    <div className="space-y-2">
      <Label>{label}</Label>
      <div className="flex flex-wrap gap-1.5">
        {chips.map((c) => (
          <span
            key={c}
            className="border-border bg-surface-raised inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-xs"
          >
            <code>{c}</code>
            <button
              type="button"
              aria-label={`Remove ${c}`}
              onClick={() => setChips(chips.filter((x) => x !== c))}
              className="text-foreground-dim hover:text-foreground"
            >
              ×
            </button>
          </span>
        ))}
        <input
          type="text"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === ",") {
              e.preventDefault();
              commit();
            }
          }}
          list={suggestions ? `${label}-sugg` : undefined}
          placeholder={placeholder ?? "press enter to add"}
          className="border-border bg-background text-foreground placeholder:text-foreground-dim focus-visible:ring-brand h-7 min-w-[160px] flex-1 rounded-md border px-2 text-xs focus-visible:outline-none focus-visible:ring-2"
        />
        {suggestions && (
          <datalist id={`${label}-sugg`}>
            {suggestions.map((s) => (
              <option key={s} value={s} />
            ))}
          </datalist>
        )}
      </div>
    </div>
  );
}
