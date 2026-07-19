# Upstream Synchronization Contract

`boxdd-sys/upstream.toml` is the only revision authority for the vendored Box2D source and every revision-coupled artifact. Rust source code must not duplicate the active or target commit as a constant.

The manifest records:

- the official repository and exact 40-character active, next, and recording revisions;
- named artifact paths and content digests with precision, target, and provider coordinates;
- executable binding routes with deterministic Rust target triples and explicit Cargo feature roots;
- the exact Git blob and BLAKE3 identities of all seven reviewed recording sources;
- the exact sorted relative paths of required C sources, private headers, inline files, and public headers;
- the target commit tree identity.

## Check Mode

```bash
cargo run -p xtask -- upstream-sync --check
```

Check mode is read-only and runs Git probes with optional index locking disabled. It rejects an uninitialized or dirty submodule, checkout/gitlink/manifest disagreement, missing commit objects, any source-path substitution, reviewed recording-source drift, an invalid operation registry read directly from the pinned `recording_revision:src/recording_ops.inl` Git object, artifact digest or revision mismatch, and any failure from the complete `api-coverage --check` contract gate. The operation registry is not copied into a second repository artifact. Check mode intentionally validates working-tree artifact content even when those artifacts are not committed, so contributors can run the gate before creating a commit.

## Write Mode

An upstream migration has two explicit phases:

1. Commit all generator inputs, then run `cargo run -p xtask -- upstream-sync --prepare-next`. The command pins the current root `HEAD`, rejects tracked, untracked, or ignored changes under the Cargo, `boxdd`, `boxdd-sys`, and `xtask` generator paths, builds the target headers and bindings in an isolated worktree at that exact root commit, mechanically reconciles the reviewed contract against the target API, validates the target candidate, and atomically records its path and digest without changing the active checkout or artifacts. Review the resulting manifest-declared `candidate_path`; `--check` continues to validate the active state during this phase.
2. Commit the candidate and manifest declaration, then run `cargo run -p xtask -- upstream-sync --write`. Write mode refuses dirty active or candidate artifacts. The command verifies both the active repository state and target candidate identities, generates and validates every target artifact in an isolated detached worktree, then switches the checkout, indexed gitlink, artifacts, and manifest as one rollback-capable transaction.

The command validates the complete target state, including `api-coverage --check`, before returning success. A failed generation, replacement, checkout, index update, or terminal validation restores every file and Git state mutated by the command. Candidate artifacts are preserved because they existed before the transaction.

Never select bindgen output by modification time or by scanning a shared target directory. Each bindings artifact owns an isolated target directory, runs Cargo with `--locked --target <manifest rust_target>`, passes the same target to Clang, and must produce exactly one `target/<rust_target>/.../bindings.rs` candidate. Install every manifest route target with `rustup target add <rust_target>` before generation.

`api-coverage --refresh-abi` refreshes the reviewed contract for the current active revision only. It cannot declare a next-revision candidate. During the schema-3 to schema-4 bootstrap, first commit the generator implementation, reviewed active inputs, pregenerated bindings, and the all-zero manifest. The managed transaction deliberately refuses to bootstrap from dirty generator paths. It accepts the all-zero digest manifest only while `artifact_digests_initialized = false` and only when the same operation refreshes exactly the reviewed active contract and every `api-coverage` output. Before calculating any artifact digest, it validates the active checkout and gitlink, target inventory, all seven reviewed recording-source identities, parses the operation registry from the pinned Git blob, regenerates every bindings artifact in an isolated worktree at the committed root revision and manifest Rust target, and requires byte-for-byte equality with the checked-in bindings. It then computes every remaining artifact digest, flips the initialization state, installs the manifest last, and passes the complete repository and API checks. Commit that initialized state as the second bootstrap commit. An initialized manifest can never contain zero digests; mixed initialized/zero digest manifests and ordinary generated-output writes fail closed.

The manifest Rust target is also passed to Clang. On macOS hosts, Linux-target bindgen uses the Xcode SDK only as the source of ISO C headers; the manifest Linux target continues to control ABI layout. The byte-for-byte checked-in bindings comparison remains the cross-host reproducibility gate.

## Concurrency and Durability Boundary

Mutating `upstream-sync` and `api-coverage` modes require cooperative exclusive access to the manifest, every manifest-declared active and candidate artifact, the Box2D submodule checkout, and the root repository index for the duration of generation and commit. They acquire the shared advisory lock before loading mutable repository state. The lock serializes commands that honor it; ordinary Git commands, editors, generators, and other tools do not. These commands therefore do not claim linearizability with arbitrary concurrent repository operations.

Managed files use before-state validation, no-clobber installation, and rollback compare-and-swap checks. The root Git index uses a complete-byte compare-and-swap: a candidate is built through a private `GIT_INDEX_FILE`, the standard `index.lock` is acquired, the captured index is compared under that lock, and the replacement is installed atomically. This preserves unrelated staged entries and rejects third-state index changes.

Git does not expose an expected-old atomic checkout operation. Submodule checkout therefore remains inside the cooperative advisory-lock boundary: preflight and rollback compare exact checkout state, Git rejects dirty conflicts, and every forward or restore checkout uses `--no-overwrite-ignore` so ignored user files cannot be replaced. Do not run ordinary Git checkout commands in the submodule while a mutating xtask command is active. A clean concurrent checkout in the narrow verify/write window is outside the linearizability guarantee. Once a managed path, index, or observable checkout reaches a third state, rollback preserves it, retains quarantined transaction or original content when necessary, and reports the conflict and retained location instead of overwriting it.

The rollback guarantee covers errors reported while the process remains alive. Individual managed-file replacements use same-directory atomic renames, but the multi-file transaction has no crash-recovery journal and does not promise recovery after `SIGKILL`, process termination during an operating-system call, power loss, or storage failure. A quarantine cleanup failure after terminal validation is reported explicitly as "installed successfully but quarantine cleanup failed"; the validated installed state is not rolled back at that point.
