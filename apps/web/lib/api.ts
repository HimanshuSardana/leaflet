/** Same-origin by construction: in prod the rust binary serves this export. */
import type {
  AuthSession,
  Book,
  Problem,
  RegistrationMode,
  User,
  Version,
} from "@leaflet/contracts";

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

export const listBooks = (accessToken: string) => request<Book[]>("/books", { accessToken });

/** Same-origin URL of a book's bytes; needs an access token, so not linkable yet. */
export const bookFileUrl = (id: string) => `${BASE}/books/${id}/file`;

export type UploadResult = { book: Book; created: boolean };

/**
 * Uploads one file. Uses XMLHttpRequest rather than fetch because a book can be
 * hundreds of megabytes and fetch still cannot report upload progress. Resolves
 * with `created: false` when the server already had these exact bytes.
 */
export function uploadBook(
  file: File,
  accessToken: string,
  options: { onProgress?: (fraction: number) => void; signal?: AbortSignal } = {},
): Promise<UploadResult> {
  const { onProgress, signal } = options;

  return new Promise<UploadResult>((resolve, reject) => {
    if (signal?.aborted) {
      reject(new ApiProblem(0, { title: "Upload cancelled" }));
      return;
    }

    const xhr = new XMLHttpRequest();
    xhr.open("POST", `${BASE}/books`);
    xhr.setRequestHeader("authorization", `Bearer ${accessToken}`);
    xhr.responseType = "text";
    xhr.withCredentials = true;

    const abort = () => xhr.abort();
    signal?.addEventListener("abort", abort);

    const settle = () => signal?.removeEventListener("abort", abort);

    xhr.upload.onprogress = (event) => {
      if (event.lengthComputable) onProgress?.(event.loaded / event.total);
    };

    xhr.onload = () => {
      settle();
      let body: unknown = null;
      try {
        body = xhr.responseText ? JSON.parse(xhr.responseText) : null;
      } catch {
        body = null;
      }
      if (xhr.status >= 200 && xhr.status < 300 && body) {
        resolve({ book: body as Book, created: xhr.status === 201 });
      } else {
        reject(new ApiProblem(xhr.status, (body ?? {}) as Partial<Problem>));
      }
    };

    xhr.onerror = () => {
      settle();
      reject(new ApiProblem(0, { title: "Could not reach the server" }));
    };

    xhr.onabort = () => {
      settle();
      reject(new ApiProblem(0, { title: "Upload cancelled" }));
    };

    const form = new FormData();
    form.append("file", file, file.name);
    xhr.send(form);
  });
}

export type { AuthSession, Book, Problem, RegistrationMode, User, Version };
