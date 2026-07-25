use std::{
    env, fs,
    path::{Path, PathBuf},
};

use xtask::{
    abi_probe::{AbiProbePrecision, generate_workspace_probe_from_public_include},
    source_overlay::materialize_effective_box2d_sources,
};

const SOURCE_ROUTE_OVERRIDE_ENV: [&str; 4] = [
    "BOX2D_LIB_DIR",
    "BOXDD_SYS_SKIP_CC",
    "DOCS_RS",
    "CARGO_CFG_DOCSRS",
];

fn main() {
    if let Err(error) = run() {
        panic!("failed to build Box2D ABI probe: {error}");
    }
}

fn run() -> Result<(), String> {
    reject_source_route_overrides()?;
    let manifest_dir = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").map_err(|error| format!("CARGO_MANIFEST_DIR: {error}"))?,
    );
    let workspace_root = manifest_dir
        .parent()
        .and_then(|parent| parent.parent())
        .ok_or_else(|| "ABI probe manifest has no workspace root".to_owned())?;
    let out_dir = PathBuf::from(
        env::var("OUT_DIR").map_err(|error| format!("OUT_DIR is unavailable: {error}"))?,
    );
    let precision = if env::var_os("CARGO_FEATURE_DOUBLE_PRECISION").is_some() {
        AbiProbePrecision::Double
    } else {
        AbiProbePrecision::Single
    };
    let sys_manifest_dir = workspace_root.join("boxdd-sys");
    let materialized_sources = materialize_effective_box2d_sources(&sys_manifest_dir, &out_dir)
        .map_err(|error| format!("materialize effective Box2D source tree: {error}"))?;
    let generated = generate_workspace_probe_from_public_include(
        workspace_root,
        &materialized_sources.public_include,
        precision,
    )
    .map_err(|error| format!("generate {} probe: {error}", precision.as_str()))?;

    let c_source = out_dir.join("abi_probe.c");
    let mixed_source = out_dir.join("abi_probe_mixed.c");
    let rust_source = out_dir.join("abi_probe_cases.rs");
    fs::write(&c_source, generated.c_source)
        .map_err(|error| format!("write {}: {error}", c_source.display()))?;
    fs::write(&mixed_source, generated.mixed_precision_c_source)
        .map_err(|error| format!("write {}: {error}", mixed_source.display()))?;
    fs::write(&rust_source, generated.rust_source)
        .map_err(|error| format!("write {}: {error}", rust_source.display()))?;

    compile_c_source(
        &c_source,
        &materialized_sources.public_include,
        "boxdd_abi_probe",
    );
    compile_c_source(
        &mixed_source,
        &materialized_sources.public_include,
        "boxdd_abi_probe_mixed",
    );

    println!(
        "cargo:rerun-if-changed={}",
        sys_manifest_dir.join("third-party/box2d").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        sys_manifest_dir.join("effective-source.toml").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        workspace_root
            .join("boxdd-sys/src/bindings_pregenerated.rs")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        workspace_root
            .join("boxdd-sys/src/bindings_double.rs")
            .display()
    );
    Ok(())
}

fn reject_source_route_overrides() -> Result<(), String> {
    let mut inherited = Vec::new();
    for key in SOURCE_ROUTE_OVERRIDE_ENV {
        println!("cargo:rerun-if-env-changed={key}");
        if env::var_os(key).is_some() {
            inherited.push(key);
        }
    }
    if inherited.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "ABI qualification requires the vendored Box2D source route; unset inherited environment overrides: {}",
            inherited.join(", ")
        ))
    }
}

fn compile_c_source(source: &Path, include_dir: &Path, library: &str) {
    let mut build = cc::Build::new();
    build.file(source).include(include_dir);
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        build.flag_if_supported("/std:c17");
    } else {
        build.flag_if_supported("-std=c17");
    }
    build.compile(library);
}
