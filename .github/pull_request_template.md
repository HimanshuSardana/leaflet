## What

<!-- one paragraph: what changes, why -->

## Scope check

- [ ] One concern. If this touches api + web + schema, it probably wants splitting.
- [ ] Under ~400 changed lines of hand-written code.
- [ ] `make check test-rust test-web contract-check` passes locally.

## Contract / schema

- [ ] No API shape change, or: `openapi.json` + `api.d.ts` regenerated via `make contract`.
- [ ] No migration, or: migration is additive and named `NNNN_<what>.sql`.

## Verification

<!-- commands run, endpoints hit, screenshots for ui -->
