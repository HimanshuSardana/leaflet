"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";

import {
  Alert,
  AlertContent,
  AlertDescription,
  AlertIcon,
  AlertTitle,
} from "@/components/motion/alert";
import { StatefulButton } from "@/components/motion/button/stateful";
import { Card } from "@/components/motion/card";
import { Input } from "@/components/motion/input";
import { useToast } from "@/components/toast";
import { ApiProblem } from "@/lib/api";
import { useAuth } from "@/lib/auth";
import { emailProblem, passwordProblem } from "@/lib/validation";

export function LoginForm() {
  const router = useRouter();
  const { login } = useAuth();
  const { toast } = useToast();

  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [touched, setTouched] = useState<{ email?: boolean; password?: boolean }>({});
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const emailError = touched.email ? emailProblem(email) : null;
  const passwordError = touched.password ? passwordProblem(password) : null;
  const ready = !emailProblem(email) && !passwordProblem(password);

  const submit = async () => {
    setBusy(true);
    setError(null);
    try {
      await login(email.trim(), password);
      toast("success", "Signed in");
      router.push("/library/");
    } catch (caught) {
      const problem = caught instanceof ApiProblem ? caught : null;
      setError(
        problem?.status === 401
          ? "Wrong email or password."
          : (problem?.message ?? "Could not sign in. Try again."),
      );
    } finally {
      setBusy(false);
    }
  };

  return (
    <Card className="mx-auto mt-10 w-full max-w-md p-7">
      <form
        className="space-y-5"
        onSubmit={(event) => {
          event.preventDefault();
          if (ready) void submit();
        }}
      >
        <div className="space-y-2">
          <h1 className="text-2xl font-semibold">Sign in</h1>
          <p className="text-sm text-muted-foreground">
            Your reading position follows you from device to device.
          </p>
        </div>

        <Input
          label="Email"
          type="email"
          value={email}
          onChange={setEmail}
          onBlur={() => setTouched((t) => ({ ...t, email: true }))}
          error={emailError ?? false}
          autoComplete="email"
        />
        <Input
          label="Password"
          type="password"
          value={password}
          onChange={setPassword}
          onBlur={() => setTouched((t) => ({ ...t, password: true }))}
          error={passwordError ?? false}
          autoComplete="current-password"
        />

        {error && (
          <Alert variant="destructive">
            <AlertIcon />
            <AlertContent>
              <AlertTitle>Sign in failed</AlertTitle>
              <AlertDescription>{error}</AlertDescription>
          </AlertContent>
          </Alert>
        )}

        <StatefulButton
          type="submit"
          className="w-full"
          disabled={!ready}
          state={busy ? "loading" : "idle"}
        >
          Sign in
        </StatefulButton>
      </form>
    </Card>
  );
}
