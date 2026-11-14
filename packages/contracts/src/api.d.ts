// Generated from openapi.json by packages/contracts/scripts/generate.mjs.

export type AccessToken = {
    access_expires_at: string;
    access_token: string;
  };

export type AuthSession = {
    access_expires_at: string;
    access_token: string;
    user: User;
  };

export type Health = {
    database: string;
    status: string;
  };

export type LoginRequest = {
    email: string;
    password: string;
  };

export type Problem = {
    detail?: string | null;
    status: number;
    title: string;
    type: string;
  };

export type RegisterRequest = {
    display_name: string;
    email: string;
    invite_code?: string | null;
    password: string;
  };

export type RegistrationMode = "open" | "invite" | "closed";

export type Role = "admin" | "member";

export type User = {
    created_at: string;
    display_name: string;
    email: string;
    id: string;
    role: Role;
  };

export type Version = {
    api: string;
    name: string;
    registration: RegistrationMode;
    schema: string;
    version: string;
  };

export type Operations = {
  "POST /api/v1/auth/login": AuthSession;
  "POST /api/v1/auth/logout": unknown;
  "POST /api/v1/auth/refresh": AccessToken;
  "POST /api/v1/auth/register": unknown;
  "GET /api/v1/me": User;
  "GET /api/v1/version": Version;
  "GET /healthz": Health;
};
