import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import { ThemeProvider } from "next-themes";

import { Nav } from "@/components/nav";
import { Providers } from "./providers";
import "./globals.css";

// beUI's components are drawn against Geist; without it everything falls back to
// the OS UI font, which changes metrics, letter fit and heading weight.
const geistSans = Geist({ subsets: ["latin"], variable: "--font-geist-sans", display: "swap" });
const geistMono = Geist_Mono({
  subsets: ["latin"],
  variable: "--font-geist-mono",
  display: "swap",
});

export const metadata: Metadata = {
  title: "leaflet",
  description: "Self-hosted ebook, manga and light novel server",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html
      lang="en"
      suppressHydrationWarning
      className={`${geistSans.variable} ${geistMono.variable}`}
    >
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
