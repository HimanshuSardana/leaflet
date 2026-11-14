"use client";

import { useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useRef, useState } from "react";

import { FileUpload, type FileUploadItem } from "@/components/motion/file-upload";
import { useToast } from "@/components/toast";
import { getProblem, uploadBook } from "@/lib/api";
import { useAuth } from "@/lib/auth";

/** The server sniffs the bytes, so a wrong pick is refused rather than stored
 *  as something it is not. Filtering here only saves a round trip. */
const ACCEPT = ".pdf,.epub,application/pdf,application/epub+zip";

function describe(error: unknown): string {
  const problem = getProblem(error);
  if (!problem) return "upload failed";

  switch (problem.kind) {
    case "unsupported_format":
      return "not a PDF or EPUB";
    case "payload_too_large":
      return "larger than this server allows";
    case "unauthorized":
    case "invalid_token":
      return "signed out — sign in again";
    case "invalid_request":
      return problem.message;
    default:
      return problem.message || "upload failed";
  }
}

export function UploadBooks() {
  const { state } = useAuth();
  const accessToken = state.status === "authenticated" ? state.accessToken : null;
  const queryClient = useQueryClient();
  const { toast } = useToast();

  const [items, setItems] = useState<FileUploadItem[]>([]);
  const inFlight = useRef(new Map<string, AbortController>());

  const update = useCallback((id: string, patch: Partial<FileUploadItem>) => {
    setItems((current) =>
      current.map((item) => (item.id === id ? { ...item, ...patch } : item)),
    );
  }, []);

  const run = useCallback(
    async (id: string, file: File, token: string) => {
      const controller = new AbortController();
      inFlight.current.set(id, controller);
      update(id, { status: "uploading", progress: 0, error: undefined });

      try {
        const { book, created } = await uploadBook(file, token, {
          signal: controller.signal,
          // Percent steps, so React re-renders ~100 times per file, not 10k.
          onProgress: (fraction) => update(id, { progress: Math.round(fraction * 100) }),
        });
        update(id, { status: "success", progress: 100 });
        void queryClient.invalidateQueries({ queryKey: ["books"] });
        if (!created) {
          toast("info", "Already in your library", `${book.title} was uploaded before.`);
        }
      } catch (error) {
        if (controller.signal.aborted) return;
        update(id, { status: "error", error: describe(error) });
      } finally {
        inFlight.current.delete(id);
      }
    },
    [queryClient, toast, update],
  );

  const start = useCallback(
    (added: FileUploadItem[], files: File[]) => {
      if (!accessToken) return;
      added.forEach((item, index) => {
        const file = files[index] ?? item.file;
        if (file) void run(item.id, file, accessToken);
      });
    },
    [accessToken, run],
  );

  const retry = useCallback(
    (item: FileUploadItem) => {
      if (!accessToken || !item.file) return;
      void run(item.id, item.file, accessToken);
    },
    [accessToken, run],
  );

  const remove = useCallback((item: FileUploadItem) => {
    inFlight.current.get(item.id)?.abort();
    inFlight.current.delete(item.id);
  }, []);

  // Leaving the page must not leave a socket pinned on a half-sent book.
  useEffect(() => {
    const running = inFlight.current;
    return () => {
      for (const controller of running.values()) controller.abort();
      running.clear();
    };
  }, []);

  const finished = items.filter((item) => item.status === "success");

  return (
    <div className="space-y-3">
      <FileUpload
        value={items}
        onValueChange={setItems}
        onFilesAdded={start}
        onRemove={remove}
        onRetry={retry}
        accept={ACCEPT}
        title="Drop a PDF or EPUB"
        description="or browse — the file is copied to this server"
        browseLabel="Choose file"
        // "Uploaded" would be a lie for a file the server already had; both
        // outcomes end with the book in the library.
        labels={{ success: "In your library" }}
        disabled={!accessToken}
      />

      {finished.length > 0 ? (
        <div className="flex justify-end text-xs text-muted-foreground">
          <button
            type="button"
            onClick={() => setItems((current) => current.filter((item) => item.status !== "success"))}
            className="rounded text-xs text-muted-foreground underline-offset-4 transition-colors duration-150 hover:text-foreground hover:underline"
          >
            Clear finished
          </button>
        </div>
      ) : null}
    </div>
  );
}
