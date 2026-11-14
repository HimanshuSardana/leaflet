"use client";

import { useQuery } from "@tanstack/react-query";
import { motion, useReducedMotion } from "motion/react";
import { useRouter } from "next/navigation";
import { useEffect } from "react";

import { LinkButton } from "@/components/link-button";
import { getVersion } from "@/lib/api";
import { useAuth } from "@/lib/auth";

const CAPABILITIES = [
  {
    mark: "01",
    title: "Formats left alone",
    body: "PDFs and EPUBs are stored as uploaded and read in the browser. Nothing is converted, re-encoded or rewritten on disk.",
  },
  {
    mark: "02",
    title: "A position, not a page number",
    body: "Each book keeps the locator your device understands plus a normalised position, so a reflowable EPUB and a fixed-layout PDF agree on where you are.",
  },
  {
    mark: "03",
    title: "One binary, one folder",
    body: "SQLite for metadata, a content-addressed folder for the books. No external account, no sync service in the middle.",
  },
];

export function LandingPage() {
  const router = useRouter();
  const reduce = useReducedMotion();
  const { state } = useAuth();
  const version = useQuery({ queryKey: ["version"], queryFn: getVersion, staleTime: 5 * 60_000 });

  // A signed-in reader has no use for a pitch.
  useEffect(() => {
    if (state.status === "authenticated") router.replace("/library/");
  }, [router, state.status]);

  const firstRun = version.data?.registration === "open";
  const closed = version.data?.registration === "closed";
  const cta = firstRun ? "Set up this server" : "Create an account";

  const rise = (delay: number) =>
    reduce
      ? {}
      : {
          initial: { opacity: 0, y: 12 },
          animate: { opacity: 1, y: 0 },
          transition: { duration: 0.45, delay, ease: [0.23, 1, 0.32, 1] as const },
        };

  return (
    <div className="mx-auto w-full max-w-6xl px-6">
      <section className="grid items-center gap-14 py-16 lg:grid-cols-[minmax(0,1fr)_360px] lg:gap-20 lg:py-24">
        <div>
          <motion.p
            {...rise(0)}
            className="flex items-center gap-2 font-mono text-[11px] uppercase tracking-[0.18em] text-muted-foreground"
          >
            <span aria-hidden="true" className="size-1.5 rounded-full bg-primary" />
            self-hosted · AGPL-3.0
          </motion.p>

          <h1 className="mt-6 max-w-[16ch] font-serif text-[clamp(2.75rem,6vw,4.5rem)] leading-[0.95] tracking-[-0.01em]">
            <motion.span {...rise(0.06)} className="block">
              Your books,
            </motion.span>
            <motion.span {...rise(0.12)} className="block">
              on <em className="italic text-primary">every</em>
            </motion.span>
            <motion.span {...rise(0.18)} className="block">
              device.
            </motion.span>
          </h1>

          <motion.p
            {...rise(0.24)}
            className="mt-7 max-w-[46ch] text-[15px] leading-7 text-muted-foreground"
          >
            leaflet runs on hardware you own. Upload the books you already bought, shelf them into
            collections, and pick up on any device exactly where you stopped reading.
          </motion.p>

          <motion.div {...rise(0.3)} className="mt-9 flex flex-wrap items-center gap-3">
            {closed ? (
              <LinkButton href="/login/" className="h-11 px-6">
                Sign in
              </LinkButton>
            ) : (
              <>
                <LinkButton href="/onboarding/" className="h-11 px-6">
                  {cta}
                </LinkButton>
                <LinkButton href="/login/" variant="ghost" className="h-11 px-5">
                  I already have an account
                </LinkButton>
              </>
            )}
          </motion.div>

          <motion.p {...rise(0.36)} className="mt-6 text-xs text-muted-foreground">
            {firstRun
              ? "This server has no accounts yet — the first one becomes the admin."
              : closed
                ? "This server is not accepting new accounts."
                : "Registration on this server needs an invite code."}
          </motion.p>
        </div>

        <BookArtifact />
      </section>

      <section aria-label="What leaflet does" className="border-t border-border py-14">
        <dl className="grid gap-10 sm:grid-cols-3 sm:gap-8">
          {CAPABILITIES.map(({ mark, title, body }) => (
            <div key={mark} className="space-y-2.5">
              <span className="block font-mono text-[11px] tabular tracking-[0.1em] text-muted-foreground">
                {mark}
              </span>
              <dt className="font-serif text-xl leading-tight">{title}</dt>
              <dd className="max-w-[38ch] text-sm leading-6 text-muted-foreground">{body}</dd>
            </div>
          ))}
        </dl>
      </section>

      <section
        aria-label="What it will not do"
        className="flex flex-wrap items-baseline gap-x-8 gap-y-2 border-t border-border py-8 font-mono text-[11px] uppercase tracking-[0.16em] text-muted-foreground"
      >
        <span className="text-foreground">Never</span>
        <span>a store</span>
        <span>DRM removal</span>
        <span>an account elsewhere</span>
        <span>telemetry</span>
      </section>

      <section className="flex flex-wrap items-end justify-between gap-6 border-t border-border py-16">
        <p className="max-w-[22ch] font-serif text-[clamp(1.75rem,3.5vw,2.75rem)] leading-[1.05]">
          Start with one book.
        </p>
        <div className="flex flex-wrap gap-3">
          {!closed && (
            <LinkButton href="/onboarding/" className="h-11 px-6">
              {cta}
            </LinkButton>
          )}
          <LinkButton href="/login/" variant="secondary" className="h-11 px-6">
            Sign in
          </LinkButton>
        </div>
      </section>
    </div>
  );
}

