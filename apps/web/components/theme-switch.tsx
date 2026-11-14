"use client";

import { Moon, Sun } from "lucide-react";
import { useTheme } from "next-themes";
import { useEffect, useState } from "react";

import { cn } from "@/lib/utils";

/**
 * Deliberately quiet. beUI's toggle repaints the entire page through a View
 * Transition (a 400–700ms clip wipe of the document root) — far too much motion
 * for a control pressed casually. Here only the icon crossfades, 150ms.
 */
export function ThemeSwitch({ className }: { className?: string }) {
  const { resolvedTheme, setTheme } = useTheme();
  const [mounted, setMounted] = useState(false);

  // Theme is unknown during prerender; render a stable shell until hydration.
  useEffect(() => setMounted(true), []);

  const isDark = mounted && resolvedTheme === "dark";

  return (
    <button
      type="button"
      aria-label={isDark ? "Switch to light theme" : "Switch to dark theme"}
      onClick={() => setTheme(isDark ? "light" : "dark")}
      className={cn(
        "grid size-9 place-items-center rounded-full text-muted-foreground",
        "transition-colors duration-150 ease-[var(--ease-out)] hover:bg-muted hover:text-foreground",
        "outline-none focus-visible:ring-2 focus-visible:ring-ring",
        className,
      )}
    >
      <span className="relative size-4">
        <Sun
          aria-hidden="true"
          className={cn(
            "absolute inset-0 size-4 transition-[opacity,transform] duration-150 ease-[var(--ease-out)]",
            isDark ? "rotate-0 opacity-100" : "-rotate-90 opacity-0",
          )}
        />
        <Moon
          aria-hidden="true"
          className={cn(
            "absolute inset-0 size-4 transition-[opacity,transform] duration-150 ease-[var(--ease-out)]",
            isDark ? "rotate-90 opacity-0" : "rotate-0 opacity-100",
          )}
        />
      </span>
    </button>
  );
}
