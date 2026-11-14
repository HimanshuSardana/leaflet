"use client";

import type { Book } from "@leaflet/contracts";

import { formatAdded, formatBytes } from "@/lib/format";

/** The shelf. Rows, not cards: without covers yet, a list carries more books. */
export function BookList({ books }: { books: Book[] }) {
  if (books.length === 0) {
    return (
      <p className="border-t border-border py-10 text-sm text-muted-foreground">
        Nothing here yet. The books you add above live on this server, not in someone else&apos;s
        cloud.
      </p>
    );
  }

  return (
    <div>
      <div className="flex items-baseline gap-4 pb-2 font-mono text-[10px] uppercase tracking-[0.14em] text-muted-foreground">
        <span className="w-11 shrink-0">Format</span>
        <span className="min-w-0 flex-1">Title</span>
        <span className="w-16 shrink-0 text-right">Size</span>
        <span className="w-24 shrink-0 text-right">Added</span>
      </div>
      <ul className="divide-y divide-border border-t border-border">
        {books.map((book) => (
          <li key={book.id} className="flex items-baseline gap-4 py-3.5">
            <span className="w-11 shrink-0 font-mono text-[10px] uppercase tracking-[0.12em] text-muted-foreground">
              {book.format}
            </span>
            <span className="min-w-0 flex-1">
              <span className="block truncate text-sm leading-6">{book.title}</span>
              {book.author ? (
                <span className="block truncate text-xs text-muted-foreground">{book.author}</span>
              ) : null}
            </span>
            <span className="w-16 shrink-0 text-right font-mono text-[11px] tabular text-muted-foreground">
              {formatBytes(book.byte_size)}
            </span>
            <span className="w-24 shrink-0 text-right text-xs text-muted-foreground">
              {formatAdded(book.added_at)}
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}
