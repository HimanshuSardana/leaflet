import type { Metadata } from "next";
import { ThemeProvider } from "next-themes";

import { Nav } from "@/components/nav";
import { Providers } from "./providers";
import "./globals.css";

export const metadata: Metadata = {
  title: "leaflet",
  description: "Self-hosted ebook, manga and light novel server",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body>
        <ThemeProvider attribute="class" defaultTheme="dark" enableSystem={false}>
          <Providers>
            <Nav />
            <main className="mx-auto max-w-5xl px-6 py-10">{children}</main>
          </Providers>
        </ThemeProvider>
      </body>
    </html>
  );
}
