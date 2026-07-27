pub mod abi_contract;
pub mod abi_probe;
pub mod c_api;
pub mod config;
pub(crate) mod emscripten_sdk;
pub mod error;
pub mod paths;
#[path = "../../boxdd-sys/src/provenance_policy.rs"]
pub(crate) mod provenance_policy;
#[allow(dead_code)]
#[path = "../../boxdd-sys/src/provider_archive.rs"]
pub(crate) mod provider_archive;
#[allow(dead_code)]
#[path = "../../boxdd-sys/src/provider_manifest.rs"]
pub(crate) mod provider_manifest;
pub(crate) mod qualified_git;
pub mod recording_ops;
pub mod recording_wire;
pub mod rust_index;
#[path = "../../boxdd-sys/src/source_overlay.rs"]
pub mod source_overlay;
pub mod sys_abi_index;
pub mod toolchains;
pub(crate) mod wasm_identity;
#[path = "../../boxdd-sys/src/wasm_provider_contract.rs"]
pub(crate) mod wasm_provider_contract;

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
        [command, rest @ ..] if command == "api-coverage" => {
            commands::api_coverage::run(paths, rest)
        }
        [command, rest @ ..] if command == "recording-wire-codegen" => {
            commands::recording_codegen::run(paths, rest)
        }
        [command, rest @ ..] if command == "upstream-sync" => {
            commands::upstream_sync::run(paths, rest)
        }
        [command, rest @ ..] if command == "sample-parity" => {
            commands::sample_parity::run(paths.root(), rest)
        }
        [command] if command == "verify-toolchains" => {
            toolchains::verify_configuration(paths.root())
                .map(|verification| println!("{verification}"))
                .map_err(|error| Error::message(error.to_string()))
        }
        [command, option, root] if command == "provision-emsdk" && option == "--root" => {
            emscripten_sdk::provision_emscripten_sdk(
                &paths.root().join("xtask"),
                std::path::Path::new(root),
                false,
            )
            .map_err(Error::message)
        }
        [command, option, root, github_actions]
            if command == "provision-emsdk"
                && option == "--root"
                && github_actions == "--github-actions" =>
        {
            emscripten_sdk::provision_emscripten_sdk(
                &paths.root().join("xtask"),
                std::path::Path::new(root),
                true,
            )
            .map_err(Error::message)
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
        [command, rest @ ..] if command == "verify-precision-contract" => {
            commands::precision_contract::run(paths.root(), rest)
        }
        [command, rest @ ..] if command == "verify-feature-matrix" => {
            commands::verification::verify_feature_matrix(paths.root(), rest)
        }
        [command, rest @ ..] if command == "verify-compile-fail" => {
            commands::verification::verify_compile_fail(paths.root(), rest)
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
        [command, rest @ ..] if command == "verify-semver" => {
            commands::verification::verify_semver(paths.root(), rest)
        }
        [command, rest @ ..] if command == "verify-packages" => {
            commands::package_registry::run(paths.root(), rest)
        }
        [command, rest @ ..] if command == "release-contract" => {
            commands::release_contract::run(paths.root(), rest)
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
  cargo run -p xtask -- sample-parity --check
  cargo run -p xtask -- sample-parity --write
  cargo run -p xtask -- api-coverage --check
  cargo run -p xtask -- api-coverage --write
  cargo run -p xtask -- api-coverage --refresh-abi
  cargo run -p xtask -- recording-wire-codegen --check
  cargo run -p xtask -- recording-wire-codegen --write
  cargo run -p xtask -- api-coverage --audit-evidence
  cargo run -p xtask -- api-coverage --audit-canonical-paths
  cargo run -p xtask -- api-coverage --audit-reviewed-migration <40-hex-commit>
  cargo run -p xtask -- api-coverage --migrate-reviewed-contract <40-hex-commit>
  cargo run -p xtask -- upstream-sync --check
  cargo run -p xtask -- upstream-sync --prepare-next
  cargo run -p xtask -- upstream-sync --write
  cargo run -p xtask -- verify-toolchains
  cargo run -p xtask -- provision-emsdk --root <absolute-path> [--github-actions]
  cargo run -p xtask -- verify-precision-contract
  cargo run -p xtask -- verify-feature-matrix
  cargo run -p xtask -- verify-compile-fail
  cargo run -p xtask -- verify-wasm --compile-only
  cargo run -p xtask -- verify-wasm --runtime
  cargo run -p xtask -- verify-miri
  cargo run -p xtask -- verify-sanitizers --address|--undefined|--thread
  cargo run -p xtask -- verify-semver
  cargo run -p xtask -- verify-packages
  cargo run -p xtask -- release-contract --check
  cargo run -p xtask -- qualify-native-provider --provider system ...
  cargo run -p xtask -- provider-smoke-app
  cargo run -p xtask -- provider-smoke
  cargo run -p xtask -- wasm-provider-contract --check
  cargo run -p xtask -- wasm-provider-contract --write
  cargo run -p xtask -- build-pages-wasm
  cargo run -p xtask -- generate-pages
  cargo run -p xtask -- validate-pages

Commands:
  api-coverage  Validate, regenerate, audit, or perform an explicitly reviewed structured API-contract migration
  recording-wire-codegen  Validate or regenerate the allocation-free runtime parser table
  upstream-sync  Validate, prepare, or apply the exact-SHA Box2D migration transaction
  sample-parity  Validate or regenerate the upstream sample parity report
  verify-toolchains  Validate workspace versions and pinned compiler configuration
  provision-emsdk  Download, verify, extract, and qualify the pinned Emscripten SDK
  verify-precision-contract  Verify matching precision routes and deterministic mismatch failures
  verify-feature-matrix  Check every supported feature, provider, and precision coordinate
  verify-compile-fail  Run the ownership and lifetime compile-fail contract in both precisions
  verify-wasm  Run compile-only targets or Node + Chromium provider runtime in both precisions
  verify-miri  Run pure-Rust unsafe helper and state-machine tests under the pinned nightly
  verify-sanitizers  Run mixed C/Rust ASan, C UBSan, or targeted mixed TSan suites
  verify-semver  Check the intentional 0.5-to-0.6 public break with pinned SemVer tooling
  verify-packages  Package and consume all publishable crates through an isolated registry
  release-contract  Validate tag, commit, upstream, changelog, artifacts, digests, and provenance
  qualify-native-provider  Qualify a packaged crate against one exact native provider coordinate
  provider-smoke-app  Build the Rust wasm provider-smoke app and export list
  provider-smoke  Build the Rust app, Box2D provider, and run Node smoke
  wasm-provider-contract  Validate or atomically refresh both checked WASM provider ABI identities
  build-pages-wasm  Build browser provider and Bevy testbed assets into docs/pages
  generate-pages  Generate the GitHub Pages Bevy example index from SCENE_REGISTRY
  validate-pages  Validate generated pages and local links in docs/pages/**/*.html

Environment:
  BOXDD_PAGES_WASM_PROFILE=debug|release|wasm-release  Select the Rust profile for Pages wasm; default: wasm-release
  BOXDD_PAGES_WASM_OPT=0                              Disable optional wasm-opt -Oz post-processing
"
    );
}
