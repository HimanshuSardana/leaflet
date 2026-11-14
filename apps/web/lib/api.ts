/** Same-origin by construction: in prod the rust binary serves this export. */
export type Version = {
  name: string;
  version: string;
  api: string;
  schema: string;
};

export async function getVersion(): Promise<Version> {
  const response = await fetch("/api/v1/version", { headers: { accept: "application/json" } });
  if (!response.ok) {
    throw new Error(`version request failed: ${response.status}`);
  }
  return (await response.json()) as Version;
}
