/**
 * Color helpers for the admin community editor.
 *
 * Communities store accents as OKLCH triplets ("L C H") because that's what
 * DESIGN.md mandates. Admins typically have a hex color in hand (from a brand
 * book), so we convert at the form boundary using culori.
 */

import { converter, formatHex } from "culori";

const toOklch = converter("oklch");
const toRgb = converter("rgb");

/** Convert a #rrggbb hex to an OKLCH triplet "L C H". Returns null on invalid input. */
export function hexToOklchTriplet(hex: string): string | null {
  const cleaned = hex.trim();
  if (!/^#?[0-9a-fA-F]{6}$/.test(cleaned.replace(/^#/, ""))) return null;
  const normalized = cleaned.startsWith("#") ? cleaned : `#${cleaned}`;
  const c = toOklch(normalized);
  if (!c) return null;
  const L = (c.l ?? 0).toFixed(3);
  const C = (c.c ?? 0).toFixed(3);
  // OKLCH hue is undefined for true greys. Default to 0.
  const H = (c.h ?? 0).toFixed(2);
  return `${L} ${C} ${H}`;
}

/** Convert an OKLCH triplet back to a hex string for previewing. */
export function oklchTripletToHex(triplet: string): string | null {
  const parts = triplet.trim().split(/\s+/);
  if (parts.length !== 3) return null;
  const [lStr, cStr, hStr] = parts;
  const l = Number(lStr);
  const c = Number(cStr);
  const h = Number(hStr);
  if (![l, c, h].every(Number.isFinite)) return null;
  const rgb = toRgb({ mode: "oklch", l, c, h });
  if (!rgb) return null;
  return formatHex(rgb);
}

/** Curated swatches — restrained, warm-leaning, avoiding gaming-purple. */
export const CURATED_SWATCHES: { hex: string; label: string }[] = [
  { hex: "#3b82f6", label: "Nordic blue" },
  { hex: "#f59e0b", label: "Amber" },
  { hex: "#10b981", label: "Emerald" },
  { hex: "#ef4444", label: "Coral" },
  { hex: "#8b5cf6", label: "Iris" },
  { hex: "#ec4899", label: "Magenta" },
  { hex: "#14b8a6", label: "Teal" },
  { hex: "#f97316", label: "Tangerine" },
];

/** Build a high-contrast OKLCH triplet for foreground text on top of an accent. */
export function inferAccentContrast(accent: string): string | null {
  const parts = accent.trim().split(/\s+/);
  if (parts.length !== 3) return null;
  const l = Number(parts[0]);
  if (!Number.isFinite(l)) return null;
  // Light accent gets dark text, dark accent gets light text. Keep chroma low.
  return l > 0.7 ? `0.18 0.01 ${parts[2]}` : `0.98 0.01 ${parts[2]}`;
}
