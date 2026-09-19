# apidiff

Diffs two OpenAPI specs and reports breaking changes — a CI gate on your
API contract. Go's `oasdiff` is the closest existing tool; no standalone
Rust equivalent.

## Usage

```bash
apidiff old-spec.json new-spec.json
```

Exit code `1` if any breaking change is found — drop it in CI comparing
the spec on `main` against a PR branch's spec (most frameworks that
auto-generate OpenAPI — FastAPI, NestJS, `utoipa` for Rust — can dump
`openapi.json` as a build step).

## What counts as breaking (and what doesn't)

Only changes that can actually break an *existing* client, not every
difference between the two specs:

| Change | Breaking? |
|---|---|
| Endpoint removed | ✅ yes |
| Endpoint added | ❌ no — new, nothing depends on it yet |
| New **required** parameter | ✅ yes — existing clients don't send it |
| New **optional** parameter | ❌ no — additive |
| Parameter optional → required | ✅ yes |
| Parameter required → optional | ❌ no — strictly more permissive |
| Parameter removed | ❌ no — a client that stops sending it is fine |

## Status: built and verified against two realistic specs with three real breaking changes

- **14 unit tests** (`cargo test --lib`): spec parsing (paths/methods
  extracted from real OpenAPI JSON shape, an operation with no
  `parameters` key correctly an empty list rather than an error, non-HTTP-
  method keys at the path-item level like a shared `parameters` block
  correctly ignored rather than mistaken for a method); and every row of
  the breaking/not-breaking table above as its own test — including the
  two "looks similar but isn't the same rule" pairs (required→optional is
  safe, optional→required isn't; endpoint *removed* is breaking, endpoint
  *added* isn't) that are the easiest cases to get backwards by accident.
- **CLI run against two realistic specs with three real breaking
  changes**: a v1→v2 spec pair where `DELETE /users/{id}` was removed,
  `GET /users`'s `limit` parameter went from optional to required, and a
  new required `api_key` header parameter was added — all three
  correctly identified and named in one run, exit code 1. The same spec
  diffed against itself: `no breaking changes`, exit code 0.

## Scope

A pragmatic, partial OpenAPI model, not a full spec-compliant parser: no
`$ref` resolution (a parameter defined via `{"$ref": "#/components/..."}`
is skipped, not resolved — noted explicitly in `spec.rs`, not a silent
gap), JSON only (most auto-generated specs are JSON; YAML would need an
extra dependency for real spec-shape parsing, not attempted in this v1),
and only path/method/required-parameter changes — no request/response
**body schema** diffing (a required field added deep inside a JSON Schema
request body is a real, common breaking change this doesn't catch yet;
schema-level diffing is a substantially bigger v2, not attempted here to
ship a working, correctly-scoped v1 instead).
