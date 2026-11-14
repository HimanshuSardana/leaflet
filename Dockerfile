# syntax=docker/dockerfile:1
# Two stages: the web export, then a single rust binary that serves it.
# SQLite + blobs live on /data, so the container itself is stateless.

FROM node:22-alpine AS web
WORKDIR /build
ENV NEXT_TELEMETRY_DISABLED=1
COPY pnpm-lock.yaml pnpm-workspace.yaml package.json ./
COPY apps/web/package.json apps/web/
COPY packages/contracts/package.json packages/contracts/
RUN corepack enable && pnpm install --frozen-lockfile
COPY packages/contracts packages/contracts
COPY apps/web apps/web
RUN pnpm --filter web build

FROM rust:1.99-bookworm AS server
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY apps/server/Cargo.toml apps/server/
COPY apps/server/src apps/server/src
COPY apps/server/migrations apps/server/migrations
COPY apps/server/tests apps/server/tests
COPY --from=web /build/apps/web/out apps/web/out
RUN cargo build --release -p leaflet-server

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*
COPY --from=server /build/target/release/leaflet-server /usr/local/bin/leaflet-server
COPY --from=web /build/apps/web/out /srv/web
RUN useradd --system --uid 10001 --home-dir /data leaflet \
 && mkdir -p /data && chown leaflet:leaflet /data
USER leaflet
ENV LEAFLET_DATA_DIR=/data \
    LEAFLET_WEB_DIR=/srv/web \
    LEAFLET_BIND=0.0.0.0:8080 \
    LEAFLET_LOG=info,tower_http=warn
VOLUME ["/data"]
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/leaflet-server"]
