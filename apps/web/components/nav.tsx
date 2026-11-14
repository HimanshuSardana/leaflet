"use client";

import { useRouter } from "next/navigation";

import { StatefulButton } from "@/components/motion/button/stateful";
import { ThemeToggle } from "@/components/motion/theme-toggle";
import { useAuth } from "@/lib/auth";

const LINKS = [
  { href: "/library/", label: "Library" },
  { href: "/", label: "Status" },
];

export function Nav() {
  const router = useRouter();
  const { state, logout } = useAuth();

  return (
    <header className="border-b border-border">
      <nav className="mx-auto flex max-w-5xl items-center gap-5 px-6 py-3">
        <a href="/" className="font-semibold tracking-tight text-primary">
          leaflet
        </a>
        {LINKS.map((link) => (
          <a
            key={link.href}
            href={link.href}
            className="text-sm text-muted-foreground transition-colors hover:text-foreground"
          >
            {link.label}
          </a>
        ))}

        <div className="ml-auto flex items-center gap-3">
          <ThemeToggle />
          {state.status === "authenticated" ? (
            <>
              <span className="hidden text-sm text-muted-foreground sm:inline">
                {state.user.display_name}
              </span>
              <StatefulButton
                size="sm"
                variant="ghost"
                onClick={async () => {
                  await logout();
                  router.push("/login/");
                }}
              >
                Sign out
              </StatefulButton>
            </>
          ) : (
            <a
              href="/login/"
              className="text-sm text-muted-foreground transition-colors hover:text-foreground"
            >
              Sign in
            </a>
          )}
        </div>
      </nav>
    </header>
  );
}
