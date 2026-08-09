# Upstream Synchronization

`boxdd-sys/upstream.toml` is the revision authority for the vendored Box2D checkout and the
generated artifacts that depend on it. It records the exact upstream and recording revisions, the
reviewed source inventory, recording inputs, six bindings files, the recording wire contract, and
two WASM provider identities.

## Validate The Current Checkout

```bash
cargo run -p xtask -- upstream-sync --check
cargo run -p xtask -- api-inventory --check
cargo run -p xtask -- recording-wire-codegen --check
```

`upstream-sync --check` is read-only. It checks that:

- the submodule gitlink, checkout, and manifest use the same exact commit;
- the source inventory and reviewed recording inputs still match both the pinned and active commit;
- every reviewed recording input matches its declared post-overlay SHA-256 identity;
- every manifest artifact has the recorded content digest;
- the recording fixture digest matches the canonical provider contract identity;
- the provider identities have the required single- and double-precision coordinates.

`api-inventory` separately checks that every public C function is explicitly classified and that
all checked-in bindings expose the same public function set. Its `safe` list records reviewed intent
that a production public Safe Rust route uses the exact C function; `raw` means the exact entry point
is not a public Safe operation, even when Rust code covers equivalent semantics. The command does
not parse or prove the Safe Rust implementation.

## Update Box2D

1. Check out the intended exact commit in `boxdd-sys/third-party/box2d` and update the parent
   repository gitlink.
2. Update `active_revision` in `boxdd-sys/upstream.toml`. Update `recording_revision` only when the
   reviewed recording sources intentionally move with it.
3. Run `cargo run -p xtask -- upstream-sync --write`.
4. Review the generated bindings, provider identities, manifest, and API inventory.
5. Classify any new C functions in `xtask/api-inventory.toml`; never infer a Safe classification
   from source scanning.
6. Run the three validation commands above and the relevant workspace tests.

Write mode operates only on the current checkout. It does not fetch commits, change submodules,
create worktrees, edit the Git index, or roll back files. Generated files are written atomically,
and ordinary Git review and recovery remain the source of truth.

Binding regeneration requires the Rust targets declared by the manifest. WASI generation also
requires the pinned WASI sysroot described by the build error. If an upstream change invalidates a
WASM provider identity, activate the documented Emscripten version before running write mode.

Do not hand-edit generated bindings or provider identities. Do not select bindgen output by
modification time; `upstream-sync` owns a fresh target directory and selects the exact output for
each declared target and precision. Native ABI compatibility is checked by the compiler-backed
`boxdd-abi-probe` qualification described in `docs/development/ci.md`.
