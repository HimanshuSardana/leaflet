import type { Metadata } from "next";
import { ThemeProvider } from "next-themes";
import { ThemeSwitch } from "@/components/theme-switch";
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

function Nav() {
  return (
    <header className="border-b border-border">
      <nav className="mx-auto flex max-w-5xl items-center gap-6 px-6 py-4">
        <span className="font-semibold tracking-tight text-primary">leaflet</span>
        <a className="text-sm text-muted-foreground transition-colors hover:text-foreground" href="/">
          Status
        </a>
        <a className="text-sm text-muted-foreground transition-colors hover:text-foreground" href="/library/">
          Library
        </a>
        <div className="ml-auto">
          <ThemeSwitch />
        </div>
      </nav>
    </header>
  );
}
