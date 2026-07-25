use std::fs;

use crate::{
    Error, Result,
    commands::{
        UpdateMode, parse_update_mode,
        upstream_sync::{ArtifactKind, UpstreamManifest},
    },
    config::{read_toml, write_atomic},
    paths::WorkspacePaths,
    recording_wire::{RecordingWireContract, render_runtime_parser},
    source_overlay::effective_source_identity,
};

const OUTPUT_PATH: &str = "boxdd/src/generated/recording_wire.rs";

pub fn run(paths: &WorkspacePaths, args: &[String]) -> Result<()> {
    let mode = parse_update_mode("recording-wire-codegen", args)?;
    let manifest = UpstreamManifest::load(paths)?;
    let contract_path = manifest.artifact_path(paths.root(), ArtifactKind::RecordingWire)?;
    let contract_bytes =
        fs::read(&contract_path).map_err(|source| Error::io(&contract_path, source))?;
    let contract: RecordingWireContract = read_toml(&contract_path)?;
    let digest = blake3::hash(&contract_bytes).to_hex().to_string();
    let effective_source = effective_source_identity(&paths.root().join("boxdd-sys"))
        .map_err(|error| Error::message(format!("validate effective source identity: {error}")))?;
    let rendered = render_runtime_parser(
        &contract,
        &digest,
        &effective_source.effective_source_sha256,
    )?;
    let output_path = paths.root().join(OUTPUT_PATH);

    match mode {
        UpdateMode::Check => {
            let actual = fs::read_to_string(&output_path)
                .map_err(|source| Error::io(&output_path, source))?;
            if actual != rendered {
                return Err(Error::message(format!(
                    "{} is stale; run cargo run -p xtask -- recording-wire-codegen --write",
                    output_path.display()
                )));
            }
            println!("runtime recording wire parser is current");
        }
        UpdateMode::Write => {
            write_atomic(&output_path, &rendered)?;
            println!("wrote {}", output_path.display());
        }
    }
    Ok(())
}
