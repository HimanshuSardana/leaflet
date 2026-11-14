import type { Metadata } from "next";

import { LibraryView } from "@/components/library-view";

export const metadata: Metadata = { title: "Library · leaflet" };

export default function LibraryPage() {
  return <LibraryView />;
}
