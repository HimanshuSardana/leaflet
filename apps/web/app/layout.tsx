import type { Metadata } from "next";
import { ThemeProvider } from "next-themes";
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
    <header className="border-b border-neutral-200 dark:border-neutral-800">
      <nav className="mx-auto flex max-w-5xl items-center gap-6 px-6 py-4">
        <span className="font-semibold tracking-tight text-leaf-700 dark:text-leaf-500">leaflet</span>
        <a className="text-sm text-neutral-600 hover:text-neutral-900 dark:text-neutral-400 dark:hover:text-neutral-100" href="/">
          Status
        </a>
        <a className="text-sm text-neutral-600 hover:text-neutral-900 dark:text-neutral-400 dark:hover:text-neutral-100" href="/library/">
          Library
        </a>
      </nav>
    </header>
  );
}
