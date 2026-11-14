/** Same-origin by construction: in prod the rust binary serves this export. */
import type { AuthSession, Problem, RegistrationMode, User, Version } from "@leaflet/contracts";

const BASE = "/api/v1";

/** A non-2xx answer, already decoded from RFC 9457 problem+json. */
export class ApiProblem extends Error {
  readonly status: number;
  readonly kind: string;

  constructor(status: number, problem: Partial<Problem>) {
    super(problem.detail ?? problem.title ?? `request failed (${status})`);
    this.name = "ApiProblem";
    this.status = status;
    this.kind = problem.type ?? "unknown";
  }

  /** Auth problems the UI must not treat as "your data is wrong". */
  get isAuthFailure(): boolean {
    return this.status === 401;
  }
}

async function readProblem(response: Response): Promise<ApiProblem> {
  try {
    return new ApiProblem(response.status, (await response.json()) as Problem);
  } catch {
    return new ApiProblem(response.status, { title: response.statusText });
  }
}

type RequestOptions = {
  method?: "GET" | "POST" | "DELETE";
  body?: unknown;
  accessToken?: string | null;
  signal?: AbortSignal;
};

export async function request<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const { method = "GET", body, accessToken, signal } = options;
  const headers: Record<string, string> = { accept: "application/json" };
  if (body !== undefined) headers["content-type"] = "application/json";
  if (accessToken) headers.authorization = `Bearer ${accessToken}`;

  const response = await fetch(`${BASE}${path}`, {
    method,
    headers,
    signal,
    // The refresh cookie is scoped to /api/v1/auth and must ride along.
    credentials: "same-origin",
    body: body === undefined ? undefined : JSON.stringify(body),
  });

  if (!response.ok) throw await readProblem(response);
  if (response.status === 204) return undefined as T;
  return (await response.json()) as T;
}

export const getVersion = () => request<Version>("/version");
export const getProblem = (error: unknown) =>
  error instanceof ApiProblem ? error : null;

export type RegisterBody = {
  email: string;
  display_name: string;
  password: string;
  invite_code?: string | null;
};

export const registerAccount = (body: RegisterBody) =>
  request<AuthSession>("/auth/register", { method: "POST", body });

export const login = (body: { email: string; password: string }) =>
  request<AuthSession>("/auth/login", { method: "POST", body });

export const refreshSession = () =>
  request<{ access_token: string; access_expires_at: string }>("/auth/refresh", {
    method: "POST",
  });

export const logout = () => request<void>("/auth/logout", { method: "POST" });

export const currentUser = (accessToken: string) =>
  request<User>("/me", { accessToken });

export type { AuthSession, Problem, RegistrationMode, User, Version };
