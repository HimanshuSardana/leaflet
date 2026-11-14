import type { Metadata } from "next";

import { LandingPage } from "@/components/landing-page";

export const metadata: Metadata = {
  title: "leaflet — your books, on every device",
  description:
    "Self-hosted server for the PDFs and EPUBs you already own, with reading progress that follows you between devices.",
};

export default function Home() {
  return <LandingPage />;
}
