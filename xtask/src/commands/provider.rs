use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    Error, Result,
    provider_manifest::{
        ADAPTER_SOURCE_PATHS, RECORDING_CONTRACT_BLAKE3, REQUIRED_ADAPTER_SYMBOLS,
        adapter_source_sha256,
    },
    source_overlay::materialize_effective_box2d_sources,
};

use super::support::{
    BuildProfile, WASM_TARGET, add_wasm_app_link_args, cargo_target_dir, copy_file, ensure_file,
    ensure_runnable_tool, replace_dir_under, run_command, runnable_tool,
};

pub(super) const PROVIDER_MODULE: &str = "box2d-sys-v1-single";
const PROVIDER_MODULE_DOUBLE: &str = "box2d-sys-v1-double";
const PROVIDER_ABI: &str = "box2d-sys-v1";
const EMSCRIPTEN_VERSION: &str = "6.0.3";
const EMSDK_REPOSITORY: &str = "https://github.com/emscripten-core/emsdk.git";
const EMSDK_REVISION: &str = "db04e88298d9916fc51fcd3743045ca3eb695127";
const WASM_BINDGEN_VERSION: &str = "0.2.126";
const SDK_CONFIG: &str = include_str!("../../../tools/emscripten-provider.toml");
const PROVIDER_SMOKE_PACKAGE: &str = "boxdd-provider-smoke";
const PROVIDER_SMOKE_WASM: &str = "boxdd_provider_smoke.wasm";
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
    args: Vec<PathBuf>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ProviderSdkIdentity {
    schema_version: u64,
    provider_abi: String,
    emscripten_version: String,
    emsdk_repository: String,
    emsdk_revision: String,
    wasm_bindgen_version: String,
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
    if let Some(missing) = REQUIRED_ADAPTER_SYMBOLS
        .iter()
        .find(|required| !imports.iter().any(|import| import == *required))
    {
        return Err(Error::Message(format!(
            "{} does not import required provider adapter symbol `{missing}` from {provider_module}",
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

const here = dirname(fileURLToPath(import.meta.url));
const memory = new WebAssembly.Memory({{ initial: 2048, maximum: 8192 }});
let memoryBuffer = memory.buffer;
function refreshMemoryViews() {{
  // WebAssembly.Memory.buffer is replaced after growth; every typed view must be rebound.
  if (memoryBuffer !== memory.buffer) memoryBuffer = memory.buffer;
}}
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
const importObject = {{
  env: {{ memory }},
  '{provider_module}': {{}},
}};

for (const name of providerImports) {{
  const exported = provider[`_${{name}}`] || provider[name];
  if (typeof exported !== 'function') {{
    throw new Error(`provider is missing export for ${{name}}`);
  }}
  importObject['{provider_module}'][name] = exported;
}}

const appBytes = fs.readFileSync(join(here, '{app_name}'));
const {{ instance }} = await WebAssembly.instantiate(appBytes, importObject);
if (typeof instance.exports.boxdd_provider_smoke !== 'function') {{
  throw new Error('boxdd_provider_smoke export is missing from Rust wasm');
}}

const beforeGrowth = memory.buffer;
memory.grow(1);
refreshMemoryViews();
if (beforeGrowth === memory.buffer) {{
  throw new Error('shared WebAssembly.Memory did not grow');
}}
const postGrowthMetric = instance.exports.boxdd_provider_ray_hit_millimeters();
if (postGrowthMetric < 0) {{
  throw new Error(`provider failed after memory growth with code ${{postGrowthMetric}}`);
}}

const code = instance.exports.boxdd_provider_smoke();
refreshMemoryViews();
if (code !== 0) {{
  throw new Error(`boxdd provider smoke failed with code ${{code}}`);
}}

const metricExports = {{
  dropMillimeters: 'boxdd_provider_drop_millimeters',
  rayHitMillimeters: 'boxdd_provider_ray_hit_millimeters',
  shapeCastPermyriad: 'boxdd_provider_shape_cast_permyriad',
  jointErrorMillimeters: 'boxdd_provider_joint_error_millimeters',
}};
const metrics = {{}};
for (const [label, exportName] of Object.entries(metricExports)) {{
  const exported = instance.exports[exportName];
  if (typeof exported !== 'function') {{
    throw new Error(`${{exportName}} export is missing from Rust wasm`);
  }}
  const value = exported();
  refreshMemoryViews();
  if (value < 0) {{
    throw new Error(`${{exportName}} failed with code ${{value}}`);
  }}
  metrics[label] = value;
}}

const runtimeInit = instance.exports.boxdd_runtime_init();
if (runtimeInit !== 0) {{
  throw new Error(`boxdd runtime init failed with code ${{runtimeInit}}`);
}}
for (let i = 0; i < 30; i += 1) {{
  const frame = instance.exports.boxdd_runtime_step();
  refreshMemoryViews();
  if (frame < 0) throw new Error(`boxdd runtime step failed with code ${{frame}}`);
}}
const runtimeBodies = instance.exports.boxdd_runtime_body_count();
if (runtimeBodies <= 0) {{
  throw new Error(`boxdd runtime body count failed with code ${{runtimeBodies}}`);
}}
const runtimeState = [];
for (let index = 0; index < runtimeBodies; index += 1) {{
  const body = {{
    shape: instance.exports.boxdd_runtime_body_shape(index),
    xMillimeters: instance.exports.boxdd_runtime_body_x_millimeters(index),
    yMillimeters: instance.exports.boxdd_runtime_body_y_millimeters(index),
    angleMilliradians: instance.exports.boxdd_runtime_body_angle_milliradians(index),
    halfWidthMillimeters: instance.exports.boxdd_runtime_body_half_width_millimeters(index),
    halfHeightMillimeters: instance.exports.boxdd_runtime_body_half_height_millimeters(index),
    radiusMillimeters: instance.exports.boxdd_runtime_body_radius_millimeters(index),
  }};
  if (body.shape === 1) {{
    if (
      body.halfWidthMillimeters <= 0 ||
      body.halfHeightMillimeters <= 0 ||
      body.radiusMillimeters !== 0
    ) {{
      throw new Error(`invalid box geometry at runtime body ${{index}}: ${{JSON.stringify(body)}}`);
    }}
  }} else if (body.shape === 2) {{
    if (
      body.halfWidthMillimeters !== 0 ||
      body.halfHeightMillimeters !== 0 ||
      body.radiusMillimeters <= 0
    ) {{
      throw new Error(`invalid circle geometry at runtime body ${{index}}: ${{JSON.stringify(body)}}`);
    }}
  }} else {{
    throw new Error(`unknown runtime shape ${{body.shape}} at body ${{index}}`);
  }}
  runtimeState.push(body);
}}

console.log(
  `boxdd provider smoke passed: drop_mm=${{metrics.dropMillimeters}}, ` +
    `ray_hit_mm=${{metrics.rayHitMillimeters}}, ` +
    `shape_cast_permyriad=${{metrics.shapeCastPermyriad}}, ` +
    `joint_error_mm=${{metrics.jointErrorMillimeters}}, ` +
    `runtime_bodies=${{runtimeBodies}}, ` +
    `runtime_state=${{JSON.stringify(runtimeState)}}`
);
"#
    );
    let package_json = out_dir.join("package.json");
    fs::write(&package_json, r#"{"type":"module"}"#)
        .map_err(|source| Error::io(&package_json, source))?;
    let path = out_dir.join("run-provider-smoke.mjs");
    fs::write(&path, runner).map_err(|source| Error::io(&path, source))
}

fn find_emcc() -> Result<EmccInvocation> {
    if let Ok(root) = env::var("EMSDK") {
        let emsdk = PathBuf::from(root);
        let emscripten = emsdk.join("upstream").join("emscripten");
        for name in ["emcc", "emcc.exe", "emcc.bat"] {
            let candidate = emscripten.join(name);
            if candidate.exists() {
                let invocation = EmccInvocation {
                    program: candidate,
                    args: Vec::new(),
                };
                verify_emcc(&invocation, Some(&emsdk))?;
                return Ok(invocation);
            }
        }
        let emcc_py = emscripten.join("emcc.py");
        if emcc_py.exists()
            && let Some(python) = find_emsdk_python(&emsdk)
        {
            let invocation = EmccInvocation {
                program: python,
                args: vec![emcc_py],
            };
            verify_emcc(&invocation, Some(&emsdk))?;
            return Ok(invocation);
        }
    }

    if let Some(path) = runnable_tool("emcc", "--version") {
        let invocation = EmccInvocation {
            program: path,
            args: Vec::new(),
        };
        verify_emcc(&invocation, None)?;
        return Ok(invocation);
    }

    Err(Error::Message(
        "failed to locate emcc; install emsdk, run emsdk_env, or set EMSDK to the emsdk root"
            .to_owned(),
    ))
}

fn verify_emcc(invocation: &EmccInvocation, emsdk: Option<&Path>) -> Result<()> {
    validate_sdk_config(SDK_CONFIG)?;
    let mut command = invocation.command();
    command.arg("--version");
    let output = command
        .output()
        .map_err(|source| Error::io("emcc --version", source))?;
    if !output.status.success() {
        return Err(Error::Message(format!(
            "Emscripten compiler failed --version with status {}",
            output.status
        )));
    }
    let version = String::from_utf8_lossy(&output.stdout);
    if !version_has_exact_token(&version, EMSCRIPTEN_VERSION) {
        return Err(Error::Message(format!(
            "provider runtime requires Emscripten {EMSCRIPTEN_VERSION}; found {}",
            version.lines().next().unwrap_or("unknown version")
        )));
    }

    let revision = emsdk
        .and_then(|root| {
            Command::new("git")
                .args(["-C", root.to_string_lossy().as_ref(), "rev-parse", "HEAD"])
                .output()
                .ok()
                .filter(|output| output.status.success())
                .and_then(|output| String::from_utf8(output.stdout).ok())
                .map(|value| value.trim().to_owned())
        })
        .or_else(|| env::var("BOXDD_EMSDK_REVISION").ok());
    if revision.as_deref() != Some(EMSDK_REVISION) {
        return Err(Error::Message(format!(
            "provider runtime requires immutable emsdk revision {EMSDK_REVISION}; set EMSDK to that checkout or BOXDD_EMSDK_REVISION to attest a PATH compiler"
        )));
    }
    Ok(())
}

pub(super) fn verify_wasm_bindgen_cli() -> Result<()> {
    validate_sdk_config(SDK_CONFIG)?;
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
    if !version_has_exact_token(&version, WASM_BINDGEN_VERSION) {
        return Err(Error::Message(format!(
            "provider runtime requires wasm-bindgen-cli {WASM_BINDGEN_VERSION}; found {}",
            version.lines().next().unwrap_or("unknown version")
        )));
    }
    Ok(())
}

fn validate_sdk_config(source: &str) -> Result<()> {
    let identity: ProviderSdkIdentity = toml::from_str(source).map_err(|error| {
        Error::Message(format!(
            "tools/emscripten-provider.toml is invalid: {error}"
        ))
    })?;
    if identity.schema_version != 1
        || identity.provider_abi != PROVIDER_ABI
        || identity.emscripten_version != EMSCRIPTEN_VERSION
        || identity.emsdk_repository != EMSDK_REPOSITORY
        || identity.emsdk_revision != EMSDK_REVISION
        || identity.wasm_bindgen_version != WASM_BINDGEN_VERSION
    {
        return Err(Error::Message(
            "tools/emscripten-provider.toml is inconsistent with the provider ABI constants"
                .to_owned(),
        ));
    }
    Ok(())
}

fn version_has_exact_token(output: &str, expected: &str) -> bool {
    output
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '.')
        .any(|token| token == expected)
}

fn find_emsdk_python(emsdk: &Path) -> Option<PathBuf> {
    let python_dir = emsdk.join("python");
    let mut candidates = Vec::new();
    if let Ok(entries) = fs::read_dir(&python_dir) {
        for entry in entries.flatten() {
            let path = entry.path().join("python.exe");
            if path.exists() {
                candidates.push(path);
            }
        }
    }
    candidates.sort();
    candidates.pop()
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
    fn sdk_contract_is_structured_exact_and_fail_closed() {
        validate_sdk_config(SDK_CONFIG).unwrap();
        assert!(validate_sdk_config(&SDK_CONFIG.replace("6.0.3", "6.0.30")).is_err());
        assert!(validate_sdk_config(&format!("{SDK_CONFIG}\nunknown = true\n")).is_err());
        assert!(version_has_exact_token("emcc 6.0.3", "6.0.3"));
        assert!(!version_has_exact_token("emcc 16.0.30", "6.0.3"));
    }
}
