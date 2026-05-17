import { NextResponse, type NextRequest } from "next/server";

import { ADMIN_COOKIE } from "@/lib/auth";

const API_URL =
  process.env.STREAM_AGGREGATOR_API_URL ??
  process.env.NEXT_PUBLIC_API_URL ??
  "http://localhost:8080";

/**
 * Validate an API key against the backend by calling a known-auth-gated
 * route (POST /streamers requires auth when enabled; if it returns 400
 * Bad Request rather than 401, the key is valid and the route just rejected
 * an empty body. Either way, 401 means the key is wrong).
 *
 * On success, set the httpOnly cookie. The cookie is read by server actions
 * later and forwarded as X-API-Key on writes.
 */
export async function POST(req: NextRequest) {
  let body: unknown;
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: "Expected JSON body" }, { status: 400 });
  }
  if (!body || typeof body !== "object" || !("key" in body)) {
    return NextResponse.json({ error: "Missing 'key'" }, { status: 400 });
  }
  const key = String((body as { key: unknown }).key ?? "").trim();
  if (!key) {
    return NextResponse.json({ error: "Empty 'key'" }, { status: 400 });
  }

  let upstream: Response;
  try {
    upstream = await fetch(`${API_URL}/api/v1/streamers`, {
      method: "POST",
      headers: { "Content-Type": "application/json", "X-API-Key": key },
      // empty body — backend should reject as Bad Request (400) when the
      // key is valid, or Unauthorized (401) when the key is wrong.
      body: JSON.stringify({}),
      cache: "no-store",
    });
  } catch (e) {
    return NextResponse.json(
      { error: `Could not reach backend at ${API_URL}: ${(e as Error).message}` },
      { status: 502 },
    );
  }

  if (upstream.status === 401) {
    return NextResponse.json({ error: "Invalid API key" }, { status: 401 });
  }

  // 400, 422, 200 — any of those mean the key got past the auth gate.
  // Treat anything else (500, 502, 504) as a backend error.
  if (upstream.status >= 500) {
    return NextResponse.json(
      { error: `Backend error (${upstream.status})` },
      { status: 502 },
    );
  }

  const res = NextResponse.json({ ok: true });
  res.cookies.set(ADMIN_COOKIE, key, {
    httpOnly: true,
    sameSite: "lax",
    secure: process.env.NODE_ENV === "production",
    path: "/",
    maxAge: 60 * 60 * 24 * 30,
  });
  return res;
}

/** DELETE clears the cookie. */
export async function DELETE() {
  const res = NextResponse.json({ ok: true });
  res.cookies.delete(ADMIN_COOKIE);
  return res;
}
