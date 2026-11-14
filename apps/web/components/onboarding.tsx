"use client";

import { useQuery } from "@tanstack/react-query";
import { useRouter } from "next/navigation";
import { useState } from "react";

import {
  Alert,
  AlertContent,
  AlertDescription,
  AlertIcon,
  AlertTitle,
} from "@/components/motion/alert";
import { LinkButton } from "@/components/link-button";
import { Card } from "@/components/motion/card";
import { Input } from "@/components/motion/input";
import { Loader } from "@/components/motion/loader";
import { SignUpForm } from "@/components/motion/signup-form";
import { ApiProblem, getVersion, type RegistrationMode } from "@/lib/api";
import { useAuth } from "@/lib/auth";
import { useToast } from "@/components/toast";

/**
 * One screen, driven by the server's public registration mode:
 * a fresh server offers the admin account, an invite-only one adds the code
 * field, and a closed one points at sign-in instead of rendering a form that
 * could only ever 403.
 */
export function Onboarding() {
  const router = useRouter();
  const { register } = useAuth();
  const { toast } = useToast();
  const version = useQuery({ queryKey: ["version"], queryFn: getVersion });

  const [inviteCode, setInviteCode] = useState("");
  const [inviteError, setInviteError] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);

  if (version.isPending) {
    return (
      <Centered>
        <Loader variant="bars" size={28} />
      </Centered>
    );
  }

  if (version.isError) {
    return (
      <Centered>
        <Alert variant="destructive">
          <AlertIcon />
          <AlertContent>
            <AlertTitle>Cannot reach the server</AlertTitle>
            <AlertDescription>
            {version.error instanceof Error ? version.error.message : "Unknown error"}
          </AlertDescription>
        </AlertContent>
        </Alert>
      </Centered>
    );
  }

  const mode: RegistrationMode = version.data.registration;

  if (mode === "closed") {
    return (
      <Centered>
        <Card className="w-full max-w-md space-y-4 p-7">
          <h1 className="text-xl font-semibold">This server is closed</h1>
          <Alert variant="warning">
            <AlertIcon />
            <AlertContent>
              <AlertTitle>No new accounts</AlertTitle>
              <AlertDescription>
              An admin has to create your account, or you can sign in with one you already have.
            </AlertDescription>
          </AlertContent>
          </Alert>
          <LinkButton href="/login/" className="w-full">
            Go to sign in
          </LinkButton>
        </Card>
      </Centered>
    );
  }

  return (
    <Centered>
      <div className="w-full max-w-sm space-y-3">
        {mode === "invite" && (
          <div className="rounded-3xl border border-border p-4">
            <Input
              label="Invite code"
              value={inviteCode}
              onChange={(value) => {
                setInviteCode(value);
                if (value.trim()) setInviteError(false);
              }}
              error={inviteError ? "Enter your invite code" : false}
              autoComplete="off"
            />
          </div>
        )}
        <SignUpForm
          title={mode === "open" ? "Set up leaflet" : "Join this server"}
          description={
            mode === "open"
              ? "The first account becomes the admin. Your library, collections and reading progress stay on this server."
              : "This server is invite-only — you will need the code from whoever runs it."
          }
          submitLabel={mode === "open" ? "Create admin account" : "Create account"}
          strengthMeter
          errorMessage={formError ?? undefined}
          labels={{
            nameLabel: "Display name",
            namePlaceholder: "What should we call you?",
            passwordPlaceholder: "At least 8 characters",
            terms: "I understand this account can read and manage this server's books",
          }}
          footer={
            <span className="text-xs text-muted-foreground">
              Already have an account?{" "}
              <a className="text-primary underline-offset-4 hover:underline" href="/login/">
                Sign in
              </a>
            </span>
          }
          validate={(values) => {
            const errors: Record<string, string> = {};
            if (!values.name.trim()) errors.name = "Pick a name";
            if (mode === "invite" && !inviteCode.trim()) setInviteError(true);
            return errors;
          }}
          onSubmit={async (values) => {
            setFormError(null);
            if (mode === "invite" && !inviteCode.trim()) {
              setInviteError(true);
              throw new Error("invite required");
            }
            try {
              await register({
                email: values.email.trim(),
                display_name: values.name.trim(),
                password: values.password,
                invite_code: mode === "invite" ? inviteCode.trim() : null,
              });
              toast("success", "Account created", "Welcome to leaflet.");
              router.push("/library/");
            } catch (error) {
              const detail =
                error instanceof ApiProblem ? error.message : "Could not create the account.";
              setFormError(detail);
              throw new Error(detail);
            }
          }}
        />
      </div>
    </Centered>
  );
}

function Centered({ children }: { children: React.ReactNode }) {
  return <div className="mx-auto grid w-full max-w-md place-items-center px-6 py-10">{children}</div>;
}
