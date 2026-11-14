import { afterEach, expect, test, vi } from "vitest";

import { getVersion } from "./api";

afterEach(() => vi.unstubAllGlobals());

test("getVersion reads the version payload", async () => {
  const payload = { name: "leaflet-server", version: "0.1.0", api: "v1", schema: "v1" };
  vi.stubGlobal("fetch", vi.fn(async () => new Response(JSON.stringify(payload), { status: 200 })));

  await expect(getVersion()).resolves.toEqual(payload);
});

test("getVersion surfaces a non-ok status", async () => {
  vi.stubGlobal("fetch", vi.fn(async () => new Response("nope", { status: 503 })));

  await expect(getVersion()).rejects.toThrow("503");
});
