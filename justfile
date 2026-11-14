# leaflet - self-hostable ebook/manga server
set shell := ["bash", "-euo", "pipefail", "-c"]

default:
    @just --list

# install toolchain deps (rust components, node deps)
setup:
    cargo fetch
    pnpm install

# run api (:8080) and web (:3000) together
dev:
    @echo "api -> http://localhost:8080  web -> http://localhost:3000"
    @trap 'kill 0' EXIT; cargo run -p leaflet-server & pnpm --filter web dev

# cargo check + clippy + fmt
check:
    cargo fmt --all --check
    cargo clippy --all-targets -- -D warnings

# run rust tests
test-rust:
    cargo test --all

# run web tests / typecheck / lint
test-web:
    pnpm --filter web test
    pnpm --filter web typecheck

# everything CI runs
ci: check test-rust test-web contract-check

# regenerate the typed api contract from the rust openapi spec
contract:
    cargo run -p leaflet-server --bin gen-openapi
    pnpm --filter @leaflet/contracts build

# fail if the committed contract is stale
contract-check:
    @git diff --quiet -- packages/contracts/src || (echo "contract is stale: run 'just contract'" && exit 1)

# build the release binary (embeds migrations, serves web export)
build:
    pnpm --filter web build
    cargo build --release -p leaflet-server
