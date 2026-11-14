"use client";

import { useQuery } from "@tanstack/react-query";

import { AnimatedBadge } from "@/components/motion/animated-badge";
import { Card } from "@/components/motion/card";
import { Loader } from "@/components/motion/loader";
import { getVersion } from "@/lib/api";

export function StatusPanel() {
  const { data, error, isPending } = useQuery({ queryKey: ["version"], queryFn: getVersion });

  return (
    <div className="mx-auto w-full max-w-3xl space-y-6 px-6 py-10">
      <div className="space-y-2">
        <h1 className="text-3xl font-semibold">Server status</h1>
        <p className="text-sm text-muted-foreground">
          What this leaflet build reports about itself.
        </p>
      </div>

      {isPending && <Loader variant="dots" size={22} />}

      {error && (
        <Card className="space-y-1 border-destructive/40 p-5">
          <div className="flex items-center gap-2">
            <AnimatedBadge status="danger">offline</AnimatedBadge>
            <span className="text-sm font-medium">The API did not answer</span>
          </div>
          <p className="text-sm text-muted-foreground">{error.message}</p>
        </Card>
      )}

      {data && (
        <Card className="gap-0 divide-y divide-border py-0">
          {[
            ["server", data.name],
            ["version", data.version],
            ["api", data.api],
            ["schema", data.schema],
          ].map(([label, value]) => (
            <div key={label} className="flex items-center justify-between gap-4 px-5 py-3">
              <span className="text-sm text-muted-foreground">{label}</span>
              <span className="font-mono text-sm tabular">{value}</span>
            </div>
          ))}
          <div className="flex items-center justify-between gap-4 px-5 py-3">
            <span className="text-sm text-muted-foreground">registration</span>
            <AnimatedBadge
              status={
                data.registration === "open"
                  ? "info"
                  : data.registration === "invite"
                    ? "success"
                    : "neutral"
              }
            >
              {data.registration}
            </AnimatedBadge>
          </div>
        </Card>
      )}
    </div>
  );
}
