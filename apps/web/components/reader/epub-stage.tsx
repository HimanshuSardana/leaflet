"use client";

import type { Book as EpubBook, NavItem, Rendition } from "epubjs";
import { useTheme } from "next-themes";
import { useCallback, useEffect, useRef, useState } from "react";

import { readPosition, writePosition } from "@/lib/reader";

export type TocEntry = { id: string; href: string; label: string; depth: number };

export type StageState = {
  toc: TocEntry[];
  chapter: string | null;
  percentage: number;
  atStart: boolean;
  atEnd: boolean;
};

export type ReaderControls = {
  next: () => void;
  prev: () => void;
  goTo: (href: string) => void;
};

type Props = {
  bookId: string;
  bytes: ArrayBuffer;
  state: StageState;
  onState: React.Dispatch<React.SetStateAction<StageState>>;
  controlsRef: React.RefObject<ReaderControls | null>;
  onReady: () => void;
  onError: (message: string) => void;
};

const SAVE_DELAY_MS = 400;

function flattenToc(items: NavItem[], depth = 1): TocEntry[] {
  return items.flatMap((item) => [
    {
      id: item.id || item.href,
      href: item.href,
      label: (item.label ?? "").trim() || "Untitled",
      depth,
    },
    ...flattenToc(item.subitems ?? [], depth + 1),
  ]);
}

/** Reads the app's own tokens so the page inside the book matches the app. */
function themeRules(): Record<string, Record<string, string>> {
  const styles = getComputedStyle(document.documentElement);
  const token = (name: string, fallback: string) =>
    styles.getPropertyValue(name).trim() || fallback;

  const background = token("--background", "#0b0c0b");
  const foreground = token("--foreground", "#e8e6e1");
  const border = token("--border", "#2a2e2a");
  const primary = token("--primary", "#7bd88f");

  return {
    body: {
      background,
      color: foreground,
      // A book's own typefaces win; geometry and rhythm are ours to set.
      "line-height": "1.7",
      "text-rendering": "optimizeLegibility",
    },
    "p, li, blockquote, dd": { "line-height": "1.7" },
    "h1, h2, h3, h4": { "line-height": "1.25", "margin-block": "1.2em 0.5em" },
    "img, svg, image": { "max-width": "100%", height: "auto" },
    a: { color: primary },
    hr: { "border-color": border },
    "::selection": { background: `${primary}33` },
  };
}

const initialState: StageState = {
  toc: [],
  chapter: null,
  percentage: 0,
  atStart: true,
  atEnd: false,
};

/**
 * epubjs mutates the DOM it owns, so this component is deliberately imperative:
 * the rendition is created once, and everything it learns comes back through
 * callbacks. React never re-renders the pages themselves.
 */
export function EpubStage({ bookId, bytes, onState, controlsRef, onReady, onError }: Props) {
  const { resolvedTheme } = useTheme();
  const [host, setHost] = useState<HTMLDivElement | null>(null);
  const renditionRef = useRef<Rendition | null>(null);

  // Handlers live in a ref so the rendition is created once per book: a new
  // inline callback on every parent render must not tear it down and rebuild it.
  const handlers = useRef({ onState, onReady, onError });
  handlers.current = { onState, onReady, onError };

  const publish = useCallback(
    (rendition: Rendition, epub: EpubBook) => {
      const location = rendition.location;
      if (!location?.start) return;
      const cfi = location.start.cfi;
      const href = (location.start.href ?? "").split("#")[0];
      const percentage = epub.locations.length()
        ? epub.locations.percentageFromCfi(cfi)
        : (location.start.percentage ?? 0);

      handlers.current.onState((previous) => ({
        ...previous,
        chapter:
          previous.toc.find((entry) => entry.href.split("#")[0] === href)?.label ??
          previous.chapter,
        percentage,
        atStart: Boolean(location.atStart),
        atEnd: Boolean(location.atEnd),
      }));
    },
    [],
  );

  useEffect(() => {
    if (!host) return;

    let cancelled = false;
    let saveTimer: ReturnType<typeof setTimeout> | undefined;
    let epub: EpubBook | null = null;

    (async () => {
      const { default: ePub } = await import("epubjs");
      if (cancelled) return;

      try {
        epub = ePub(bytes);
        // The package (OPF) has to be parsed before a rendition exists; epubjs
        // reads layout and spread out of it while constructing one.
        await epub.opened;
        if (cancelled) return;

        const rendition = epub.renderTo(host, {
          width: "100%",
          height: "100%",
          flow: "paginated",
          spread: "none",
          allowScriptedContent: false,
        });
        renditionRef.current = rendition;

        rendition.themes.register("leaflet", themeRules() as never);
        rendition.themes.select("leaflet");

        const navigation = await epub.loaded.navigation;
        const toc = flattenToc(navigation.toc);
        handlers.current.onState({ ...initialState, toc, chapter: toc[0]?.label ?? null });

        const saved = readPosition(bookId);
        await rendition.display(saved?.cfi);

        controlsRef.current = {
          next: () => void rendition.next(),
          prev: () => void rendition.prev(),
          goTo: (href) => void rendition.display(href),
        };

        rendition.on("relocated", () => {
          publish(rendition, epub as EpubBook);
          const location = rendition.location;
          if (!location?.start) return;
          const position = {
            cfi: location.start.cfi,
            percentage: epub!.locations.length()
              ? epub!.locations.percentageFromCfi(location.start.cfi)
              : (location.start.percentage ?? 0),
          };
          clearTimeout(saveTimer);
          saveTimer = setTimeout(() => writePosition(bookId, position), SAVE_DELAY_MS);
        });

        const keys = (event: KeyboardEvent) => {
          if (event.key === "ArrowRight" || event.key === "PageDown") {
            event.preventDefault();
            void rendition.next();
          } else if (event.key === "ArrowLeft" || event.key === "PageUp") {
            event.preventDefault();
            void rendition.prev();
          }
        };
        rendition.on("keyup", keys as never);

        handlers.current.onReady();

        // Percentages need locations, and generating them on a big book takes
        // seconds: reading starts first, the number fills in after.
        void epub.locations.generate(1024).then(() => {
          if (cancelled) return;
          publish(rendition, epub as EpubBook);
          const location = rendition.location;
          if (location?.start) {
            writePosition(bookId, {
              cfi: location.start.cfi,
              percentage: epub!.locations.percentageFromCfi(location.start.cfi),
            });
          }
        });
      } catch (error) {
        if (cancelled) return;
        handlers.current.onError(
          error instanceof Error ? error.message : "This file could not be opened",
        );
      }
    })();

    return () => {
      cancelled = true;
      clearTimeout(saveTimer);
      controlsRef.current = null;
      // Teardown throws if the rendition never finished building, and that is
      // not worth reporting: the book is already gone from the screen.
      try {
        renditionRef.current?.destroy();
        epub?.destroy();
      } catch {
        /* Epub not fully opened; nothing to release. */
      }
      renditionRef.current = null;
    };
  }, [bookId, bytes, host, controlsRef, publish]);

  // A book carries its own colours; the app's theme wins, and re-registering is
  // instant where restyling by hand would flash the old page.
  useEffect(() => {
    const rendition = renditionRef.current;
    if (!rendition) return;
    rendition.themes.register("leaflet", themeRules() as never);
    rendition.themes.select("leaflet");
  }, [resolvedTheme]);

  return <div ref={setHost} className="h-full w-full overflow-hidden" />;
}