/** The one decorative artifact on the page: a book, because this is a book server. */
function BookArtifact() {
  const reduce = useReducedMotion();

  return (
    <motion.figure
      initial={reduce ? undefined : { opacity: 0, y: 18 }}
      animate={reduce ? undefined : { opacity: 1, y: 0 }}
      transition={{ duration: 0.6, delay: 0.15, ease: [0.23, 1, 0.32, 1] }}
      className="mx-auto w-full max-w-[260px] lg:mx-0"
    >
      <div className="relative">
        {/* page block, offset twice for edges */}
        <div className="absolute inset-y-1.5 -right-2 w-full rounded-r-md border border-border bg-card/60" />
        <div className="absolute inset-y-0.5 -right-1 w-full rounded-r-md border border-border bg-card" />

        <div
          className="relative aspect-2/3 w-full overflow-hidden rounded-l-[3px] rounded-r-md"
          style={{
            background:
              "linear-gradient(112deg, oklch(0.31 0.05 153) 0%, oklch(0.36 0.055 152) 46%, oklch(0.3 0.05 154) 100%)",
            boxShadow:
              "0 1px 0 oklch(1 0 0 / 0.12) inset, 0 -1px 0 oklch(0 0 0 / 0.35) inset, 0 28px 60px -28px oklch(0.08 0.02 150 / 0.85)",
          }}
        >
          {/* spine */}
          <span
            aria-hidden="true"
            className="absolute inset-y-0 left-0 w-2.5"
            style={{ background: "oklch(0.26 0.045 154 / 0.9)" }}
          />
          <span aria-hidden="true" className="absolute inset-y-0 left-2.5 w-px bg-white/15" />
          {/* paper grain: one hairline, not a texture soup */}
          <span
            aria-hidden="true"
            className="absolute inset-y-0 right-2.5 w-px"
            style={{ background: "oklch(1 0 0 / 0.06)" }}
          />

          <div className="absolute inset-x-6 top-10 text-center">
            <p
              className="font-serif text-[27px] leading-[1.02]"
              style={{ color: "oklch(0.93 0.03 96)" }}
            >
              Collected
              <br />
              Stories
            </p>
            <span
              aria-hidden="true"
              className="mx-auto mt-4 block h-px w-8"
              style={{ background: "oklch(0.93 0.03 96 / 0.35)" }}
            />
            <p
              className="mt-4 font-mono text-[9px] uppercase tracking-[0.22em]"
              style={{ color: "oklch(0.9 0.02 96 / 0.55)" }}
            >
              leaflet library
            </p>
          </div>

          <div className="absolute inset-x-6 bottom-6">
            <div className="h-px w-full" style={{ background: "oklch(0.93 0.03 96 / 0.25)" }} />
            <p
              className="mt-2 text-right font-mono text-[9px] tabular"
              style={{ color: "oklch(0.9 0.02 96 / 0.5)" }}
            >
              03
            </p>
          </div>
        </div>
      </div>

      <figcaption className="mt-6 space-y-2">
        <div className="flex items-baseline justify-between font-mono text-[11px] tabular text-muted-foreground">
          <span>reading position</span>
          <span className="text-foreground">214 / 380</span>
        </div>
        <div
          role="presentation"
          className="h-0.5 w-full overflow-hidden rounded-full bg-border"
        >
          <div className="h-full w-[56%] rounded-full bg-primary" />
        </div>
        <p className="text-xs text-muted-foreground">
          34 minutes left · last read on this device
        </p>
      </figcaption>
    </motion.figure>
  );
}
