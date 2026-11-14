import type { Metadata } from "next";

import { LibraryGate } from "@/components/library-gate";

export const metadata: Metadata = { title: "Library · leaflet" };

export default function LibraryPage() {
  return (
    <LibraryGate>
      <div className="space-y-4">
        <h1 className="text-2xl font-semibold tracking-tight">Library</h1>
        <p className="max-w-prose text-sm text-muted-foreground">
          No books yet. Uploading arrives in the next milestone — this page exists to prove the
          shell, the session and the static export.
        </p>
      </div>
    </LibraryGate>
  );
}
