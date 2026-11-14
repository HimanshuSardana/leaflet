"use client";

import {
  BookOpen,
  ChevronRight,
  Info,
  LibraryBig,
  LogOut,
  PanelLeft,
  Settings,
  Sparkles,
  X,
} from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { usePathname } from "next/navigation";
import { useState } from "react";

import {
  AnimatedSidebar,
  AnimatedSidebarClose,
  AnimatedSidebarContent,
  AnimatedSidebarFooter,
  AnimatedSidebarGroup,
  AnimatedSidebarGroupContent,
  AnimatedSidebarGroupLabel,
  AnimatedSidebarHeader,
  AnimatedSidebarInset,
  AnimatedSidebarMenu,
  AnimatedSidebarMenuButton,
  AnimatedSidebarMenuItem,
  AnimatedSidebarMenuSub,
  AnimatedSidebarMenuSubButton,
  AnimatedSidebarMenuSubItem,
  AnimatedSidebarProvider,
  AnimatedSidebarRail,
  AnimatedSidebarTrigger,
} from "@/components/motion/animated-sidebar";
import { LinkButton } from "@/components/link-button";
import { ThemeSwitch } from "@/components/theme-switch";
import { getVersion } from "@/lib/api";
import { useAuth } from "@/lib/auth";
import { cn } from "@/lib/utils";

type Destination = {
  label: string;
  icon: typeof LibraryBig;
  href?: string;
  soon?: boolean;
  children?: { label: string; href?: string; soon?: boolean }[];
};

const GROUPS: { label: string; items: Destination[] }[] = [
  {
    label: "Reading",
    items: [
      {
        label: "Library",
        icon: LibraryBig,
        href: "/library/",
        children: [
          { label: "All books", href: "/library/" },
          { label: "Collections", soon: true },
          { label: "Reading now", soon: true },
          { label: "Finished", soon: true },
        ],
      },
      { label: "Discover", icon: Sparkles, soon: true },
    ],
  },
  {
    label: "Server",
    items: [
      { label: "Status", icon: Info, href: "/status/" },
      { label: "Settings", icon: Settings, soon: true },
    ],
  },
];

/**
 * Two frames, chosen by auth state: a dashboard with a navigation rail for
 * signed-in readers, and an ordinary page for everyone else. Rendering the
 * sidebar for a visitor who has no library to navigate was the whole problem.
 */
export function AppShell({ children }: { children: React.ReactNode }) {
  const { state } = useAuth();

  if (state.status === "authenticated") {
    return <Dashboard>{children}</Dashboard>;
  }
  return <PublicFrame>{children}</PublicFrame>;
}

