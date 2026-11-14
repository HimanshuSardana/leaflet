# leaflet

Self-hostable ebook, manga and light-novel server. Upload PDFs/EPUBs, shelve them
into collections, read in the browser, and keep your reading position in sync
across devices.

Scaffolding stage: the bones boot, the schema exists, no book logic yet.

## Run it

```sh
pnpm install && cargo fetch
make dev            # api :8080, web :3000 (proxying /api to the api)
```

Or as one container, which serves both from a single origin:

```sh
docker compose up --build   # http://localhost:8080
```

Data (SQLite + book blobs) lives on the `/data` volume.

## Layout

```
apps/server       rust API, serves the web export, owns SQLite + blobs
apps/web          next.js static export (no SSR: one origin, one binary)
packages/contracts  openapi.json + generated TS types (do not hand-edit)
```

`make` lists the rest: `make check`, `make test-rust`, `make test-web`,
`make contract`, `make build`.

## How it fits together

- **Same origin always.** The SPA talks to `/api/v1/*` relatively; in dev Next
  rewrites to `:8080`, in prod the Rust binary serves `apps/web/out`. No CORS, no
  build-time API URL.
- **SQLite, single writer.** WAL, `foreign_keys`, `busy_timeout`, one pooled
  connection so writes queue instead of failing. Migrations run on boot.
- **Content-addressed blobs.** `<data>/blobs/ab/cd/<sha256>`; identical uploads
  share bytes, and a blob is deleted only when no book row references it.
- **Progress is the product.** `reading_progress` stores a device-native locator
  (`{href, cfi|page|scroll_pct}`) plus a normalized `0..1` fraction, so a
  reflowable EPUB and a fixed-layout PDF agree on "where you are".
- **The contract is generated.** `utoipa` emits the OpenAPI spec from the Rust
  handlers; `packages/contracts` compiles it to TS. CI fails on drift.

## Status

Done: workspace, config, health/version endpoints, v1 schema, storage layout,
web shell, generated contract, container, CI.

Next: uploads (sha256 streaming, OPF metadata, validation), auth handlers
(multi-user, closed registration), reader engines, Android client.

AGPL-3.0-only.
