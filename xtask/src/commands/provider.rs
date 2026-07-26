use std::{
    collections::BTreeSet,
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    Error, Result,
    emscripten_sdk::{
        EMSCRIPTEN_VERSION, EmscriptenSdkInputs, SdkContract, qualify_emscripten_sdk,
        validate_wasm_bindgen_lock, version_has_exact_token,
    },
    provider_manifest::{
        ADAPTER_SOURCE_PATHS, RECORDING_CONTRACT_BLAKE3, REQUIRED_RUNTIME_IDENTITY_IMPORTS,
        adapter_source_sha256,
    },
    source_overlay::materialize_effective_box2d_sources,
};

use super::support::{
    BuildProfile, WASM_TARGET, add_wasm_app_link_args, cargo_target_dir, copy_file, ensure_file,
    ensure_runnable_tool, replace_dir_under, run_command,
};

pub(super) const PROVIDER_MODULE: &str = "box2d-sys-v1-single";
const PROVIDER_MODULE_DOUBLE: &str = "box2d-sys-v1-double";
const SDK_CONFIG: &str = include_str!("../../../boxdd-sys/emscripten-sdk.toml");
const CARGO_LOCK: &str = include_str!("../../../Cargo.lock");
const BOXDD_SYS_MANIFEST_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../boxdd-sys");
const PROVIDER_SMOKE_PACKAGE: &str = "boxdd-provider-smoke";
const PROVIDER_SMOKE_WASM: &str = "boxdd_provider_smoke.wasm";
const PROVIDER_RUNTIME_CONTRACT_FILE: &str = "provider-runtime-contract.mjs";
const PROVIDER_RUNTIME_CONTRACT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../examples-wasm/provider-smoke/provider-runtime-contract.mjs"
));
const PROVIDER_SMOKE_EXPORTS: &[&str] = &[
    "boxdd_provider_smoke",
    "boxdd_provider_drop_millimeters",
    "boxdd_provider_ray_hit_millimeters",
    "boxdd_provider_shape_cast_permyriad",
    "boxdd_provider_joint_error_millimeters",
];
const RUNTIME_EXPORTS: &[&str] = &[
    "boxdd_runtime_init",
    "boxdd_runtime_step",
    "boxdd_runtime_body_count",
    "boxdd_runtime_body_shape",
    "boxdd_runtime_body_x_millimeters",
    "boxdd_runtime_body_y_millimeters",
    "boxdd_runtime_body_angle_milliradians",
    "boxdd_runtime_body_half_width_millimeters",
    "boxdd_runtime_body_half_height_millimeters",
    "boxdd_runtime_body_radius_millimeters",
];

struct EmccInvocation {
    program: PathBuf,
    args: Vec<OsString>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(super) enum ProviderPrecision {
    Single,
    Double,
}

impl ProviderPrecision {
    pub(super) fn from_env() -> Result<Self> {
        match env::var("BOXDD_WASM_PRECISION")
            .unwrap_or_else(|_| "single".to_owned())
            .as_str()
        {
            "single" => Ok(Self::Single),
            "double" => Ok(Self::Double),
            value => Err(Error::Message(format!(
                "invalid BOXDD_WASM_PRECISION `{value}`; expected single or double"
            ))),
        }
    }

    pub(super) const fn module(self) -> &'static str {
        match self {
            Self::Single => PROVIDER_MODULE,
            Self::Double => PROVIDER_MODULE_DOUBLE,
        }
    }

    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Double => "double",
        }
    }

    pub(super) const fn cargo_feature(self) -> Option<&'static str> {
        match self {
            Self::Single => None,
            Self::Double => Some("double-precision"),
        }
    }

    pub(super) const fn c_define(self) -> Option<&'static str> {
        match self {
            Self::Single => None,
            Self::Double => Some("-DBOX2D_DOUBLE_PRECISION=1"),
        }
    }
}

impl EmccInvocation {
    fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command.args(&self.args);
        command
    }
}

pub(crate) fn provider_smoke_app(root: &Path) -> Result<()> {
    let target_dir = cargo_target_dir(root)?;
    let precision = ProviderPrecision::from_env()?;
    let app = build_provider_smoke_app(root, &target_dir, precision)?;
    let imports = collect_provider_imports(&app, precision.module())?;
    write_exports_json(&provider_smoke_dir(&target_dir), &imports)?;
    println!(
        "provider smoke app ready: {} ({} provider imports)",
        app.display(),
        imports.len()
    );
    Ok(())
}

pub(crate) fn provider_smoke(root: &Path) -> Result<()> {
    let precision = ProviderPrecision::from_env()?;
    provider_smoke_for_precision(root, precision)
}

