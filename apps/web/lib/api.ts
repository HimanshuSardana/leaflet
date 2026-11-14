import type { Version } from "@leaflet/contracts";

/** Same-origin by construction: in prod the rust binary serves this export. */
export async function getVersion(): Promise<Version> {
  const response = await fetch("/api/v1/version", { headers: { accept: "application/json" } });
  if (!response.ok) {
    throw new Error(`version request failed: ${response.status}`);
  }
  return (await response.json()) as Version;
}
