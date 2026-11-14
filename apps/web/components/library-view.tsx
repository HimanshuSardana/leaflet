"use client";

import { useQuery } from "@tanstack/react-query";

import {
  Alert,
  AlertContent,
  AlertDescription,
  AlertIcon,
  AlertTitle,
} from "@/components/motion/alert";
import { Loader } from "@/components/motion/loader";
import { BookList } from "@/components/book-list";
import { LinkButton } from "@/components/link-button";
import { UploadBooks } from "@/components/upload-books";
import { getProblem, getVersion, listBooks } from "@/lib/api";
import { useAuth } from "@/lib/auth";
import { formatBytes } from "@/lib/format";

export function LibraryView() {
  const { state } = useAuth();
  const version = useQuery({ queryKey: ["version"], queryFn: getVersion });
  const accessToken = state.status === "authenticated" ? state.accessToken : null;
  const books = useQuery({
    queryKey: ["books"],
    queryFn: () => listBooks(accessToken ?? ""),
    enabled: accessToken !== null,
  });

  if (state.status === "loading") {
    return (
      <div className="grid place-items-center py-24">
        <Loader variant="bars" size={28} />
      </div>
    );
  }

  if (state.status === "anonymous") {
    const firstRun = version.data?.registration === "open";
    return (
      <div className="mx-auto w-full max-w-md space-y-4 px-6 py-14">
        <Alert variant="info">
          <AlertIcon />
          <AlertContent>
            <AlertTitle>Sign in to see your library</AlertTitle>
            <AlertDescription>
              Books, collections and reading progress belong to an account on this server.
            </AlertDescription>
          </AlertContent>
        </Alert>
        <div className="flex gap-3">
          <LinkButton href="/login/" className="flex-1">
            Sign in
          </LinkButton>
          <LinkButton href="/onboarding/" variant="secondary" className="flex-1">
            {firstRun ? "Set up this server" : "Create an account"}
          </LinkButton>
        </div>
      </div>
    );
  }

  const shelf = books.data ?? [];
  const totalBytes = shelf.reduce((sum, book) => sum + book.byte_size, 0);
  const problem = books.isError ? getProblem(books.error) : null;

  return (
    <div className="mx-auto w-full max-w-4xl px-6 py-10">
      <header className="flex flex-wrap items-end justify-between gap-3 pb-8">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Library</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            {books.isLoading
              ? "Loading…"
              : shelf.length === 0
                ? "No books yet"
                : `${shelf.length} ${shelf.length === 1 ? "book" : "books"} · ${formatBytes(totalBytes)}`}
          </p>
        </div>
      </header>

      <section aria-label="Add books" className="pb-10">
        <UploadBooks />
      </section>

      <section aria-label="Shelf">
        {problem ? (
          <Alert variant="destructive">
            <AlertIcon />
            <AlertContent>
              <AlertTitle>Could not load your shelf</AlertTitle>
              <AlertDescription>{problem.message}</AlertDescription>
            </AlertContent>
          </Alert>
        ) : books.isLoading ? (
          <div className="border-t border-border py-10">
            <Loader variant="bars" size={20} />
          </div>
        ) : (
          <BookList books={shelf} />
        )}
      </section>
    </div>
  );
}
