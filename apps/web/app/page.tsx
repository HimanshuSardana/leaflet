"use client";

import { useQuery } from "@tanstack/react-query";
import { getVersion } from "@/lib/api";

export default function StatusPage() {
  const { data, error, isPending } = useQuery({ queryKey: ["version"], queryFn: getVersion });

  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-semibold tracking-tight">Server status</h1>
      {isPending && <p className="text-sm text-neutral-500">Contacting the API…</p>}
      {error && (
        <p className="rounded-md border border-red-300 bg-red-50 p-3 text-sm text-red-700 dark:border-red-900 dark:bg-red-950 dark:text-red-300">
          API unreachable: {error.message}
        </p>
      )}
      {data && (
        <dl className="grid grid-cols-2 gap-x-8 gap-y-3 text-sm sm:max-w-md">
          <dt className="text-neutral-500">server</dt>
          <dd className="font-mono">{data.name}</dd>
          <dt className="text-neutral-500">version</dt>
          <dd className="font-mono">{data.version}</dd>
          <dt className="text-neutral-500">api</dt>
          <dd className="font-mono">{data.api}</dd>
          <dt className="text-neutral-500">schema</dt>
          <dd className="font-mono">{data.schema}</dd>
        </dl>
      )}
    </div>
  );
}
