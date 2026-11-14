"use client";

import { useQuery } from "@tanstack/react-query";
import { BookPlus } from "lucide-react";

import {
  Alert,
  AlertContent,
  AlertDescription,
  AlertIcon,
  AlertTitle,
} from "@/components/motion/alert";
import { StatefulButton } from "@/components/motion/button/stateful";
import { Card } from "@/components/motion/card";
import { Loader } from "@/components/motion/loader";
import { LinkButton } from "@/components/link-button";
import { getVersion } from "@/lib/api";
import { useAuth } from "@/lib/auth";

export function LibraryView() {
  const { state } = useAuth();
  const version = useQuery({ queryKey: ["version"], queryFn: getVersion });

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

  return (
    <div className="mx-auto w-full max-w-4xl space-y-6 px-6 py-10">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div className="space-y-1">
          <h1 className="text-3xl font-semibold">Library</h1>
          <p className="text-sm text-muted-foreground">
            {state.user.display_name} · nothing uploaded yet
          </p>
        </div>
        <StatefulButton icon={<BookPlus className="size-4" />} disabled>
          Upload a book
        </StatefulButton>
      </div>

      <Card className="grid place-items-center gap-3 border-dashed p-14 text-center">
        <BookPlus className="size-8 text-muted-foreground" />
        <p className="text-sm font-medium">Your shelves are empty</p>
        <p className="max-w-sm text-sm text-muted-foreground">
          PDFs and EPUBs land here once uploading ships — with covers, series and reading
          progress that follows you between devices.
        </p>
      </Card>
    </div>
  );
}
