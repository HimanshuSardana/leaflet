import { cn } from "@/lib/utils";

/**
 * Anchors that look like beUI buttons. `StatefulButton` has no `href` (it owns a
 * <button>), and navigation must stay a real link so the static export's pages
 * are crawlable and middle-clickable.
 */
const VARIANT = {
  primary: "bg-primary text-primary-foreground hover:bg-primary/90",
  secondary: "border border-border bg-card text-foreground hover:bg-muted/40",
  ghost: "text-muted-foreground hover:bg-muted/60 hover:text-foreground",
} as const;

export function LinkButton({
  href,
  children,
  variant = "primary",
  className,
}: {
  href: string;
  children: React.ReactNode;
  variant?: keyof typeof VARIANT;
  className?: string;
}) {
  return (
    <a
      href={href}
      className={cn(
        "inline-flex h-10 items-center justify-center gap-2 rounded-full px-5 text-sm font-medium",
        "transition-[background-color,border-color,color,transform] duration-150 active:scale-[0.985]",
        "outline-none focus-visible:ring-2 focus-visible:ring-ring",
        VARIANT[variant],
        className,
      )}
    >
      {children}
    </a>
  );
}
