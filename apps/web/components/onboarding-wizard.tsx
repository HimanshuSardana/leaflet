"use client";

import { useQuery } from "@tanstack/react-query";
import { AnimatePresence, motion } from "motion/react";
import { useRouter } from "next/navigation";
import { useState } from "react";

import { Alert } from "@/components/motion/alert";
import { AnimatedBadge } from "@/components/motion/animated-badge";
import { StatefulButton } from "@/components/motion/button/stateful";
import { Card } from "@/components/motion/card";
import { Input } from "@/components/motion/input";
import { Loader } from "@/components/motion/loader";
import { ApiProblem, getVersion, type RegistrationMode } from "@/lib/api";
import { useAuth } from "@/lib/auth";
import {
  confirmationProblem,
  displayNameProblem,
  emailProblem,
  passwordProblem,
} from "@/lib/validation";

type Step = "welcome" | "identity" | "password" | "done";

const STEPS: Step[] = ["welcome", "identity", "password"];

/** The wizard only ever renders for `open` (bootstrap admin) or `invite`. */
export function OnboardingWizard() {
  const router = useRouter();
  const { register } = useAuth();
  const version = useQuery({ queryKey: ["version"], queryFn: getVersion });

  const [step, setStep] = useState<Step>("welcome");
  const [inviteCode, setInviteCode] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [confirmation, setConfirmation] = useState("");
  const [touched, setTouched] = useState<Record<string, boolean>>({});
  const [serverError, setServerError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  if (version.isPending) {
    return (
      <Shell>
        <Loader variant="bars" size={28} />
      </Shell>
    );
  }

  if (version.isError) {
    return (
      <Shell>
        <Alert variant="destructive" title="Cannot reach the server">
          {version.error instanceof Error ? version.error.message : "Unknown error"}
        </Alert>
      </Shell>
    );
  }

  const mode: RegistrationMode = version.data.registration;
  if (mode === "closed") {
    return (
      <Shell>
        <Alert variant="warning" title="This server is not accepting new accounts">
          An admin has to create your account, or you can sign in with one you already have.
        </Alert>
        <StatefulButton onClick={() => router.push("/login/")}>Go to sign in</StatefulButton>
      </Shell>
    );
  }

  const identityErrors = {
    email: touched.email ? emailProblem(email) : null,
    displayName: touched.displayName ? displayNameProblem(displayName) : null,
    invite: touched.invite && mode === "invite" && !inviteCode.trim() ? "Enter your invite code" : null,
  };
  const passwordErrors = {
    password: touched.password ? passwordProblem(password) : null,
    confirmation: touched.confirmation ? confirmationProblem(password, confirmation) : null,
  };

  const identityComplete =
    !emailProblem(email) &&
    !displayNameProblem(displayName) &&
    (mode !== "invite" || inviteCode.trim().length > 0);
  const passwordComplete = !passwordProblem(password) && !confirmationProblem(password, confirmation);

  const submit = async () => {
    setBusy(true);
    setServerError(null);
    try {
      await register({
        email: email.trim(),
        display_name: displayName.trim(),
        password,
        invite_code: mode === "invite" ? inviteCode.trim() : null,
      });
      setStep("done");
    } catch (error) {
      const problem = error instanceof ApiProblem ? error : null;
      setServerError(problem?.message ?? "Something went wrong. Try again.");
      // A 409 means the account already exists: send them to sign in instead of
      // letting them edit a form that can never succeed.
      if (problem?.status === 409) setStep("identity");
    } finally {
      setBusy(false);
    }
  };

  return (
    <Shell>
      <div className="flex items-center justify-between gap-3">
        <AnimatedBadge status={mode === "open" ? "info" : "neutral"}>
          {mode === "open" ? "New server" : "Invite only"}
        </AnimatedBadge>
        <ol className="flex items-center gap-1.5" aria-label="progress">
          {STEPS.map((name) => (
            <li
              key={name}
              aria-current={name === step ? "step" : undefined}
              className={`h-1.5 w-8 rounded-full transition-colors ${
                STEPS.indexOf(step) >= STEPS.indexOf(name) ? "bg-primary" : "bg-border"
              }`}
            />
          ))}
        </ol>
      </div>

      <AnimatePresence mode="wait" initial={false}>
        <motion.div
          key={step}
          initial={{ opacity: 0, y: 8 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -8 }}
          transition={{ duration: 0.2 }}
          className="space-y-5"
        >
          {step === "welcome" && (
            <>
              <div className="space-y-2">
                <h1 className="text-2xl font-semibold tracking-tight">
                  {mode === "open" ? "Set up leaflet" : "Join this leaflet server"}
                </h1>
                <p className="text-sm text-muted-foreground">
                  {mode === "open"
                    ? "No accounts exist yet, so the one you create now becomes the admin. Your library, collections and reading progress stay on this server."
                    : "This server is invite-only. You will need the invite code from whoever runs it."}
                </p>
              </div>
              <StatefulButton onClick={() => setStep("identity")} className="w-full">
                {mode === "open" ? "Create the admin account" : "Continue"}
              </StatefulButton>
              <p className="text-xs text-muted-foreground">
                Already have an account?{" "}
                <a className="text-primary underline-offset-4 hover:underline" href="/login/">
                  Sign in
                </a>
              </p>
            </>
          )}

          {step === "identity" && (
            <>
              <div className="space-y-2">
                <h1 className="text-xl font-semibold tracking-tight">Who is reading?</h1>
                <p className="text-sm text-muted-foreground">
                  This name shows up next to your reading progress.
                </p>
              </div>
              {mode === "invite" && (
                <Input
                  label="Invite code"
                  value={inviteCode}
                  onChange={setInviteCode}
                  onBlur={() => setTouched((t) => ({ ...t, invite: true }))}
                  error={identityErrors.invite ?? false}
                  autoComplete="off"
                />
              )}
              <Input
                label="Display name"
                value={displayName}
                onChange={setDisplayName}
                onBlur={() => setTouched((t) => ({ ...t, displayName: true }))}
                error={identityErrors.displayName ?? false}
                autoComplete="nickname"
              />
              <Input
                label="Email"
                type="email"
                value={email}
                onChange={setEmail}
                onBlur={() => setTouched((t) => ({ ...t, email: true }))}
                error={identityErrors.email ?? false}
                autoComplete="email"
              />
              <div className="flex gap-3">
                <StatefulButton variant="secondary" onClick={() => setStep("welcome")} className="flex-1">
                  Back
                </StatefulButton>
                <StatefulButton
                  onClick={() => setStep("password")}
                  disabled={!identityComplete}
                  className="flex-1"
                >
                  Continue
                </StatefulButton>
              </div>
            </>
          )}

          {step === "password" && (
            <>
              <div className="space-y-2">
                <h1 className="text-xl font-semibold tracking-tight">Choose a password</h1>
                <p className="text-sm text-muted-foreground">
                  At least 8 characters. It is stored as an argon2id hash, never in the clear.
                </p>
              </div>
              <Input
                label="Password"
                type="password"
                value={password}
                onChange={setPassword}
                onBlur={() => setTouched((t) => ({ ...t, password: true }))}
                error={passwordErrors.password ?? false}
                success={passwordComplete}
                autoComplete="new-password"
              />
              <Input
                label="Repeat password"
                type="password"
                value={confirmation}
                onChange={setConfirmation}
                onBlur={() => setTouched((t) => ({ ...t, confirmation: true }))}
                error={passwordErrors.confirmation ?? false}
                success={!confirmationProblem(password, confirmation) && confirmation.length > 0}
                autoComplete="new-password"
              />
              {serverError && (
                <Alert variant="destructive" title="Could not create the account">
                  {serverError}
                </Alert>
              )}
              <div className="flex gap-3">
                <StatefulButton
                  variant="secondary"
                  onClick={() => setStep("identity")}
                  className="flex-1"
                  disabled={busy}
                >
                  Back
                </StatefulButton>
                <StatefulButton
                  onClick={() => void submit()}
                  disabled={!passwordComplete}
                  state={busy ? "loading" : "idle"}
                  className="flex-1"
                >
                  {mode === "open" ? "Create admin account" : "Create account"}
                </StatefulButton>
              </div>
            </>
          )}

          {step === "done" && (
            <>
              <div className="space-y-2">
                <h1 className="text-2xl font-semibold tracking-tight">You are in</h1>
                <p className="text-sm text-muted-foreground">
                  {mode === "open"
                    ? "This account is the server admin. Next: upload a book, or invite other readers by setting an invite code."
                    : "Your account is ready. Your library starts empty until you upload something."}
                </p>
              </div>
              <StatefulButton onClick={() => router.push("/library/")} className="w-full">
                Open my library
              </StatefulButton>
            </>
          )}
        </motion.div>
      </AnimatePresence>
    </Shell>
  );
}

function Shell({ children }: { children: React.ReactNode }) {
  return (
    <Card className="mx-auto mt-6 max-w-md p-7">
      <div className="space-y-5">{children}</div>
    </Card>
  );
}