pub(super) fn provider_smoke_for_precision(
    root: &Path,
    precision: ProviderPrecision,
) -> Result<()> {
    let target_dir = cargo_target_dir(root)?;
    verify_provider_compiler()?;
    verify_wasm_bindgen_cli()?;
    let app_wasm = build_provider_smoke_app(root, &target_dir, precision)?;
    let imports = collect_provider_imports(&app_wasm, precision.module())?;
    let out_dir = provider_smoke_dir(&target_dir);
    let exports = write_exports_json(&out_dir, &imports)?;
    let provider = build_box2d_provider(root, &out_dir, &exports, precision)?;
    let app_copy = out_dir.join(PROVIDER_SMOKE_WASM);
    write_node_runner(&out_dir, &provider, &app_copy, &imports, precision.module())?;

    let runner = out_dir.join("run-provider-smoke.mjs");
    let mut command = Command::new("node");
    command.arg(runner);
    run_command(&mut command, "run provider shared-memory smoke")
}

pub(super) fn verify_provider_compiler() -> Result<()> {
    find_emcc().map(|_| ())
}

pub(super) fn provider_smoke_dir(target_dir: &Path) -> PathBuf {
    target_dir.join("boxdd-provider-smoke")
}

fn build_provider_smoke_app(
    root: &Path,
    target_dir: &Path,
    precision: ProviderPrecision,
) -> Result<PathBuf> {
    let profile = BuildProfile::for_provider_smoke()?;
    let mut command = Command::new("cargo");
    command
        .arg("rustc")
        .arg("-p")
        .arg(PROVIDER_SMOKE_PACKAGE)
        .arg("--lib")
        .arg("--target")
        .arg(WASM_TARGET)
        .args(profile.cargo_args())
        .current_dir(root)
        .env("BOXDD_SYS_PROVIDER", "wasm-provider");
    if let Some(feature) = precision.cargo_feature() {
        command.arg("--features").arg(feature);
    }
    add_wasm_app_link_args(&mut command, &[PROVIDER_SMOKE_EXPORTS, RUNTIME_EXPORTS]);
    run_command(&mut command, "build provider-smoke Rust wasm")?;

    let wasm = target_dir
        .join(WASM_TARGET)
        .join(profile.target_dir())
        .join(PROVIDER_SMOKE_WASM);
    ensure_file(&wasm, "provider-smoke wasm artifact")?;

    let out_dir = provider_smoke_dir(target_dir);
    replace_dir_under(&out_dir, target_dir)?;
    copy_file(&wasm, &out_dir.join(PROVIDER_SMOKE_WASM))?;
    Ok(wasm)
}

