"use client";

import {
  BookOpen,
  CircleUser,
  Info,
  LibraryBig,
  PanelLeft,
  Settings,
} from "lucide-react";
import { usePathname } from "next/navigation";

import {
  AnimatedSidebar,
  AnimatedSidebarContent,
  AnimatedSidebarFooter,
  AnimatedSidebarGroup,
  AnimatedSidebarGroupLabel,
  AnimatedSidebarHeader,
  AnimatedSidebarInset,
  AnimatedSidebarMenu,
  AnimatedSidebarMenuButton,
  AnimatedSidebarMenuItem,
  AnimatedSidebarProvider,
  AnimatedSidebarTrigger,
} from "@/components/motion/animated-sidebar";
import { ThemeToggle } from "@/components/motion/theme-toggle";
import { useAuth } from "@/lib/auth";

const NAV = [
  { href: "/library/", label: "Library", icon: <LibraryBig /> },
  { href: "/status/", label: "Status", icon: <Info /> },
];

const SOON = [{ href: "/settings/", label: "Settings", icon: <Settings />, disabled: true }];

export function AppShell({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const { state, logout } = useAuth();

  return (
    <AnimatedSidebarProvider defaultOpen>
      <AnimatedSidebar ariaLabel="Main">
        <AnimatedSidebarHeader>
          <a href="/" className="flex items-center gap-2 px-2 py-1">
            <BookOpen className="size-5 text-primary" />
            <span className="text-base font-semibold tracking-tight">leaflet</span>
          </a>
        </AnimatedSidebarHeader>

        <AnimatedSidebarContent>
          <AnimatedSidebarGroup>
            <AnimatedSidebarGroupLabel>Read</AnimatedSidebarGroupLabel>
            <AnimatedSidebarMenu>
              {NAV.map((item) => (
                <AnimatedSidebarMenuItem key={item.href}>
                  <AnimatedSidebarMenuButton
                    href={item.href}
                    icon={item.icon}
                    isActive={pathname === item.href}
                  >
                    {item.label}
                  </AnimatedSidebarMenuButton>
                </AnimatedSidebarMenuItem>
              ))}
              {SOON.map((item) => (
                <AnimatedSidebarMenuItem key={item.href}>
                  <AnimatedSidebarMenuButton icon={item.icon} disabled badge="soon">
                    {item.label}
                  </AnimatedSidebarMenuButton>
                </AnimatedSidebarMenuItem>
              ))}
            </AnimatedSidebarMenu>
          </AnimatedSidebarGroup>
        </AnimatedSidebarContent>

        <AnimatedSidebarFooter>
          <div className="flex items-center justify-between gap-2 px-2">
            <div className="flex min-w-0 items-center gap-2">
              <CircleUser className="size-4 shrink-0 text-muted-foreground" />
              <span className="truncate text-xs text-muted-foreground">
                {state.status === "authenticated" ? state.user.display_name : "Not signed in"}
              </span>
            </div>
            <ThemeToggle />
          </div>
          {state.status === "authenticated" && (
            <button
              type="button"
              onClick={() => void logout()}
              className="mx-2 mt-2 rounded-lg px-2 py-1 text-left text-xs text-muted-foreground transition-colors hover:bg-muted/60 hover:text-foreground"
            >
              Sign out
            </button>
          )}
        </AnimatedSidebarFooter>
      </AnimatedSidebar>

      <AnimatedSidebarInset>
        <div className="flex h-12 shrink-0 items-center gap-2 border-b border-border px-3">
          <AnimatedSidebarTrigger className="text-muted-foreground hover:text-foreground">
            <PanelLeft className="size-4" />
          </AnimatedSidebarTrigger>
          <span className="text-xs text-muted-foreground">
            {state.status === "authenticated" ? state.user.display_name : "Reading, synced"}
          </span>
        </div>
        {children}
      </AnimatedSidebarInset>
    </AnimatedSidebarProvider>
  );
}
