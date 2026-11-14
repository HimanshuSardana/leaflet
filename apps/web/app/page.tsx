"use client";

import { useQuery } from "@tanstack/react-query";
import { getVersion } from "@/lib/api";

export default function StatusPage() {
  const { data, error, isPending } = useQuery({ queryKey: ["version"], queryFn: getVersion });

  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-semibold tracking-tight">Server status</h1>
      {isPending && <p className="text-sm text-muted-foreground">Contacting the API…</p>}
      {error && (
        <p className="rounded-md border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive">
          API unreachable: {error.message}
        </p>
      )}
      {data && (
        <dl className="grid grid-cols-2 gap-x-8 gap-y-3 text-sm sm:max-w-md">
          <dt className="text-muted-foreground">server</dt>
          <dd className="font-mono">{data.name}</dd>
          <dt className="text-muted-foreground">version</dt>
          <dd className="font-mono">{data.version}</dd>
          <dt className="text-muted-foreground">api</dt>
          <dd className="font-mono">{data.api}</dd>
          <dt className="text-muted-foreground">schema</dt>
          <dd className="font-mono">{data.schema}</dd>
        </dl>
      )}
    </div>
  );
}
