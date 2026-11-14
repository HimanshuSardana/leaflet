"use client";

import { useRouter } from "next/navigation";

import { Alert } from "@/components/motion/alert";
import { StatefulButton } from "@/components/motion/button/stateful";
import { Loader } from "@/components/motion/loader";
import { useAuth } from "@/lib/auth";

/** Static export has no middleware, so the guard is a client component. */
export function LibraryGate({ children }: { children: React.ReactNode }) {
  const router = useRouter();
  const { state } = useAuth();

  if (state.status === "loading") {
    return (
      <div className="grid place-items-center py-16">
        <Loader variant="bars" size={28} />
      </div>
    );
  }

  if (state.status === "anonymous") {
    return (
      <div className="mx-auto max-w-md space-y-4">
        <Alert variant="info" title="Sign in to see your library">
          Books, collections and reading progress belong to an account on this server.
        </Alert>
        <div className="flex gap-3">
          <StatefulButton onClick={() => router.push("/login/")} className="flex-1">
            Sign in
          </StatefulButton>
          <StatefulButton
            variant="secondary"
            onClick={() => router.push("/onboarding/")}
            className="flex-1"
          >
            Create an account
          </StatefulButton>
        </div>
      </div>
    );
  }

  return <>{children}</>;
}
