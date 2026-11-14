// Generated from openapi.json by packages/contracts/scripts/generate.mjs.

export type Health = {
    database: string;
    status: string;
  };

export type Problem = {
    detail?: unknown;
    status: number;
    title: string;
    type: string;
  };

export type Version = {
    api: string;
    name: string;
    schema: string;
    version: string;
  };

export type Operations = {
  "GET /api/v1/version": Version;
  "GET /healthz": Health;
};
