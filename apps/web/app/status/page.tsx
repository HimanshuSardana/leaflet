import type { Metadata } from "next";

import { StatusPanel } from "@/components/status-panel";

export const metadata: Metadata = { title: "Status · leaflet" };

export default function StatusPage() {
  return <StatusPanel />;
}
