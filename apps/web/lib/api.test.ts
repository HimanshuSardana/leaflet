import { afterEach, expect, test, vi } from "vitest";

import { ApiProblem, getVersion, request } from "./api";

afterEach(() => vi.unstubAllGlobals());

function jsonResponse(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

test("getVersion reads the version payload", async () => {
  const payload = { name: "leaflet-server", version: "0.1.0", api: "v1", schema: "v1", registration: "open" };
  vi.stubGlobal("fetch", vi.fn(async () => jsonResponse(payload)));

  await expect(getVersion()).resolves.toEqual(payload);
});

test("a problem+json error becomes an ApiProblem carrying the detail", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () =>
      jsonResponse(
        { type: "invalid_credentials", title: "Invalid credentials", status: 401, detail: "email or password is wrong" },
        401,
      ),
    ),
  );

  const error = await request("/auth/login", { method: "POST", body: {} }).catch((e) => e);
  expect(error).toBeInstanceOf(ApiProblem);
  expect(error).toMatchObject({ status: 401, kind: "invalid_credentials" });
  expect(error.message).toBe("email or password is wrong");
  expect(error.isAuthFailure).toBe(true);
});

test("a non-json error body still yields an ApiProblem", async () => {
  vi.stubGlobal("fetch", vi.fn(async () => new Response("boom", { status: 503 })));

  const error = await getVersion().catch((e) => e);
  expect(error).toBeInstanceOf(ApiProblem);
  expect(error.status).toBe(503);
  expect(error.isAuthFailure).toBe(false);
});

test("requests send the bearer token and same-origin credentials", async () => {
  const spy = vi.fn(async () => jsonResponse({ id: "u1" }));
  vi.stubGlobal("fetch", spy);

  await request("/me", { accessToken: "token-123" });

  const [url, init] = spy.mock.calls[0] as unknown as [string, RequestInit];
  expect(url).toBe("/api/v1/me");
  expect(init.credentials).toBe("same-origin");
  expect((init.headers as Record<string, string>).authorization).toBe("Bearer token-123");
});
