// Cross-package source links are restricted to contracts that must execute identically in the
// published `boxdd-sys` build script and repository tooling. General I/O, process, path, and
// temporary-file helpers belong to xtask itself.
#[path = "../../boxdd-sys/src/adapter_contract.rs"]
pub(crate) mod adapter_contract;
#[allow(dead_code)]
#[path = "../../boxdd-sys/src/bindgen_contract.rs"]
pub(crate) mod bindgen_contract;
// The consumer parses this shared protocol; rendering is exercised by its contract tests.
#[allow(dead_code)]
#[path = "../../boxdd-sys/src/build_identity.rs"]
pub(crate) mod build_identity;
pub(crate) mod build_support;
pub mod config;
pub(crate) mod emscripten_sdk;
pub mod error;
pub(crate) mod isolated_git;
pub mod paths;
#[allow(dead_code)]
#[path = "../../boxdd-sys/src/prebuilt_provenance.rs"]
pub(crate) mod prebuilt_provenance;
#[path = "../../boxdd-sys/src/provenance_policy.rs"]
pub(crate) mod provenance_policy;
#[allow(dead_code)]
#[path = "../../boxdd-sys/src/provider_archive.rs"]
pub(crate) mod provider_archive;
#[allow(dead_code)]
#[path = "../../boxdd-sys/src/provider_catalog.rs"]
pub(crate) mod provider_catalog;
#[allow(dead_code)]
#[path = "../../boxdd-sys/src/provider_manifest.rs"]
pub(crate) mod provider_manifest;
pub mod recording_ops;
pub mod recording_wire;
#[path = "../../boxdd-sys/src/source_overlay.rs"]
pub mod source_overlay;
pub(crate) mod subprocess_policy;
pub(crate) mod wasm_identity;
#[path = "../../boxdd-sys/src/wasm_provider_contract.rs"]
pub(crate) mod wasm_provider_contract;
mod wasm_provider_gate;
#[path = "../../boxdd-sys/src/wasm_provider_memory.rs"]
pub(crate) mod wasm_provider_memory;
pub(crate) mod wasm_release_provenance;

pub mod commands;

pub use error::{Error, Result};

use paths::WorkspacePaths;

pub fn run(args: impl IntoIterator<Item = String>) -> Result<()> {
    let paths = WorkspacePaths::discover()?;
    run_in(&paths, args)
}

pub fn run_in(paths: &WorkspacePaths, args: impl IntoIterator<Item = String>) -> Result<()> {
    let args: Vec<String> = args.into_iter().collect();
    match args.as_slice() {
        [] => {
            print_help();
            Ok(())
        }
        [arg] if arg == "help" || arg == "--help" || arg == "-h" => {
            print_help();
            Ok(())
        }
        [command, rest @ ..] if command == "api-inventory" => {
            commands::api_inventory::run(paths, rest)
        }
        [command, rest @ ..] if command == "recording-wire-codegen" => {
            commands::recording_codegen::run(paths, rest)
        }
        [command, rest @ ..] if command == "upstream-sync" => {
            commands::upstream_sync::run(paths, rest)
        }
        [command] if command == "provider-smoke-app" => {
            commands::provider::provider_smoke_app(paths.root())
        }
        [command] if command == "provider-smoke" => {
            commands::provider::provider_smoke(paths.root())
        }
        [command, rest @ ..] if command == "wasm-provider-contract" => {
            commands::provider::wasm_provider_contract(paths.root(), rest)
        }
        [command, rest @ ..] if command == "build-wasm-provider-package" => {
            commands::wasm_release::build(paths.root(), rest)
        }
        [command, rest @ ..] if command == "qualify-wasm-provider" => {
            commands::wasm_release::qualify(paths.root(), rest)
        }
        [command, rest @ ..] if command == "verify-precision-contract" => {
            commands::precision_contract::run(paths.root(), rest)
        }
        [command, rest @ ..] if command == "verify-wasm" => {
            commands::verification::verify_wasm(paths.root(), rest)
        }
        [command, rest @ ..] if command == "verify-miri" => {
            commands::verification::verify_miri(paths.root(), rest)
        }
        [command, rest @ ..] if command == "verify-sanitizers" => {
            commands::verification::verify_sanitizers(paths.root(), rest)
        }
        [command, rest @ ..] if command == "verify-packages" => {
            commands::package_registry::run(paths.root(), rest)
        }
        [command, rest @ ..] if command == "release-contract" => {
            commands::release_contract::run(paths.root(), rest)
        }
        [command, rest @ ..] if command == "native-package" => {
            commands::native_package::run(paths.root(), rest)
        }
        [command, rest @ ..] if command == "qualify-native-provider" => {
            commands::native_provider::run(paths.root(), rest)
        }
        [command] if command == "build-pages-wasm" => {
            commands::pages::build_pages_wasm(paths.root())
        }
        [command] if command == "generate-pages" => commands::pages::generate_pages(paths.root()),
        [command] if command == "validate-pages" => commands::pages::validate_pages(paths.root()),
        [command, ..] => Err(Error::message(format!(
            "unknown xtask command `{command}`; run `cargo run -p xtask -- help`"
        ))),
    }
}

