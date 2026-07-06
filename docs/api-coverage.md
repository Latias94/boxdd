# Box2D API Coverage

<!-- api-coverage: total=430 safe=424 raw=4 omitted=2 deferred=0 -->

This document summarizes how `boxdd` accounts for every vendored `B2_API` function under `boxdd-sys/third-party/box2d/include/box2d`.
The authoritative per-symbol fixture is `boxdd/tests/fixtures/api_coverage_symbols.txt`, and `cargo nextest run -p boxdd --test api_coverage` validates that it matches the vendored headers and this summary.

## Status Values

- `safe`: represented by the Rust safe layer.
- `raw`: available through `boxdd_sys::ffi`; no stable safe wrapper is assigned yet.
- `omitted`: intentionally excluded from the safe layer with rationale.
- `deferred`: planned but not yet implemented.

## Summary

| Status | Count |
|---|---:|
| `safe` | 424 |
| `raw` | 4 |
| `omitted` | 2 |
| `deferred` | 0 |
| Total | 430 |

## By Surface

| Surface | Safe | Raw | Omitted | Deferred | Total |
|---|---:|---:|---:|---:|---:|
| Body | 68 | 0 | 0 | 0 | 68 |
| Chain | 13 | 0 | 0 | 0 | 13 |
| Collision | 21 | 0 | 0 | 0 | 21 |
| DynamicTree | 21 | 0 | 0 | 0 | 21 |
| Foundation | 38 | 4 | 0 | 0 | 42 |
| Joint | 151 | 0 | 0 | 0 | 151 |
| Math | 10 | 0 | 0 | 0 | 10 |
| Shape | 60 | 0 | 0 | 0 | 60 |
| World | 42 | 0 | 2 | 0 | 44 |

## Maintenance

- Run `cargo run -p xtask -- api-coverage --write` after changing vendored Box2D or adding safe wrappers.
- Review every `raw`, `omitted`, and `deferred` row before a breaking release.
- Treat vendored headers as the exact API source; online Box2D docs may describe a nearby but different release.
