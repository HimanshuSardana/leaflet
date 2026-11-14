import type { Metadata } from "next";
import { Suspense } from "react";

import { ReaderView } from "@/components/reader/reader-view";

export const metadata: Metadata = { title: "Reading · leaflet" };

export default function ReadPage() {
  // useSearchParams needs a boundary; the export prerenders the fallback only.
  return (
    <Suspense fallback={<div className="h-svh" />}>
      <ReaderView />
    </Suspense>
  );
}
