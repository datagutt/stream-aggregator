"use server";

import { revalidatePath } from "next/cache";

import {
  createCommunity as apiCreate,
  updateCommunity as apiUpdate,
  deleteCommunity as apiDelete,
} from "@/lib/api";
import { invalidateCommunityCache } from "@/lib/communities";
import { getAdminKey } from "@/lib/auth";
import type { UpsertCommunityInput } from "@/lib/api-types";

type ActionResult<T> = { ok: true; data: T } | { ok: false; error: string };

export async function createCommunityAction(
  input: UpsertCommunityInput,
): Promise<ActionResult<{ slug: string }>> {
  const apiKey = await getAdminKey();
  if (!apiKey) return { ok: false, error: "Not signed in" };
  try {
    const community = await apiCreate(input, { apiKey });
    invalidateCommunityCache();
    revalidatePath("/admin/communities");
    return { ok: true, data: { slug: community.slug } };
  } catch (e) {
    return { ok: false, error: (e as Error).message };
  }
}

export async function updateCommunityAction(
  slug: string,
  input: UpsertCommunityInput,
): Promise<ActionResult<{ slug: string }>> {
  const apiKey = await getAdminKey();
  if (!apiKey) return { ok: false, error: "Not signed in" };
  try {
    const community = await apiUpdate(slug, input, { apiKey });
    invalidateCommunityCache();
    revalidatePath("/admin/communities");
    revalidatePath(`/admin/communities/${slug}`);
    revalidatePath(`/c/${slug}`);
    return { ok: true, data: { slug: community.slug } };
  } catch (e) {
    return { ok: false, error: (e as Error).message };
  }
}

export async function deleteCommunityAction(
  slug: string,
): Promise<ActionResult<null>> {
  const apiKey = await getAdminKey();
  if (!apiKey) return { ok: false, error: "Not signed in" };
  try {
    await apiDelete(slug, { apiKey });
    invalidateCommunityCache();
    revalidatePath("/admin/communities");
    return { ok: true, data: null };
  } catch (e) {
    return { ok: false, error: (e as Error).message };
  }
}