pub(super) fn collect_provider_imports(wasm: &Path, provider_module: &str) -> Result<Vec<String>> {
    ensure_runnable_tool(
        "node",
        "--version",
        "Node.js is required for provider smoke",
    )?;
    let script = r#"
const fs = require('node:fs');
const wasmPath = process.argv[1];
const providerModule = process.argv[2];
const module = new WebAssembly.Module(fs.readFileSync(wasmPath));
const names = WebAssembly.Module.imports(module)
  .filter((i) => i.kind === 'function' && i.module === providerModule)
  .map((i) => i.name)
  .sort();
for (const name of names) console.log(name);
"#;
    let output = Command::new("node")
        .arg("-e")
        .arg(script)
        .arg(wasm)
        .arg(provider_module)
        .output()
        .map_err(|source| Error::io("node", source))?;
    if !output.status.success() {
        return Err(Error::Message(format!(
            "failed to inspect wasm imports with node: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let imports = String::from_utf8(output.stdout)
        .map_err(|err| Error::Message(format!("node printed invalid UTF-8: {err}")))?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if imports.is_empty() {
        return Err(Error::Message(format!(
            "{} does not import any functions from {provider_module}",
            wasm.display()
        )));
    }
    if let Some(missing) = REQUIRED_RUNTIME_IDENTITY_IMPORTS
        .iter()
        .find(|required| !imports.iter().any(|import| import == *required))
    {
        return Err(Error::Message(format!(
            "{} does not import required provider runtime identity function `{missing}` from {provider_module}",
            wasm.display()
        )));
    }
    Ok(imports)
}

pub(super) fn write_exports_json(out_dir: &Path, imports: &[String]) -> Result<PathBuf> {
    fs::create_dir_all(out_dir).map_err(|source| Error::io(out_dir, source))?;
    let mut exported = imports
        .iter()
        .map(|name| format!("\"_{name}\""))
        .collect::<Vec<_>>();
    exported.sort();
    let path = out_dir.join("box2d-provider-exports.json");
    fs::write(&path, format!("[{}]", exported.join(",")))
        .map_err(|source| Error::io(&path, source))?;
    Ok(path)
}

pub(super) fn build_box2d_provider(
    root: &Path,
    out_dir: &Path,
    exports_json: &Path,
    precision: ProviderPrecision,
) -> Result<PathBuf> {
    let emcc = find_emcc()?;
    let crate_root = root.join("boxdd-sys");
    let adapter_dir = crate_root.join("native");
    let provider = out_dir.join(format!("{}.js", precision.module()));

    let effective_sources =
        materialize_effective_box2d_sources(&crate_root, out_dir).map_err(|error| {
            Error::Message(format!(
                "failed to materialize provider effective source tree: {error}"
            ))
        })?;
    for source in &effective_sources.c_sources {
        ensure_file(source, "Box2D provider source inventory entry")?;
    }
    let adapter_sources = ADAPTER_SOURCE_PATHS
        .iter()
        .filter(|relative| relative.ends_with(".c"))
        .map(|relative| root.join("boxdd-sys").join(relative))
        .collect::<Vec<_>>();
    for source in &adapter_sources {
        ensure_file(source, "BoxDD provider adapter source")?;
    }
    let adapter_digest = adapter_source_sha256(&root.join("boxdd-sys"))
        .map_err(|error| Error::Message(format!("failed to identify provider adapter: {error}")))?;

    let mut command = emcc.command();
    command
        .arg("-std=c17")
        .arg("-O2")
        .arg("-s")
        .arg("MODULARIZE=1")
        .arg("-s")
        .arg("EXPORT_ES6=1")
        .arg("-s")
        .arg("ENVIRONMENT=node,web")
        .arg("-s")
        .arg("INCOMING_MODULE_JS_API=['wasmMemory','wasmBinary','locateFile','print','printErr']")
        .arg("-s")
        .arg("GLOBAL_BASE=67108864")
        .arg("-s")
        .arg("IMPORTED_MEMORY=1")
        .arg("-s")
        .arg("ALLOW_MEMORY_GROWTH=1")
        .arg("-s")
        .arg("INITIAL_MEMORY=134217728")
        .arg("-s")
        .arg("MAXIMUM_MEMORY=536870912")
        .arg("-s")
        .arg("FILESYSTEM=0")
        .arg("-s")
        .arg("NO_EXIT_RUNTIME=1")
        .arg("-s")
        .arg("MALLOC=emmalloc")
        .arg("-s")
        .arg("ASSERTIONS=1")
        .arg("-s")
        .arg("STACK_SIZE=1048576")
        .arg("-s")
        .arg("ERROR_ON_UNDEFINED_SYMBOLS=1")
        .arg("-s")
        .arg(format!(
            "EXPORTED_FUNCTIONS=@{}",
            exports_json.to_string_lossy().replace('\\', "/")
        ))
        .arg("-D_POSIX_C_SOURCE=199309L")
        .arg("-DBOX2D_DISABLE_SIMD")
        .arg(c_string_define(
            "BOXDD_UPSTREAM_SHA",
            &effective_sources.identity.upstream_sha,
        ))
        .arg(c_string_define(
            "BOXDD_EFFECTIVE_SOURCE_SHA256",
            &effective_sources.identity.effective_source_sha256,
        ))
        .arg(c_string_define("BOXDD_TARGET_ABI", WASM_TARGET))
        .arg(c_string_define(
            "BOXDD_ADAPTER_SOURCE_SHA256",
            &adapter_digest,
        ))
        .arg(c_string_define(
            "BOXDD_RECORDING_CONTRACT_BLAKE3",
            RECORDING_CONTRACT_BLAKE3,
        ))
        .arg("-I")
        .arg(&effective_sources.public_include)
        .arg("-I")
        .arg(&effective_sources.private_include)
        .arg("-I")
        .arg(&adapter_dir);
    if let Some(define) = precision.c_define() {
        command.arg(define);
    }
    for file in &effective_sources.c_sources {
        command.arg(file);
    }
    for file in adapter_sources {
        command.arg(file);
    }
    command.arg("-o").arg(&provider);
    run_command(&mut command, "build Box2D provider wasm")?;
    validate_box2d_provider_runtime(&provider)?;
    Ok(provider)
}

fn validate_box2d_provider_runtime(provider: &Path) -> Result<()> {
    let source = fs::read_to_string(provider).map_err(|source| Error::io(provider, source))?;
    if source.contains("toResizableBuffer") {
        return Err(Error::Message(format!(
            "{} was generated with resizable-buffer typed views; Emscripten {EMSCRIPTEN_VERSION} must expose refreshable wasmMemory.buffer views",
            provider.display()
        )));
    }
    if !source.contains("wasmMemory.buffer") {
        return Err(Error::Message(format!(
            "{} does not expose wasmMemory.buffer; refusing an unqualified provider runtime",
            provider.display()
        )));
    }
    Ok(())
}

fn c_string_define(name: &str, value: &str) -> String {
    format!("-D{name}=\"{value}\"")
}

fn write_node_runner(
    out_dir: &Path,
    provider: &Path,
    app_wasm: &Path,
    imports: &[String],
    provider_module: &str,
) -> Result<()> {
    let provider_name = provider
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| Error::Message("invalid provider file name".to_owned()))?;
    let app_name = app_wasm
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| Error::Message("invalid app wasm file name".to_owned()))?;
    let imports_array = imports
        .iter()
        .map(|name| format!("  \"{name}\""))
        .collect::<Vec<_>>()
        .join(",\n");
    let runner = format!(
        r#"import fs from 'node:fs';
import {{ dirname, join }} from 'node:path';
import {{ fileURLToPath }} from 'node:url';
import createProvider from './{provider_name}';
import {{
  inspectProviderContract,
  resolveProviderFunctions,
  runProviderPhysicsScenario,
}} from './provider-runtime-contract.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const memory = new WebAssembly.Memory({{ initial: 2048, maximum: 8192 }});
const provider = await createProvider({{
  wasmMemory: memory,
  locateFile: (path) => join(here, path),
  print: (text) => console.log(`[{provider_module}] ${{text}}`),
  printErr: (text) => console.warn(`[{provider_module}] ${{text}}`),
}});

if (provider.wasmMemory && provider.wasmMemory !== memory) {{
  throw new Error('provider did not use the shared WebAssembly.Memory');
}}

const providerImports = [
{imports_array}
];
const appBytes = fs.readFileSync(join(here, '{app_name}'));
const appModule = await WebAssembly.compile(appBytes);
const providerContract = inspectProviderContract(appModule, '{provider_module}');
if (JSON.stringify(providerContract.names) !== JSON.stringify(providerImports)) {{
  throw new Error('generated provider import inventory differs from the runtime module');
}}
const providerFunctions = resolveProviderFunctions(provider, providerContract.names);
const result = await runProviderPhysicsScenario({{
  appModule,
  memory,
  contract: providerContract,
  functions: providerFunctions,
}});

console.log(
  `boxdd provider smoke passed: drop_mm=${{result.metrics.dropMillimeters}}, ` +
    `ray_hit_mm=${{result.metrics.rayHitMillimeters}}, ` +
    `shape_cast_permyriad=${{result.metrics.shapeCastPermyriad}}, ` +
    `joint_error_mm=${{result.metrics.jointErrorMillimeters}}, ` +
    `stale_views_rejected=${{result.memoryProof.staleTypedArrayRejected && result.memoryProof.staleDataViewRejected}}, ` +
    `provider_glue_calls_after_growth=${{result.memoryProof.providerGlueCallsAfterGrowth}}, ` +
    `link_failures=${{JSON.stringify(result.linkFailures)}}, ` +
    `runtime_bodies=${{result.runtimeBodies}}, ` +
    `runtime_state=${{JSON.stringify(result.runtimeState)}}`
);
"#
    );
    let package_json = out_dir.join("package.json");
    fs::write(&package_json, r#"{"type":"module"}"#)
        .map_err(|source| Error::io(&package_json, source))?;
    let runtime_contract = out_dir.join(PROVIDER_RUNTIME_CONTRACT_FILE);
    fs::write(&runtime_contract, PROVIDER_RUNTIME_CONTRACT)
        .map_err(|source| Error::io(&runtime_contract, source))?;
    let path = out_dir.join("run-provider-smoke.mjs");
    fs::write(&path, runner).map_err(|source| Error::io(&path, source))
}

fn find_emcc() -> Result<EmccInvocation> {
    let root = env::var_os("EMSDK").filter(|value| !value.is_empty());
    let compiler_override = env::var_os("BOXDD_SYS_EMCC").filter(|value| !value.is_empty());
    let em_config_override = env::var_os("EM_CONFIG").filter(|value| !value.is_empty());
    let self_attested_revision =
        env::var_os("BOXDD_EMSDK_REVISION").filter(|value| !value.is_empty());
    let sdk = qualify_emscripten_sdk(
        Path::new(BOXDD_SYS_MANIFEST_DIR),
        EmscriptenSdkInputs {
            root: root.as_deref(),
            compiler_override: compiler_override.as_deref(),
            em_config_override: em_config_override.as_deref(),
            self_attested_revision: self_attested_revision.as_deref(),
        },
    )
    .map_err(Error::Message)?;

    Ok(EmccInvocation {
        program: sdk.compiler,
        args: vec![
            OsString::from("--em-config"),
            sdk.em_config.into_os_string(),
        ],
    })
}

pub(super) fn verify_wasm_bindgen_cli() -> Result<()> {
    let contract = provider_toolchain_contract()?;
    let output = Command::new("wasm-bindgen")
        .arg("--version")
        .output()
        .map_err(|source| Error::io("wasm-bindgen --version", source))?;
    if !output.status.success() {
        return Err(Error::Message(format!(
            "wasm-bindgen failed --version with status {}",
            output.status
        )));
    }
    let version = String::from_utf8_lossy(&output.stdout);
    if !version_has_exact_token(&version, &contract.wasm_bindgen_version) {
        return Err(Error::Message(format!(
            "provider runtime requires wasm-bindgen-cli {}; found {}",
            contract.wasm_bindgen_version,
            version.lines().next().unwrap_or("unknown version")
        )));
    }
    Ok(())
}

fn provider_toolchain_contract() -> Result<SdkContract> {
    let contract = SdkContract::parse(SDK_CONFIG).map_err(Error::Message)?;
    validate_wasm_bindgen_lock(&contract, CARGO_LOCK).map_err(Error::Message)?;
    Ok(contract)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_precision_owns_distinct_module_and_build_identity() {
        assert_eq!(ProviderPrecision::Single.module(), "box2d-sys-v1-single");
        assert_eq!(ProviderPrecision::Single.cargo_feature(), None);
        assert_eq!(ProviderPrecision::Double.module(), "box2d-sys-v1-double");
        assert_eq!(
            ProviderPrecision::Double.cargo_feature(),
            Some("double-precision")
        );
        assert_eq!(
            ProviderPrecision::Double.c_define(),
            Some("-DBOX2D_DOUBLE_PRECISION=1")
        );
    }

    #[test]
    fn provider_uses_the_canonical_sdk_and_lockfile_contract() {
        let contract = provider_toolchain_contract().unwrap();
        assert_eq!(contract.provider_abi, crate::emscripten_sdk::PROVIDER_ABI);
        assert_eq!(contract.emscripten_version, EMSCRIPTEN_VERSION);
        assert!(version_has_exact_token("emcc 6.0.3", "6.0.3"));
        assert!(!version_has_exact_token("emcc 16.0.30", "6.0.3"));
        assert!(!version_has_exact_token(
            "wasm-bindgen 0.2.126-custom",
            "0.2.126"
        ));

        let entry = format!(
            "name = \"wasm-bindgen\"\nversion = \"{}\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"",
            contract.wasm_bindgen_version
        );
        for drifted in [
            CARGO_LOCK.replacen(&entry, &entry.replace("0.2.126", "0.2.0"), 1),
            CARGO_LOCK.replacen(
                &entry,
                &entry.replace(
                    "registry+https://github.com/rust-lang/crates.io-index",
                    "git+https://example.invalid/wasm-bindgen?rev=unreviewed",
                ),
                1,
            ),
            CARGO_LOCK.replacen(
                &entry,
                "name = \"wasm-bindgen\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"",
                1,
            ),
            format!("{CARGO_LOCK}\n[[package]]\n{entry}\n"),
        ] {
            assert_ne!(drifted, CARGO_LOCK);
            assert!(validate_wasm_bindgen_lock(&contract, &drifted).is_err());
        }
    }

    #[test]
    fn node_runner_materializes_shared_runtime_contract() {
        let output = tempfile::tempdir().unwrap();
        write_node_runner(
            output.path(),
            Path::new("box2d-sys-v1-single.js"),
            Path::new(PROVIDER_SMOKE_WASM),
            &["boxddAdapter_AbiVersion".to_owned()],
            PROVIDER_MODULE,
        )
        .unwrap();

        let runner = fs::read_to_string(output.path().join("run-provider-smoke.mjs")).unwrap();
        assert!(runner.contains("runProviderPhysicsScenario"));
        assert!(!runner.contains("boxdd_runtime_step"));
        assert!(!runner.contains("RefreshableMemoryViews"));
        assert_eq!(
            fs::read_to_string(output.path().join(PROVIDER_RUNTIME_CONTRACT_FILE)).unwrap(),
            PROVIDER_RUNTIME_CONTRACT
        );
    }
}
