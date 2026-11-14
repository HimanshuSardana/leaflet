"use client";

import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from "react";

import {
  ApiProblem,
  type AuthSession,
  type RegisterBody,
  type User,
  currentUser,
  login as loginRequest,
  logout as logoutRequest,
  refreshSession,
  registerAccount,
} from "./api";

type AuthState =
  | { status: "loading" }
  | { status: "anonymous" }
  | { status: "authenticated"; user: User; accessToken: string };

type AuthContextValue = {
  state: AuthState;
  register: (body: RegisterBody) => Promise<void>;
  login: (email: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
};

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [state, setState] = useState<AuthState>({ status: "loading" });

  // Refresh rotates and revokes the session token, so two concurrent restores
  // would cancel each other out: the loser presents a token the winner just
  // rotated away and is told 401. One request serves every caller.
  const restoring = useRef<Promise<AuthState> | null>(null);

  /**
   * The access token lives in memory only, so a reload starts here: exchange the
   * httpOnly session cookie for a fresh token. No cookie → anonymous.
   */
  const restore = useCallback(async (signal: AbortSignal) => {
    try {
      restoring.current ??= (async () => {
        const { access_token } = await refreshSession();
        const user = await currentUser(access_token);
        return { status: "authenticated", user, accessToken: access_token } as AuthState;
      })().finally(() => {
        restoring.current = null;
      });

      const next = await restoring.current;
      if (!signal.aborted) setState(next);
    } catch (error) {
      if (signal.aborted) return;
      // 401 means "no valid session" — anything else is a server problem worth
      // seeing in the console, but the UI still has to render as anonymous.
      if (!(error instanceof ApiProblem && error.isAuthFailure)) {
        console.warn("session restore failed", error);
      }
      setState({ status: "anonymous" });
    }
  }, []);

  useEffect(() => {
    const controller = new AbortController();
    void restore(controller.signal);
    return () => controller.abort();
  }, [restore]);

  const adopt = useCallback((session: AuthSession) => {
    setState({ status: "authenticated", user: session.user, accessToken: session.access_token });
  }, []);

  const value = useMemo<AuthContextValue>(
    () => ({
      state,
      register: async (body) => adopt(await registerAccount(body)),
      login: async (email, password) => adopt(await loginRequest({ email, password })),
      logout: async () => {
        try {
          await logoutRequest();
        } finally {
          setState({ status: "anonymous" });
        }
      },
    }),
    [adopt, state],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthContextValue {
  const context = useContext(AuthContext);
  if (!context) throw new Error("useAuth must be used inside <AuthProvider>");
  return context;
}
