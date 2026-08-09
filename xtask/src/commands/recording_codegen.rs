use std::{collections::BTreeMap, fs};

use crate::{
    Error, Result,
    commands::{
        UpdateMode, parse_update_mode,
        upstream_sync::{ArtifactKind, UpstreamManifest, validate_recording_input_identities},
    },
    config::{read_toml, write_atomic},
    paths::WorkspacePaths,
    provider_manifest::validate_recording_contract_blake3,
    recording_ops,
    recording_wire::{
        RecordingWireContract, render_runtime_parser, reviewed_sources_aggregate_blake3,
        validate_wire_contract,
    },
    source_overlay::effective_source_identity,
};

const OUTPUT_PATH: &str = "boxdd/src/generated/recording_wire.rs";

pub fn run(paths: &WorkspacePaths, args: &[String]) -> Result<()> {
    let mode = parse_update_mode("recording-wire-codegen", args)?;
    let manifest = UpstreamManifest::load(paths)?;
    validate_recording_input_identities(paths, &manifest)?;
    let contract_path = manifest.artifact_path(paths.root(), ArtifactKind::RecordingWire)?;
    let contract_bytes =
        fs::read(&contract_path).map_err(|source| Error::io(&contract_path, source))?;
    let contract: RecordingWireContract = read_toml(&contract_path)?;
    let recording_ops_path = paths.box2d().join(recording_ops::RECORDING_OPS_PATH);
    let recording_ops_source = fs::read_to_string(&recording_ops_path)
        .map_err(|source| Error::io(&recording_ops_path, source))?;
    let operations = recording_ops::parse(&recording_ops_source)?;
    let sys_root = paths.root().join("boxdd-sys");
    let effective_source = effective_source_identity(&sys_root).map_err(|error| {
        Error::message(format!("validate effective recording sources: {error}"))
    })?;
    let reviewed_sources = manifest
        .recording_inputs
        .iter()
        .map(|source| (source.path.clone(), source.git_blob.clone()))
        .collect::<BTreeMap<_, _>>();
    let reviewed_sources_aggregate = reviewed_sources_aggregate_blake3(&reviewed_sources)?;
    validate_wire_contract(
        &contract,
        &operations,
        &manifest.recording_revision,
        &reviewed_sources,
        &reviewed_sources_aggregate,
    )?;
    let digest = blake3::hash(&contract_bytes).to_hex().to_string();
    validate_recording_contract_blake3(&digest).map_err(|error| {
        Error::message(format!(
            "recording wire fixture is not the canonical provider contract: {error}"
        ))
    })?;
    let rendered = render_runtime_parser(
        &contract,
        &digest,
        &effective_source.effective_source_sha256,
    )?;
    let revalidated_effective_source = effective_source_identity(&sys_root).map_err(|error| {
        Error::message(format!("revalidate effective recording sources: {error}"))
    })?;
    if revalidated_effective_source != effective_source {
        return Err(Error::message(
            "effective recording sources changed during recording wire generation",
        ));
    }
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
