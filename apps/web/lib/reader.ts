/** Reader plumbing that is not about rendering: bytes in, position out. */

import { ApiProblem, bookFileUrl } from "./api";

export async function fetchBookBytes(id: string, accessToken: string): Promise<ArrayBuffer> {
  const response = await fetch(bookFileUrl(id), {
    headers: { authorization: `Bearer ${accessToken}` },
    credentials: "same-origin",
  });

  if (!response.ok) {
    throw new ApiProblem(response.status, {
      title: response.status === 401 ? "Signed out" : "Could not load this book",
      detail:
        response.status === 401
          ? "Sign in again to keep reading."
          : `The server answered ${response.status}.`,
    });
  }

  return response.arrayBuffer();
}

export type StoredPosition = { cfi: string; percentage: number };

const key = (bookId: string) => `leaflet:reader:${bookId}`;

/**
 * Position is kept locally for now. This is the seam the sync endpoint will
 * replace: everything that reads or writes a resume point goes through here.
 */
export function readPosition(bookId: string): StoredPosition | null {
  try {
    const raw = window.localStorage.getItem(key(bookId));
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Partial<StoredPosition>;
    if (typeof parsed.cfi !== "string" || parsed.cfi.length === 0) return null;
    return { cfi: parsed.cfi, percentage: Number(parsed.percentage ?? 0) || 0 };
  } catch {
    return null;
  }
}

export function writePosition(bookId: string, position: StoredPosition): void {
  try {
    window.localStorage.setItem(key(bookId), JSON.stringify(position));
  } catch {
    // private mode or a full quota: reading still works, it just won't resume
  }
}

export function formatPercentage(value: number | undefined): string {
  const pct = Math.round((value ?? 0) * 100);
  if (pct <= 0) return "0%";
  return `${Math.min(100, pct)}%`;
}