function Dashboard({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const { state, logout } = useAuth();
  const [openSection, setOpenSection] = useState<string | null>("Library");

  // Routes outside the nav still need a title in the inset header.
  const EXTRA_TITLES: Record<string, string> = {
    "/onboarding/": "Set up",
    "/login/": "Sign in",
    "/": "Welcome",
  };
  const current =
    GROUPS.flatMap((group) => group.items)
      .flatMap((item) => [{ label: item.label, href: item.href }, ...(item.children ?? [])])
      .find((entry) => entry.href === pathname)?.label ??
    EXTRA_TITLES[pathname] ??
    "leaflet";

  return (
    <AnimatedSidebarProvider className="h-svh min-h-0 overflow-hidden">
      <AnimatedSidebar
        ariaLabel="leaflet"
        collapsible="icon"
        className="min-h-0"
        panelClassName="h-full"
      >
        <AnimatedSidebarHeader className="p-3 pb-2">
          <div className="flex min-h-11 items-center gap-3 overflow-hidden px-2">
            <a
              href="/"
              className="grid size-7 shrink-0 place-items-center rounded-lg bg-primary text-primary-foreground"
            >
              <BookOpen aria-hidden="true" className="size-4" />
            </a>
            <a
              href="/"
              className="flex min-w-0 flex-1 items-center rounded-lg text-left outline-none group-data-[state=collapsed]/sidebar:hidden focus-visible:ring-2 focus-visible:ring-ring"
            >
              <span className="truncate text-sm font-semibold text-foreground">leaflet</span>
            </a>
            <AnimatedSidebarClose className="ml-auto text-muted-foreground hover:bg-muted md:hidden">
              <X aria-hidden="true" className="size-4" />
            </AnimatedSidebarClose>
          </div>
        </AnimatedSidebarHeader>

        <AnimatedSidebarContent className="px-2 pt-1">
          {GROUPS.map((group) => (
            <AnimatedSidebarGroup key={group.label} className="pb-2">
              <AnimatedSidebarGroupLabel>{group.label}</AnimatedSidebarGroupLabel>
              <AnimatedSidebarGroupContent>
                <AnimatedSidebarMenu>
                  {group.items.map(({ label, icon: Icon, href, soon, children }) => {
                    const childActive = children?.some((child) => child.href === pathname) ?? false;
                    return (
                      <AnimatedSidebarMenuItem key={label}>
                        <AnimatedSidebarMenuButton
                          href={href}
                          disabled={soon || (!href && !children)}
                          badge={soon ? "soon" : undefined}
                          isActive={href === pathname || childActive}
                          ariaExpanded={children ? openSection === label : undefined}
                          icon={<Icon className="size-4" />}
                          onSelect={() => {
                            if (!children) return;
                            setOpenSection((current) => (current === label ? null : label));
                          }}
                        >
                          {label}
                        </AnimatedSidebarMenuButton>
                        {children ? (
                          <AnimatedSidebarMenuSub open={openSection === label}>
                            {children.map((child) => (
                              <AnimatedSidebarMenuSubItem key={child.label}>
                                <AnimatedSidebarMenuSubButton
                                  href={child.href}
                                  disabled={child.soon}
                                  isActive={child.href === pathname}
                                >
                                  {child.label}
                                </AnimatedSidebarMenuSubButton>
                              </AnimatedSidebarMenuSubItem>
                            ))}
                          </AnimatedSidebarMenuSub>
                        ) : null}
                      </AnimatedSidebarMenuItem>
                    );
                  })}
                </AnimatedSidebarMenu>
              </AnimatedSidebarGroupContent>
            </AnimatedSidebarGroup>
          ))}
        </AnimatedSidebarContent>

        <AnimatedSidebarFooter className="gap-2 border-none p-3">
          <div className="flex min-h-11 items-center gap-3 overflow-hidden rounded-xl p-1">
            <span className="grid size-9 shrink-0 place-items-center rounded-full bg-muted text-xs font-semibold uppercase text-muted-foreground">
              {initials(state.status === "authenticated" ? state.user.display_name : null)}
            </span>
            <span className="min-w-0 flex-1 group-data-[state=collapsed]/sidebar:hidden">
              <span className="block truncate text-sm font-medium text-foreground">
                {state.status === "authenticated" ? state.user.display_name : "Not signed in"}
              </span>
              <span className="block truncate text-xs text-muted-foreground">
                {state.status === "authenticated" ? state.user.email : "local server"}
              </span>
            </span>
          </div>
          <div className="flex items-center gap-1 group-data-[state=collapsed]/sidebar:flex-col">
            <ThemeSwitch />
            {state.status === "authenticated" ? (
              <button
                type="button"
                onClick={() => void logout()}
                className={cn(
                  "flex min-h-9 flex-1 items-center gap-2 rounded-lg px-2 text-left text-xs",
                  "text-muted-foreground transition-colors hover:bg-muted hover:text-foreground",
                  "group-data-[state=collapsed]/sidebar:justify-center",
                )}
              >
                <LogOut aria-hidden="true" className="size-4" />
                <span className="group-data-[state=collapsed]/sidebar:hidden">Sign out</span>
              </button>
            ) : (
              <a
                href="/login/"
                className="flex min-h-9 flex-1 items-center gap-2 rounded-lg px-2 text-xs text-muted-foreground transition-colors hover:bg-muted hover:text-foreground group-data-[state=collapsed]/sidebar:hidden"
              >
                <ChevronRight aria-hidden="true" className="size-4" />
                Sign in
              </a>
            )}
          </div>
        </AnimatedSidebarFooter>

        <AnimatedSidebarRail />
      </AnimatedSidebar>

      <AnimatedSidebarInset className="min-h-0">
        <header className="flex h-16 shrink-0 items-center gap-3 border-b border-border px-4">
          <AnimatedSidebarTrigger className="text-muted-foreground transition-colors hover:bg-muted hover:text-foreground">
            <PanelLeft aria-hidden="true" className="size-4" />
          </AnimatedSidebarTrigger>
          <div className="h-5 w-px bg-border" />
          <p className="text-sm font-medium text-foreground">{current}</p>
        </header>
        <div className="min-h-0 flex-1 overflow-y-auto">{children}</div>
      </AnimatedSidebarInset>
    </AnimatedSidebarProvider>
  );
}

function initials(name: string | null): string {
  if (!name) return "?";
  return name
    .split(/\s+/)
    .slice(0, 2)
    .map((part) => part[0] ?? "")
    .join("");
}

/** What an anonymous visitor sees: a plain top bar, no rail, no dashboard. */
function PublicFrame({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const version = useQuery({
    queryKey: ["version"],
    queryFn: getVersion,
    staleTime: 5 * 60_000,
  });
  const firstRun = version.data?.registration === "open";

  return (
    <div className="flex min-h-svh flex-col">
      <header className="sticky top-0 z-30 border-b border-border bg-background/80 backdrop-blur">
        <div className="mx-auto flex h-16 w-full max-w-6xl items-center gap-6 px-6">
          <a href="/" className="flex items-center gap-2">
            <span className="grid size-7 place-items-center rounded-lg bg-primary text-primary-foreground">
              <BookOpen aria-hidden="true" className="size-4" />
            </span>
            <span className="text-sm font-semibold tracking-tight">leaflet</span>
          </a>

          <nav className="hidden items-center gap-5 sm:flex">
            {[
              { href: "/status/", label: "Status" },
              { href: "/library/", label: "Library" },
            ].map((link) => (
              <a
                key={link.href}
                href={link.href}
                aria-current={pathname === link.href ? "page" : undefined}
                className={cn(
                  "text-sm text-muted-foreground transition-colors hover:text-foreground",
                  pathname === link.href && "text-foreground",
                )}
              >
                {link.label}
              </a>
            ))}
          </nav>

          <div className="ml-auto flex items-center gap-2">
            <ThemeSwitch />
            {pathname !== "/login/" && (
              <LinkButton href="/login/" variant="ghost" className="h-9 px-4">
                Sign in
              </LinkButton>
            )}
            {pathname !== "/onboarding/" && (
              <LinkButton href="/onboarding/" className="h-9 px-4">
                {firstRun ? "Set up this server" : "Create account"}
              </LinkButton>
            )}
          </div>
        </div>
      </header>

      <main className="flex-1">{children}</main>

      <footer className="border-t border-border">
        <div className="mx-auto flex w-full max-w-6xl flex-wrap items-center gap-x-6 gap-y-2 px-6 py-6 text-xs text-muted-foreground">
          <span>leaflet — self-hosted reading, AGPL-3.0</span>
          <a href="/status/" className="transition-colors hover:text-foreground">
            Server status
          </a>
          <a href="/library/" className="transition-colors hover:text-foreground">
            Library
          </a>
        </div>
      </footer>
    </div>
  );
}
