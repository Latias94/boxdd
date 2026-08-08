use std::{
    env,
    path::{Path, PathBuf},
};

// The probe must compile the same effective source overlay as `boxdd-sys`; the materialized
// include directory exported by its build script is intentionally temporary and cannot be reused.
#[allow(dead_code)]
#[path = "../../boxdd-sys/src/source_overlay.rs"]
mod source_overlay;

use source_overlay::materialize_effective_box2d_sources;

const SOURCE_ROUTE_OVERRIDE_ENV: &[&str] = &[
    "BOXDD_SYS_PROVIDER",
    "BOX2D_LIB_DIR",
    "BOXDD_SYS_SYSTEM_MANIFEST",
    "BOXDD_SYS_PREBUILT_MANIFEST",
    "BOXDD_SYS_PREBUILT_PROVENANCE",
    "BOXDD_SYS_PREBUILT_BUNDLE",
    "BOXDD_SYS_PREBUILT_TRUSTED_ROOT",
    "BOXDD_SYS_COSIGN",
    "BOXDD_SYS_LINK_KIND",
    "BOXDD_SYS_SKIP_CC",
    "BOXDD_SYS_FORCE_BINDGEN",
    "BOXDD_SYS_BINDGEN_TARGET",
    "BOXDD_SYS_WASI_SYSROOT",
    "BOXDD_SYS_WASM_PROVIDER_FINAL_LINK",
    "DOCS_RS",
    "CARGO_CFG_DOCSRS",
];

const CALLBACK_TYPES: [&str; 17] = [
    "b2AllocFcn",
    "b2AssertFcn",
    "b2CastResultFcn",
    "b2CustomFilterFcn",
    "b2EnqueueTaskCallback",
    "b2FinishTaskCallback",
    "b2FreeFcn",
    "b2FrictionCallback",
    "b2LogFcn",
    "b2OverlapResultFcn",
    "b2PlaneResultFcn",
    "b2PreSolveFcn",
    "b2RestitutionCallback",
    "b2TaskCallback",
    "b2TreeBoxCastCallbackFcn",
    "b2TreeQueryCallbackFcn",
    "b2TreeRayCastCallbackFcn",
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
    let double_precision = env::var_os("CARGO_FEATURE_DOUBLE_PRECISION").is_some();
    let sys_manifest_dir = workspace_root.join("boxdd-sys");
    let materialized_sources = materialize_effective_box2d_sources(&sys_manifest_dir, &out_dir)
        .map_err(|error| format!("materialize effective Box2D source tree: {error}"))?;

    let c_source = manifest_dir.join("src/abi_callbacks.c");
    let mixed_source = manifest_dir.join("src/abi_mixed_precision.c");

    compile_c_source(
        &c_source,
        &materialized_sources.public_include,
        "boxdd_abi_probe",
        double_precision.then_some("BOX2D_DOUBLE_PRECISION"),
    );
    compile_c_source(
        &mixed_source,
        &materialized_sources.public_include,
        "boxdd_abi_probe_mixed",
        double_precision.then_some("BOXDD_ABI_SELECTED_DOUBLE"),
    );
    generate_ctest(
        workspace_root,
        &materialized_sources.public_include,
        double_precision,
    )?;

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
    println!("cargo:rerun-if-changed={}", c_source.display());
    println!("cargo:rerun-if-changed={}", mixed_source.display());
    Ok(())
}

fn generate_ctest(
    workspace_root: &Path,
    public_include: &Path,
    double_precision: bool,
) -> Result<(), String> {
    let bindings = workspace_root
        .join("boxdd-sys/src")
        .join(if double_precision {
            "bindings_double.rs"
        } else {
            "bindings_pregenerated.rs"
        });
    let mut generator = ctest::TestGenerator::new();
    generator
        .header("box2d/box2d.h")
        .include(public_include)
        .edition(2024)
        .skip_const(|_| true)
        .skip_alias(|alias| alias.ident() == "b2TreeNodeFlags")
        .skip_struct(|struct_| matches!(struct_.ident(), "b2Recording" | "b2RecPlayer"))
        .skip_union(|union_| {
            matches!(
                union_.ident(),
                "b2TreeNode__bindgen_ty_1" | "b2TreeNode__bindgen_ty_2"
            )
        })
        .skip_struct_field(|struct_, field| {
            struct_.ident() == "b2TreeNode"
                && matches!(field.ident(), "__bindgen_anon_1" | "__bindgen_anon_2")
        })
        .rename_type(|type_| CALLBACK_TYPES.contains(&type_).then(|| format!("{type_}*")))
        .rename_struct_field(|_, field| {
            field
                .ident()
                .strip_suffix('_')
                .filter(|name| matches!(*name, "box" | "type"))
                .map(str::to_owned)
        })
        .rename_fn(move |function| {
            (double_precision && function.ident() == "b2CreateWorldDoublePrecision")
                .then(|| "b2CreateWorld".to_owned())
        });
    if double_precision {
        generator.define("BOX2D_DOUBLE_PRECISION", Some("1"));
    }
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        // Box2D intentionally uses anonymous unions in its public C API. MSVC accepts them but
        // reports C4201 under /Wall, while ctest promotes every remaining warning to an error.
        generator.flag("/wd4201");
    }
    ctest::generate_test(&mut generator, &bindings, "abi_ctest.rs")
        .map(|_| ())
        .map_err(|error| format!("generate compiler-backed ABI conformance tests: {error}"))
}

fn reject_source_route_overrides() -> Result<(), String> {
    let mut inherited = Vec::new();
    for &key in SOURCE_ROUTE_OVERRIDE_ENV {
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

fn compile_c_source(source: &Path, include_dir: &Path, library: &str, define: Option<&str>) {
    let mut build = cc::Build::new();
    build.file(source).include(include_dir);
    if let Some(define) = define {
        build.define(define, Some("1"));
    }
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        build.flag_if_supported("/std:c17");
    } else {
        build.flag_if_supported("-std=c17");
    }
    build.compile(library);
}
