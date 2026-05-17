/**
 * Admin auth helpers. The admin pastes the backend API key into a login
 * form, we validate it against /api/v1/streamers (a cheap auth-gated route),
 * and stash it in an httpOnly cookie. Subsequent server actions read the
 * cookie and forward it as X-API-Key.
 *
 * The key never crosses to the browser as a JS-readable value.
 */

import { cookies } from "next/headers";

export const ADMIN_COOKIE = "admin_key";
/** 30 days. The cookie is just a credential cache, not a session. */
const ADMIN_COOKIE_MAX_AGE = 60 * 60 * 24 * 30;

/** Get the API key from the admin cookie, or null when not signed in. */
export async function getAdminKey(): Promise<string | null> {
  const store = await cookies();
  return store.get(ADMIN_COOKIE)?.value ?? null;
}

/** Persist a new API key in the admin cookie (httpOnly, lax). */
export async function setAdminKey(key: string): Promise<void> {
  const store = await cookies();
  store.set(ADMIN_COOKIE, key, {
    httpOnly: true,
    sameSite: "lax",
    secure: process.env.NODE_ENV === "production",
    path: "/",
    maxAge: ADMIN_COOKIE_MAX_AGE,
  });
}

/** Drop the admin cookie. */
export async function clearAdminKey(): Promise<void> {
  const store = await cookies();
  store.delete(ADMIN_COOKIE);
}

/** Quick boolean check that doesn't actually call the backend. */
export async function hasAdminCookie(): Promise<boolean> {
  return (await getAdminKey()) !== null;
}
