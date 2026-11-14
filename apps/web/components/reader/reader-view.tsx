"use client";

import { useQuery } from "@tanstack/react-query";
import { ArrowLeft, ChevronLeft, ChevronRight, ListTree, X } from "lucide-react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { Alert, AlertContent, AlertDescription, AlertIcon, AlertTitle } from "@/components/motion/alert";
import { LinkButton } from "@/components/link-button";
import { Loader } from "@/components/motion/loader";
import {
  EpubStage,
  type ReaderControls,
  type StageState,
  type TocEntry,
} from "@/components/reader/epub-stage";
import { listBooks } from "@/lib/api";
import { useAuth } from "@/lib/auth";
import { fetchBookBytes, formatPercentage } from "@/lib/reader";
import { cn } from "@/lib/utils";

export function ReaderView() {
  const params = useSearchParams();
  const bookId = params.get("id") ?? "";
  const { state } = useAuth();
  const accessToken = state.status === "authenticated" ? state.accessToken : null;

  const shelf = useQuery({
    queryKey: ["books"],
    queryFn: () => listBooks(accessToken ?? ""),
    enabled: accessToken !== null,
  });
  const book = shelf.data?.find((entry) => entry.id === bookId) ?? null;

  const [bytes, setBytes] = useState<ArrayBuffer | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const [ready, setReady] = useState(false);
  const [tocOpen, setTocOpen] = useState(false);
  const [stage, setStage] = useState<StageState>({
    toc: [],
    chapter: null,
    percentage: 0,
    atStart: true,
    atEnd: false,
  });
  const controlsRef = useRef<ReaderControls | null>(null);
  const markReady = useCallback(() => setReady(true), []);

  useEffect(() => {
    if (!accessToken || !bookId) return;
    let cancelled = false;
    setBytes(null);
    setReady(false);
    setFailure(null);

    fetchBookBytes(bookId, accessToken)
      .then((buffer) => {
        if (!cancelled) setBytes(buffer);
      })
      .catch((error: unknown) => {
        if (!cancelled) setFailure(error instanceof Error ? error.message : "Could not load this book");
      });

    return () => {
      cancelled = true;
    };
  }, [accessToken, bookId]);

  const go = useCallback((direction: "next" | "prev") => {
    const controls = controlsRef.current;
    if (!controls) return;
    if (direction === "next") controls.next();
    else controls.prev();
  }, []);

  // Keys land here when focus is on our chrome; inside the book's iframe they
  // are caught by the rendition itself.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.metaKey || event.ctrlKey || event.altKey || event.repeat) return;
      if (event.key === "ArrowRight" || event.key === "PageDown") {
        event.preventDefault();
        go("next");
      } else if (event.key === "ArrowLeft" || event.key === "PageUp") {
        event.preventDefault();
        go("prev");
      } else if (event.key === "Escape") {
        setTocOpen(false);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [go]);

  const openChapter = useCallback((entry: TocEntry) => {
    controlsRef.current?.goTo(entry.href);
    setTocOpen(false);
  }, []);

  if (state.status === "loading") {
    return <Centered><Loader variant="bars" size={26} /></Centered>;
  }

  if (state.status === "anonymous") {
    return (
      <Centered>
        <div className="w-full max-w-sm space-y-4 text-center">
          <p className="text-sm text-muted-foreground">Sign in to keep reading.</p>
          <LinkButton href="/login/" className="w-full">
            Sign in
          </LinkButton>
        </div>
      </Centered>
    );
  }

  if (shelf.isLoading || (!book && !shelf.isError)) {
    return <Centered><Loader variant="bars" size={26} /></Centered>;
  }

  if (!book) {
    return (
      <Centered>
        <div className="w-full max-w-sm space-y-4">
          <Alert variant="destructive">
            <AlertIcon />
            <AlertContent>
              <AlertTitle>No such book on your shelf</AlertTitle>
              <AlertDescription>This link points at a book that is not yours.</AlertDescription>
            </AlertContent>
          </Alert>
          <LinkButton href="/library/" variant="secondary" className="w-full">
            Back to the library
          </LinkButton>
        </div>
      </Centered>
    );
  }

  if (book.format !== "epub") {
    return (
      <Centered>
        <div className="w-full max-w-sm space-y-4 text-center">
          <p className="text-sm text-muted-foreground">
            The reader handles EPUB today. PDF books open here soon.
          </p>
          <LinkButton href="/library/" variant="secondary" className="w-full">
            Back to the library
          </LinkButton>
        </div>
      </Centered>
    );
  }

  return (
    <div className="flex h-svh flex-col bg-background">
      <header className="flex h-14 shrink-0 items-center gap-2 border-b border-border px-2 sm:px-4">
        <Link
          href="/library/"
          aria-label="Back to the library"
          className="grid size-9 shrink-0 place-items-center rounded-full text-muted-foreground transition-colors duration-150 ease-[var(--ease-out)] hover:bg-muted hover:text-foreground"
        >
          <ArrowLeft className="size-4" />
        </Link>

        <div className="min-w-0 flex-1">
          <p className="truncate font-serif text-base leading-tight">{book.title}</p>
          {stage.chapter ? (
            <p className="truncate text-xs text-muted-foreground">{stage.chapter}</p>
          ) : null}
        </div>

        <span className="shrink-0 font-mono text-[11px] tabular text-muted-foreground">
          {formatPercentage(stage.percentage)}
        </span>

        <button
          type="button"
          onClick={() => setTocOpen((open) => !open)}
          aria-expanded={tocOpen}
          aria-label={tocOpen ? "Hide contents" : "Show contents"}
          className={cn(
            "grid size-9 shrink-0 place-items-center rounded-full transition-colors duration-150 ease-[var(--ease-out)]",
            tocOpen
              ? "bg-muted text-foreground"
              : "text-muted-foreground hover:bg-muted hover:text-foreground",
          )}
        >
          <ListTree className="size-4" />
        </button>
      </header>

      <div className="relative flex min-h-0 flex-1">
        <nav
          aria-label="Contents"
          className={cn(
            "absolute inset-y-0 left-0 z-10 w-72 border-r border-border bg-background transition-transform duration-200 ease-[var(--ease-out)]",
            tocOpen ? "translate-x-0" : "-translate-x-full",
          )}
        >
          <div className="flex h-10 items-center justify-between px-3">
            <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-muted-foreground">
              Contents
            </span>
            <button
              type="button"
              onClick={() => setTocOpen(false)}
              aria-label="Hide contents"
              className="grid size-6 place-items-center rounded-full text-muted-foreground hover:bg-muted hover:text-foreground"
            >
              <X className="size-3.5" />
            </button>
          </div>
          <ol className="h-[calc(100%-2.5rem)] overflow-y-auto px-2 pb-6">
            {stage.toc.length === 0 ? (
              <li className="px-2 py-2 text-xs text-muted-foreground">This book has no contents.</li>
            ) : (
              stage.toc.map((entry) => (
                <li key={entry.id}>
                  <button
                    type="button"
                    onClick={() => openChapter(entry)}
                    aria-current={entry.label === stage.chapter ? "true" : undefined}
                    className={cn(
                      "relative w-full truncate rounded-md px-2 py-1.5 text-left text-sm",
                      "transition-colors duration-150 ease-[var(--ease-out)] hover:bg-muted hover:text-foreground",
                      entry.label === stage.chapter
                        ? "font-medium text-foreground before:absolute before:inset-y-1.5 before:left-0 before:w-0.5 before:rounded-full before:bg-primary"
                        : "text-muted-foreground",
                    )}
                    style={{ paddingLeft: `${0.75 + (entry.depth - 1) * 0.75}rem` }}
                  >
                    {entry.label}
                  </button>
                </li>
              ))
            )}
          </ol>
        </nav>

        {/* definite height: epubjs measures this box, and a box whose height
            depends on its content re-measures forever */}
        <div className="mx-auto flex h-full w-full max-w-[44rem] flex-col px-4 py-6 sm:px-6">
          {failure ? (
            <Alert variant="destructive">
              <AlertIcon />
              <AlertContent>
                <AlertTitle>Could not open this book</AlertTitle>
                <AlertDescription>{failure}</AlertDescription>
              </AlertContent>
            </Alert>
          ) : bytes ? (
            <div className="min-h-0 flex-1">
                <EpubStage
                bookId={book.id}
                bytes={bytes}
                state={stage}
                onState={setStage}
                controlsRef={controlsRef}
                onReady={markReady}
                onError={setFailure}
              />
            </div>
          ) : (
            <div className="flex h-40 items-center justify-center gap-3 text-sm text-muted-foreground">
              <Loader variant="bars" size={18} /> Opening {book.title}
            </div>
          )}
        </div>
      </div>

      {/* Navigation sits inside the reading column, not at the window edges,
          so the controls stay near the text they move. */}
      <footer className="flex h-14 shrink-0 items-center border-t border-border">
        <div className="mx-auto flex w-full max-w-[44rem] items-center justify-between px-4 sm:px-6">
          <PageButton
            label="Previous page"
            disabled={!ready || stage.atStart}
            onClick={() => go("prev")}
            icon={<ChevronLeft className="size-4" />}
          >
            Previous
          </PageButton>
          <PageButton
            label="Next page"
            disabled={!ready || stage.atEnd}
            onClick={() => go("next")}
            icon={<ChevronRight className="size-4" />}
            iconAfter
          >
            Next
          </PageButton>
        </div>
      </footer>
    </div>
  );
}

function PageButton({
  label,
  onClick,
  disabled,
  icon,
  iconAfter = false,
  children,
}: {
  label: string;
  onClick: () => void;
  disabled: boolean;
  icon: React.ReactNode;
  iconAfter?: boolean;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      onClick={onClick}
      disabled={disabled}
      className={cn(
        "inline-flex h-9 items-center gap-1.5 rounded-full px-3 text-xs text-muted-foreground",
        "transition-[color,background-color,transform] duration-150 ease-[var(--ease-out)]",
        "hover:bg-muted hover:text-foreground active:scale-[0.97]",
        "disabled:pointer-events-none disabled:opacity-35",
      )}
    >
      {iconAfter ? null : icon}
      {children}
      {iconAfter ? icon : null}
    </button>
  );
}

function Centered({ children }: { children: React.ReactNode }) {
  return <div className="grid h-svh place-items-center p-6">{children}</div>;
}
