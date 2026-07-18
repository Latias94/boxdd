use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use crate::{Error, Result};

use super::support::{
    BuildProfile, WASM_TARGET, add_wasm_app_link_args, copy_file, ensure_file,
    ensure_runnable_tool, replace_dir_under, run_command, runnable_tool,
};

pub(super) const PROVIDER_MODULE: &str = "box2d-sys-v0";
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

impl EmccInvocation {
    fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command.args(&self.args);
        command
    }
}

pub(crate) fn provider_smoke_app(root: &Path) -> Result<()> {
    let app = build_provider_smoke_app(root)?;
    let imports = collect_provider_imports(&app)?;
    write_exports_json(&provider_smoke_dir(root), &imports)?;
    println!(
        "provider smoke app ready: {} ({} provider imports)",
        app.display(),
        imports.len()
    );
    Ok(())
}

pub(crate) fn provider_smoke(root: &Path) -> Result<()> {
    let app_wasm = build_provider_smoke_app(root)?;
    let imports = collect_provider_imports(&app_wasm)?;
    let out_dir = provider_smoke_dir(root);
    let exports = write_exports_json(&out_dir, &imports)?;
    let provider = build_box2d_provider(root, &out_dir, &exports)?;
    let app_copy = out_dir.join(PROVIDER_SMOKE_WASM);
    write_node_runner(&out_dir, &provider, &app_copy, &imports)?;

    let runner = out_dir.join("run-provider-smoke.mjs");
    let mut command = Command::new("node");
    command.arg(runner);
    run_command(&mut command, "run provider shared-memory smoke")
}

pub(super) fn provider_smoke_dir(root: &Path) -> PathBuf {
    root.join("target").join("boxdd-provider-smoke")
}

fn build_provider_smoke_app(root: &Path) -> Result<PathBuf> {
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
        .env("BOXDD_SYS_WASM_MODE", "provider");
    add_wasm_app_link_args(&mut command, &[PROVIDER_SMOKE_EXPORTS, RUNTIME_EXPORTS]);
    run_command(&mut command, "build provider-smoke Rust wasm")?;

    let wasm = root
        .join("target")
        .join(WASM_TARGET)
        .join(profile.target_dir())
        .join(PROVIDER_SMOKE_WASM);
    ensure_file(&wasm, "provider-smoke wasm artifact")?;

    let out_dir = provider_smoke_dir(root);
    replace_dir_under(&out_dir, &root.join("target"))?;
    copy_file(&wasm, &out_dir.join(PROVIDER_SMOKE_WASM))?;
    Ok(wasm)
}

pub(super) fn collect_provider_imports(wasm: &Path) -> Result<Vec<String>> {
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
        .arg(PROVIDER_MODULE)
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
            "{} does not import any functions from {PROVIDER_MODULE}",
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
) -> Result<PathBuf> {
    let emcc = find_emcc()?;
    let box2d_root = root.join("boxdd-sys").join("third-party").join("box2d");
    let include_dir = box2d_root.join("include");
    let src_dir = box2d_root.join("src");
    let provider = out_dir.join("box2d-sys-v0.js");

    let mut c_files = Vec::new();
    collect_c_files(&src_dir, &mut c_files)?;
    c_files.sort();

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
        .arg("-I")
        .arg(&include_dir)
        .arg("-I")
        .arg(&src_dir);
    for file in c_files {
        command.arg(file);
    }
    command.arg("-o").arg(&provider);
    run_command(&mut command, "build Box2D provider wasm")?;
    patch_box2d_provider_runtime(&provider)?;
    Ok(provider)
}

fn patch_box2d_provider_runtime(provider: &Path) -> Result<()> {
    let source = fs::read_to_string(provider).map_err(|source| Error::io(provider, source))?;
    let patched = source.replace(
        "function getMemoryBuffer(){try{var b=wasmMemory.toResizableBuffer();return b}catch{}return wasmMemory.buffer}",
        "function getMemoryBuffer(){return wasmMemory.buffer}",
    );
    if patched == source && source.contains("toResizableBuffer") {
        return Err(Error::Message(format!(
            "{} uses toResizableBuffer but xtask could not patch the provider memory view",
            provider.display()
        )));
    }
    if patched != source {
        fs::write(provider, patched).map_err(|source| Error::io(provider, source))?;
    }
    Ok(())
}

fn collect_c_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).map_err(|source| Error::io(dir, source))? {
        let path = entry.map_err(|source| Error::io(dir, source))?.path();
        if path.is_dir() {
            collect_c_files(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "c") {
            out.push(path);
        }
    }
    Ok(())
}

fn write_node_runner(
    out_dir: &Path,
    provider: &Path,
    app_wasm: &Path,
    imports: &[String],
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
const provider = await createProvider({{
  wasmMemory: memory,
  locateFile: (path) => join(here, path),
  print: (text) => console.log(`[box2d-sys-v0] ${{text}}`),
  printErr: (text) => console.warn(`[box2d-sys-v0] ${{text}}`),
}});

if (provider.wasmMemory && provider.wasmMemory !== memory) {{
  throw new Error('provider did not use the shared WebAssembly.Memory');
}}

const providerImports = [
{imports_array}
];
const importObject = {{
  env: {{ memory }},
  '{PROVIDER_MODULE}': {{}},
}};

for (const name of providerImports) {{
  const exported = provider[`_${{name}}`] || provider[name];
  if (typeof exported !== 'function') {{
    throw new Error(`provider is missing export for ${{name}}`);
  }}
  importObject['{PROVIDER_MODULE}'][name] = exported;
}}

const appBytes = fs.readFileSync(join(here, '{app_name}'));
const {{ instance }} = await WebAssembly.instantiate(appBytes, importObject);
if (typeof instance.exports.boxdd_provider_smoke !== 'function') {{
  throw new Error('boxdd_provider_smoke export is missing from Rust wasm');
}}

const code = instance.exports.boxdd_provider_smoke();
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
  if (frame < 0) throw new Error(`boxdd runtime step failed with code ${{frame}}`);
}}
const runtimeBodies = instance.exports.boxdd_runtime_body_count();
if (runtimeBodies <= 0) {{
  throw new Error(`boxdd runtime body count failed with code ${{runtimeBodies}}`);
}}

console.log(
  `boxdd provider smoke passed: drop_mm=${{metrics.dropMillimeters}}, ` +
    `ray_hit_mm=${{metrics.rayHitMillimeters}}, ` +
    `shape_cast_permyriad=${{metrics.shapeCastPermyriad}}, ` +
    `joint_error_mm=${{metrics.jointErrorMillimeters}}, ` +
    `runtime_bodies=${{runtimeBodies}}`
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
    if let Some(path) = runnable_tool("emcc", "--version") {
        return Ok(EmccInvocation {
            program: path,
            args: Vec::new(),
        });
    }

    if let Ok(root) = env::var("EMSDK") {
        let emsdk = PathBuf::from(root);
        let emscripten = emsdk.join("upstream").join("emscripten");
        for name in ["emcc", "emcc.exe", "emcc.bat"] {
            let candidate = emscripten.join(name);
            if candidate.exists() {
                return Ok(EmccInvocation {
                    program: candidate,
                    args: Vec::new(),
                });
            }
        }
        let emcc_py = emscripten.join("emcc.py");
        if emcc_py.exists()
            && let Some(python) = find_emsdk_python(&emsdk)
        {
            return Ok(EmccInvocation {
                program: python,
                args: vec![emcc_py],
            });
        }
    }

    Err(Error::Message(
        "failed to locate emcc; install emsdk, run emsdk_env, or set EMSDK to the emsdk root"
            .to_owned(),
    ))
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
