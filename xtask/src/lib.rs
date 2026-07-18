pub mod abi_contract;
pub mod c_api;
pub mod config;
pub mod error;
pub mod paths;
pub mod recording_ops;
pub mod recording_wire;
pub mod rust_index;
pub mod sys_abi_index;
pub mod toolchains;

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
        [command] if command == "provider-smoke-app" => {
            commands::provider::provider_smoke_app(paths.root())
        }
        [command] if command == "provider-smoke" => {
            commands::provider::provider_smoke(paths.root())
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
  cargo run -p xtask -- upstream-sync --check
  cargo run -p xtask -- upstream-sync --prepare-next
  cargo run -p xtask -- upstream-sync --write
  cargo run -p xtask -- verify-toolchains
  cargo run -p xtask -- provider-smoke-app
  cargo run -p xtask -- provider-smoke
  cargo run -p xtask -- build-pages-wasm
  cargo run -p xtask -- generate-pages
  cargo run -p xtask -- validate-pages

Commands:
  api-coverage  Validate or regenerate the structured API contract and report
  upstream-sync  Validate, prepare, or apply the exact-SHA Box2D migration transaction
  sample-parity  Validate or regenerate the upstream sample parity report
  verify-toolchains  Validate workspace versions and pinned compiler configuration
  provider-smoke-app  Build the Rust wasm provider-smoke app and export list
  provider-smoke  Build the Rust app, Box2D provider, and run Node smoke
  build-pages-wasm  Build browser provider and Bevy testbed assets into docs/pages
  generate-pages  Generate the GitHub Pages Bevy example index from SCENE_REGISTRY
  validate-pages  Validate generated pages and local links in docs/pages/**/*.html

Environment:
  BOXDD_PAGES_WASM_PROFILE=debug|release|wasm-release  Select the Rust profile for Pages wasm; default: wasm-release
  BOXDD_PAGES_WASM_OPT=0                              Disable optional wasm-opt -Oz post-processing
"
    );
}
