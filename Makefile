# leaflet - self-hostable ebook/manga/light-novel server
.DEFAULT_GOAL := help
SHELL := /bin/bash

RUST_SOURCES := $(shell git ls-files '*.rs')

help: ## list targets
	@grep -hE '^[a-z-]+:.*?## ' $(MAKEFILE_LIST) | awk -F':.*?## ' '{printf "  %-16s %s\n", $$1, $$2}'

setup: ## install toolchain deps
	cargo fetch
	pnpm install

dev: ## run api (:8080) and web (:3000) together
	@echo "api -> http://localhost:8080  web -> http://localhost:3000"
	@trap 'kill 0' EXIT; cargo run -p leaflet-server & pnpm --filter web dev

fmt: ## check rust formatting
	rustfmt --edition 2024 --check $(RUST_SOURCES)

fmt-fix: ## apply rust formatting
	rustfmt --edition 2024 $(RUST_SOURCES)

check: fmt ## fmt + clippy
	cargo clippy --all-targets -- -D warnings

test-rust: ## rust tests
	cargo test --all

test-web: ## web typecheck + tests
	pnpm --filter web typecheck
	pnpm --filter web test

ci: check test-rust test-web contract-check ## what CI runs

contract: ## regenerate the typed api contract from the rust spec
	cargo run -p leaflet-server --bin gen-openapi
	pnpm --filter @leaflet/contracts build

contract-check: ## fail if the committed contract is stale
	@git diff --quiet -- packages/contracts/src || { echo "contract is stale: run 'make contract'"; exit 1; }

build: ## release binary (web export built first)
	pnpm --filter web build
	cargo build --release -p leaflet-server

.PHONY: help setup dev fmt fmt-fix check test-rust test-web ci contract contract-check build