fn print_help() {
    println!(
        "\
boxdd xtask

Usage:
  cargo run -p xtask -- api-inventory --check
  cargo run -p xtask -- recording-wire-codegen --check
  cargo run -p xtask -- recording-wire-codegen --write
  cargo run -p xtask -- upstream-sync --check
  cargo run -p xtask -- upstream-sync --write
  cargo run -p xtask -- verify-precision-contract
  cargo run -p xtask -- verify-wasm --compile-only
  cargo run -p xtask -- verify-wasm --runtime
  cargo run -p xtask -- verify-miri
  cargo run -p xtask -- verify-sanitizers --address|--undefined|--thread
  cargo run -p xtask -- verify-packages
  cargo run -p xtask -- release-contract --check --artifacts <directory>
  cargo run -p xtask -- native-package build --sys-out <dir> --build-identity <file> --output <dir> --source-commit <sha> --release-tag <tag>
  cargo run -p xtask -- native-package attest-local-system <build-identity> <archive> <header-output> <bindings> <output>
  cargo run -p xtask -- native-package trust-local-system <input> <output>
  cargo run -p xtask -- qualify-native-provider --provider system ...
  cargo run -p xtask -- provider-smoke-app
  cargo run -p xtask -- provider-smoke
  cargo run -p xtask -- wasm-provider-contract --check
  cargo run -p xtask -- wasm-provider-contract --write
  cargo run -p xtask -- build-wasm-provider-package --precision <single|double> --output <directory>
  cargo run -p xtask -- qualify-wasm-provider --precision <single|double> --artifacts <directory> --cosign <path>
  cargo run -p xtask -- build-pages-wasm
  cargo run -p xtask -- generate-pages
  cargo run -p xtask -- validate-pages

Commands:
  api-inventory  Check human-reviewed C function dispositions against headers and bindings
  recording-wire-codegen  Validate or regenerate the allocation-free runtime parser table
  upstream-sync  Validate or regenerate artifacts for the current exact-SHA Box2D checkout
  verify-precision-contract  Verify matching precision routes and deterministic mismatch failures
  verify-wasm  Run compile-only targets or Node + Chromium provider runtime in both precisions
  verify-miri  Run pure-Rust unsafe helper and state-machine tests under the pinned nightly
  verify-sanitizers  Run mixed C/Rust ASan, C UBSan, or targeted mixed TSan suites
  verify-packages  Package, unpack, and consume all publishable crates through local patches
  release-contract  Validate tag, commit, upstream, changelog, artifacts, digests, and provenance
  native-package  Package one explicit native build identity or attest a local system artifact
  qualify-native-provider  Qualify a packaged crate against one exact native provider coordinate
  provider-smoke-app  Build the Rust wasm provider-smoke app and export list
  provider-smoke  Build the Rust app, Box2D provider, and run Node smoke
  wasm-provider-contract  Validate or refresh both checked WASM provider ABI identities
  build-wasm-provider-package  Build a deterministic official WASM provider package without executing it
  qualify-wasm-provider  Authenticate and execute an official WASM provider package in Node and Chromium
  build-pages-wasm  Build browser provider and Bevy testbed assets into docs/pages
  generate-pages  Generate the GitHub Pages Bevy example index from SCENE_REGISTRY
  validate-pages  Validate generated pages and local links in docs/pages/**/*.html

Environment:
  BOXDD_PAGES_WASM_PROFILE=debug|release|wasm-release  Select the Rust profile for Pages wasm; default: wasm-release
  BOXDD_PAGES_WASM_OPT=0                              Disable optional wasm-opt -Oz post-processing
"
    );
}
