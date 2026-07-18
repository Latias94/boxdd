# Box2D API Coverage

<!-- api-coverage: total=430 safe=423 raw=5 omitted=2 deferred=0 -->

This file is generated from `boxdd/tests/fixtures/api_contract.toml`. The contract is validated against the exact vendored headers, canonical public Rust paths, real `#[test]` evidence, provider modes, precision-specific link symbols, and ABI struct/callback fingerprints.

Pinned active upstream: `c05c48738fbe5c27625e36c5f0cfbdaddfc8359a`.

## Summary

| Status | Count |
|---|---:|
| `safe` | 423 |
| `raw` | 5 |
| `omitted` | 2 |
| `deferred` | 0 |
| Total | 430 |

## By Area

| Area | Safe | Raw | Omitted | Deferred | Total |
|---|---:|---:|---:|---:|---:|
| Body | 68 | 0 | 0 | 0 | 68 |
| Chain | 13 | 0 | 0 | 0 | 13 |
| Collision | 21 | 0 | 0 | 0 | 21 |
| DynamicTree | 21 | 0 | 0 | 0 | 21 |
| Foundation | 38 | 4 | 0 | 0 | 42 |
| Joint | 151 | 0 | 0 | 0 | 151 |
| Math | 9 | 1 | 0 | 0 | 10 |
| Shape | 60 | 0 | 0 | 0 | 60 |
| World | 42 | 0 | 2 | 0 | 44 |

## Non-Safe Capabilities

| Logical API | Status | Area | Rationale |
|---|---|---|---|
| `b2InternalAssertFcn` | `raw` | Foundation | Raw only: upstream internal assert implementation, not a stable safe API surface. |
| `b2SetAllocator` | `raw` | Foundation | Raw only: process-global allocator hook needs a scoped startup guard before safe exposure. |
| `b2SetAssertFcn` | `raw` | Foundation | Raw only: process-global assert callback has panic/callback unwinding semantics that need a dedicated design. |
| `b2SetLengthUnitsPerMeter` | `raw` | Math | Raw during U1: process-global length scale mutation is not synchronized with safe Box2D activity; U6 will replace it with foundation initialization. |
| `b2SetLogFcn` | `raw` | Foundation | Raw only: process-global log callback needs a scoped callback guard before safe exposure. |
| `b2World_DumpMemoryStats` | `omitted` | World | Intentionally omitted: upstream writes fixed diagnostic output, so callers should use upstream diagnostics tooling explicitly. |
| `b2World_RebuildStaticTree` | `omitted` | World | Intentionally omitted: upstream labels this as internal testing support, not stable runtime API. |

## Maintenance

- Run `cargo run -p xtask -- api-coverage --check` to reject header, Rust path, evidence, wire-schema, ABI, or generated-document drift.
- Run `cargo run -p xtask -- api-coverage --write` only to regenerate this report after the structured contract has been reviewed.
- Use `cargo run -p xtask -- upstream-sync --check` before changing the exact upstream revision.
