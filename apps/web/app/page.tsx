import { LinkButton } from "@/components/link-button";
import { TextReveal } from "@/components/motion/text-reveal";

export default function Home() {
  return (
    <div className="mx-auto flex w-full max-w-2xl flex-col items-start gap-6 px-6 py-20">
      <TextReveal
        as="h1"
        className="text-4xl font-semibold leading-tight"
        text="Your books, on every device."
        stagger={0.05}
        blur={6}
      />
      <p className="max-w-prose text-sm text-muted-foreground">
        leaflet keeps PDFs and EPUBs on your own server and remembers exactly where you stopped —
        on the laptop, on the phone, wherever you open it next.
      </p>
      <div className="flex gap-3">
        <LinkButton href="/library/">Open the library</LinkButton>
        <LinkButton href="/onboarding/" variant="secondary">
          Create an account
        </LinkButton>
      </div>
    </div>
  );
}
