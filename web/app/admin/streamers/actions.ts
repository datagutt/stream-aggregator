"use server";

import { revalidatePath } from "next/cache";

import { addStreamer as apiAddStreamer, removeStreamer as apiRemoveStreamer } from "@/lib/api";
import { getAdminKey } from "@/lib/auth";
import type { AddStreamerInput } from "@/lib/api-types";

interface AddInput {
  platform: string;
  /** Either user_id or username; the backend's add_streamer accepts either. */
  identifier: string;
  customName?: string;
  group?: string;
  labels?: Record<string, string>;
}

export async function addStreamerAction(
  input: AddInput,
): Promise<{ ok: true } | { ok: false; error: string }> {
  const apiKey = await getAdminKey();
  if (!apiKey) return { ok: false, error: "Not signed in" };

  const payload: AddStreamerInput = {
    platform: input.platform,
    // Treat anything that's purely digits as a user_id; otherwise pass as a
    // username and let the provider resolve it.
    ...(/^\d+$/.test(input.identifier)
      ? { userId: input.identifier }
      : { username: input.identifier }),
    customName: input.customName?.trim() || undefined,
    group: input.group?.trim() || undefined,
    labels: input.labels && Object.keys(input.labels).length ? input.labels : undefined,
  };

  try {
    await apiAddStreamer(payload, { apiKey });
    revalidatePath("/admin/streamers");
    revalidatePath("/admin");
    return { ok: true };
  } catch (e) {
    return { ok: false, error: (e as Error).message };
  }
}

export async function removeStreamerAction(
  platform: string,
  userId: string,
): Promise<{ ok: true } | { ok: false; error: string }> {
  const apiKey = await getAdminKey();
  if (!apiKey) return { ok: false, error: "Not signed in" };

  try {
    await apiRemoveStreamer(platform, userId, { apiKey });
    revalidatePath("/admin/streamers");
    revalidatePath("/admin");
    return { ok: true };
  } catch (e) {
    return { ok: false, error: (e as Error).message };
  }
}
