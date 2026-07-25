use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    fmt::Write as _,
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};

use crate::{Error, Result};

use super::{
    provider::{
        ProviderPrecision, build_box2d_provider, collect_provider_imports, provider_smoke_dir,
        verify_provider_compiler, verify_wasm_bindgen_cli, write_exports_json,
    },
    support::{
        BuildProfile, WASM_TARGET, add_wasm_app_link_args, cargo_target_dir, copy_file,
        ensure_file, replace_dir_under, run_command, runnable_path, runnable_tool,
    },
};

const PAGES_WASM_OPT_ENV: &str = "BOXDD_PAGES_WASM_OPT";
const PAGES_WASM_DIR: &str = "wasm/generated";
const BEVY_EXAMPLES_DIR: &str = "examples";
const BEVY_WEB_EXAMPLE: &str = "testbed_2d";
const BEVY_WEB_OUT_DIR: &str = "bevy-testbed/generated";
const BEVY_WEB_OUT_NAME: &str = "bevy_boxdd_testbed";
const BEVY_WEB_JS: &str = "bevy_boxdd_testbed.js";
const BEVY_WEB_WASM: &str = "bevy_boxdd_testbed_bg.wasm";
const BEVY_PROVIDER_SHIM: &str = "box2d-provider-shim.js";

struct RegistrySample {
    id: String,
    category: String,
    name: String,
    description: String,
    upstream: Vec<RegistryUpstreamSample>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct RegistryUpstreamSample {
    category: String,
    name: String,
    mode: String,
}

#[derive(Debug, Default)]
struct PageSampleBuilder {
    id: Option<String>,
    category: Option<String>,
    name: Option<String>,
    description: Option<String>,
    upstream: Vec<RegistryUpstreamSample>,
}

#[derive(Debug, Default)]
struct UpstreamSampleBuilder {
    category: Option<String>,
    name: Option<String>,
    mode: Option<String>,
}

struct BevyWebArtifacts {
    out_dir: PathBuf,
    imports: Vec<String>,
}

#[derive(Copy, Clone)]
enum ExampleIndexLocation {
    Root,
    ExamplesDirectory,
}

pub(crate) fn build_pages_wasm(root: &Path) -> Result<()> {
    let target_dir = cargo_target_dir(root)?;
    let precision = ProviderPrecision::from_env()?;
    validate_pages_precision(precision)?;
    verify_wasm_bindgen_cli()?;
    verify_provider_compiler()?;
    generate_pages(root)?;
    let bevy_artifacts = build_bevy_web_app(root, &target_dir, precision)?;
    let out_dir = provider_smoke_dir(&target_dir);
    let exports = write_exports_json(&out_dir, &bevy_artifacts.imports)?;
    let provider = build_box2d_provider(root, &out_dir, &exports, precision)?;
    let provider_wasm = provider.with_extension("wasm");
    ensure_file(&provider, "Box2D provider module")?;
    ensure_file(&provider_wasm, "Box2D provider wasm")?;
    optimize_wasm_if_available(&provider_wasm, "Box2D provider wasm")?;

    let generated = pages_wasm_generated_dir(root);
    replace_dir_under(&generated, &root.join("docs/pages"))?;
    copy_file(
        &provider,
        &generated.join(format!("{}.js", precision.module())),
    )?;
    copy_file(
        &provider_wasm,
        &generated.join(format!("{}.wasm", precision.module())),
    )?;
    copy_bevy_web_artifacts(root, &bevy_artifacts)?;

    println!(
        "pages wasm assets ready: {} and {} ({} Bevy imports)",
        generated.display(),
        pages_bevy_generated_dir(root).display(),
        bevy_artifacts.imports.len()
    );
    Ok(())
}

fn validate_pages_precision(precision: ProviderPrecision) -> Result<()> {
    if precision == ProviderPrecision::Single {
        Ok(())
    } else {
        Err(Error::Message(
            "GitHub Pages currently qualifies only BOXDD_WASM_PRECISION=single; use provider-smoke to qualify the double-precision runtime"
                .to_owned(),
        ))
    }
}

fn pages_wasm_generated_dir(root: &Path) -> PathBuf {
    root.join("docs").join("pages").join(PAGES_WASM_DIR)
}

fn pages_bevy_generated_dir(root: &Path) -> PathBuf {
    root.join("docs").join("pages").join(BEVY_WEB_OUT_DIR)
}

fn pages_bevy_testbed_dir(root: &Path) -> PathBuf {
    root.join("docs").join("pages").join("bevy-testbed")
}

fn build_bevy_web_app(
    root: &Path,
    target_dir: &Path,
    precision: ProviderPrecision,
) -> Result<BevyWebArtifacts> {
    let out_dir = target_dir.join("boxdd-bevy-testbed-web");
    replace_dir_under(&out_dir, target_dir)?;

    let profile = BuildProfile::for_pages()?;
    let mut command = Command::new("cargo");
    command
        .arg("rustc")
        .arg("-p")
        .arg("bevy_boxdd")
        .arg("--example")
        .arg(BEVY_WEB_EXAMPLE)
        .arg("--target")
        .arg(WASM_TARGET)
        .args(profile.cargo_args())
        .current_dir(root)
        .env("BOXDD_SYS_PROVIDER", "wasm-provider");
    if let Some(feature) = precision.cargo_feature() {
        command
            .arg("--features")
            .arg(format!("bevy_boxdd/{feature}"));
    }
    add_wasm_app_link_args(&mut command, &[]);
    run_command(
        &mut command,
        &format!("build Bevy testbed wasm ({})", profile.label()),
    )?;

    let wasm = target_dir
        .join(WASM_TARGET)
        .join(profile.target_dir())
        .join("examples")
        .join(format!("{BEVY_WEB_EXAMPLE}.wasm"));
    ensure_file(&wasm, "Bevy testbed wasm")?;

    let mut bindgen = Command::new("wasm-bindgen");
    bindgen
        .arg("--target")
        .arg("web")
        .arg("--out-dir")
        .arg(&out_dir)
        .arg("--out-name")
        .arg(BEVY_WEB_OUT_NAME)
        .arg(&wasm);
    run_command(&mut bindgen, "run wasm-bindgen for Bevy testbed")?;

    patch_bevy_bindgen_imports(&out_dir.join(BEVY_WEB_JS), precision.module())?;
    let bevy_wasm = out_dir.join(BEVY_WEB_WASM);
    optimize_wasm_if_available(&bevy_wasm, "Bevy testbed wasm")?;
    let imports = collect_provider_imports(&bevy_wasm, precision.module())?;
    write_browser_provider_shim(&out_dir, &imports)?;

    Ok(BevyWebArtifacts { out_dir, imports })
}

fn patch_bevy_bindgen_imports(js: &Path, provider_module: &str) -> Result<()> {
    let source = fs::read_to_string(js).map_err(|source| Error::io(js, source))?;
    let patched_imports = source.replace(
        &format!("from \"{provider_module}\""),
        &format!("from \"./{BEVY_PROVIDER_SHIM}\""),
    );
    if patched_imports == source {
        return Err(Error::Message(format!(
            "wasm-bindgen output does not import {provider_module}: {}",
            js.display()
        )));
    }
    let patched = patched_imports.replace(
        "    wasm = instance.exports;\n",
        "    wasm = instance.exports;\n    if (typeof import1.setBoxddAppExports === \"function\") {\n        import1.setBoxddAppExports(wasm);\n    }\n",
    );
    if patched == patched_imports {
        return Err(Error::Message(format!(
            "wasm-bindgen output does not assign instance exports: {}",
            js.display()
        )));
    }
    let decode_patched = patched.replace(
        "cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len))",
        "cachedTextDecoder.decode(getUint8ArrayMemory0().slice(ptr, ptr + len))",
    );
    if decode_patched == patched {
        return Err(Error::Message(format!(
            "wasm-bindgen output does not decode strings from wasm memory: {}",
            js.display()
        )));
    }
    fs::write(js, decode_patched).map_err(|source| Error::io(js, source))
}

fn write_browser_provider_shim(out_dir: &Path, imports: &[String]) -> Result<PathBuf> {
    let exports = imports
        .iter()
        .map(|name| {
            format!("export function {name}(...args) {{ return callProvider(\"{name}\", args); }}")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let shim = format!(
        r#"let provider;

export function setBox2dProvider(nextProvider) {{
  provider = nextProvider;
}}

export function setBoxddAppExports(exports) {{
  if (!provider) {{
    throw new Error("Box2D provider is not initialized");
  }}
  provider.boxddAppExports = exports;
}}

function resolveProviderExport(name) {{
  if (!provider) {{
    throw new Error("Box2D provider is not initialized");
  }}
  const exported = provider[`_${{name}}`] || provider[name];
  if (typeof exported !== "function") {{
    throw new Error(`Box2D provider is missing export ${{name}}`);
  }}
  return exported;
}}

function callProvider(name, args) {{
  return resolveProviderExport(name)(...args);
}}

{exports}
"#
    );
    let path = out_dir.join(BEVY_PROVIDER_SHIM);
    fs::write(&path, shim).map_err(|source| Error::io(&path, source))?;
    Ok(path)
}

fn copy_bevy_web_artifacts(root: &Path, artifacts: &BevyWebArtifacts) -> Result<()> {
    let generated = pages_bevy_generated_dir(root);
    replace_dir_under(&generated, &root.join("docs/pages"))?;

    for file in [BEVY_WEB_JS, BEVY_WEB_WASM, BEVY_PROVIDER_SHIM] {
        copy_file(&artifacts.out_dir.join(file), &generated.join(file))?;
    }

    Ok(())
}

fn optimize_wasm_if_available(wasm: &Path, label: &str) -> Result<()> {
    if !pages_wasm_opt_enabled() {
        println!("wasm-opt skipped for {label}: disabled by {PAGES_WASM_OPT_ENV}");
        return Ok(());
    }

    let Some(wasm_opt) = find_wasm_opt() else {
        println!("wasm-opt skipped for {label}: install Binaryen or expose EMSDK/upstream/bin");
        return Ok(());
    };

    let before = file_size(wasm)?;
    let tmp = wasm.with_extension("wasm-opt.tmp");
    let mut command = Command::new(wasm_opt);
    command
        .arg("-Oz")
        .arg("--enable-bulk-memory")
        .arg("--enable-bulk-memory-opt")
        .arg("--enable-nontrapping-float-to-int")
        .arg("--strip-debug")
        .arg("--strip-producers")
        .arg(wasm)
        .arg("-o")
        .arg(&tmp);
    run_command(&mut command, &format!("optimize {label} with wasm-opt"))?;

    fs::copy(&tmp, wasm).map_err(|source| Error::io(wasm, source))?;
    fs::remove_file(&tmp).map_err(|source| Error::io(&tmp, source))?;

    let after = file_size(wasm)?;
    let saved = before.saturating_sub(after);
    let pct = if before == 0 {
        0.0
    } else {
        saved as f64 * 100.0 / before as f64
    };
    println!(
        "{label} optimized: {} -> {} ({saved} bytes saved, {pct:.1}%)",
        format_bytes(before),
        format_bytes(after)
    );
    Ok(())
}

fn pages_wasm_opt_enabled() -> bool {
    !matches!(
        env::var(PAGES_WASM_OPT_ENV).ok().as_deref(),
        Some("0" | "false" | "False" | "FALSE" | "off" | "OFF" | "no" | "NO")
    )
}

fn file_size(path: &Path) -> Result<u64> {
    fs::metadata(path)
        .map(|metadata| metadata.len())
        .map_err(|source| Error::io(path, source))
}

fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    let bytes_f = bytes as f64;
    if bytes_f >= MIB {
        format!("{:.2} MiB", bytes_f / MIB)
    } else if bytes_f >= KIB {
        format!("{:.2} KiB", bytes_f / KIB)
    } else {
        format!("{bytes} B")
    }
}
fn find_wasm_opt() -> Option<PathBuf> {
    if let Some(path) = runnable_tool("wasm-opt", "--version") {
        return Some(path);
    }

    let mut candidates = Vec::new();
    if let Ok(emsdk) = env::var("EMSDK") {
        candidates.push(PathBuf::from(emsdk).join("upstream").join("bin"));
    }

    for dir in candidates {
        for name in ["wasm-opt", "wasm-opt.exe"] {
            let candidate = dir.join(name);
            if candidate.exists()
                && let Some(path) = runnable_path(&candidate, "--version")
            {
                return Some(path);
            }
        }
    }

    None
}

pub(crate) fn generate_pages(root: &Path) -> Result<()> {
    let samples = read_testbed_registry(root)?;
    let pages = expected_bevy_pages(root, &samples);
    let pages_dir = root.join("docs/pages");
    let examples_dir = pages_dir.join("examples");

    reset_generated_examples_dir(&pages_dir, &examples_dir)?;
    for (path, html) in pages {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| Error::io(parent, source))?;
        }
        fs::write(&path, html).map_err(|source| Error::io(&path, source))?;
    }
    write_bevy_testbed_loader(root)?;
    remove_file_if_exists(&pages_dir.join("wasm/index.html"))?;
    remove_file_if_exists(&pages_dir.join("wasm/loader.js"))?;

    println!(
        "generated pages: {} Bevy WASM examples under {}",
        samples.len(),
        pages_dir.display()
    );
    Ok(())
}

fn expected_bevy_pages(root: &Path, samples: &[RegistrySample]) -> BTreeMap<PathBuf, String> {
    let pages_dir = root.join("docs/pages");
    let mut pages = BTreeMap::new();
    pages.insert(
        pages_dir.join("index.html"),
        bevy_example_index_page(samples, ExampleIndexLocation::Root),
    );
    pages.insert(
        pages_dir.join(BEVY_EXAMPLES_DIR).join("index.html"),
        bevy_example_index_page(samples, ExampleIndexLocation::ExamplesDirectory),
    );
    pages.insert(
        pages_bevy_testbed_dir(root).join("index.html"),
        bevy_testbed_page(),
    );
    for sample in samples {
        pages.insert(
            pages_dir
                .join(BEVY_EXAMPLES_DIR)
                .join(&sample.id)
                .join("index.html"),
            bevy_example_page(sample),
        );
    }
    pages
}

fn remove_file_if_exists(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(Error::io(path, source)),
    }
}

fn write_bevy_testbed_loader(root: &Path) -> Result<()> {
    let dir = pages_bevy_testbed_dir(root);
    fs::create_dir_all(&dir).map_err(|source| Error::io(&dir, source))?;
    let path = dir.join("loader.js");
    fs::write(&path, bevy_testbed_loader_js()).map_err(|source| Error::io(&path, source))
}

fn read_testbed_registry(root: &Path) -> Result<Vec<RegistrySample>> {
    let scenes = root
        .join("bevy_boxdd")
        .join("examples")
        .join("testbed_2d")
        .join("scenes.rs");
    let source = fs::read_to_string(&scenes).map_err(|source| Error::io(&scenes, source))?;
    let mut samples = Vec::new();
    let mut current: Option<PageSampleBuilder> = None;
    let mut current_upstream: Option<UpstreamSampleBuilder> = None;
    let mut in_registry = false;

    for line in source.lines() {
        if line.contains("pub const SCENE_REGISTRY") {
            in_registry = true;
            continue;
        }
        if !in_registry {
            continue;
        }

        let trimmed = line.trim();
        if let Some(upstream) = current_upstream.as_mut() {
            read_upstream_fields(upstream, trimmed);
            if trimmed == "}," || trimmed.ends_with("},") || trimmed.ends_with("}],") {
                let upstream = current_upstream
                    .take()
                    .expect("upstream builder should be present");
                current
                    .as_mut()
                    .ok_or_else(|| {
                        Error::Message(format!(
                            "upstream sample outside registry entry in {}",
                            scenes.display()
                        ))
                    })?
                    .upstream
                    .push(upstream.build()?);
            }
            continue;
        }
        if trimmed == "];" {
            break;
        }
        if trimmed.starts_with("TestbedSceneMetadata {") {
            current = Some(PageSampleBuilder::default());
            continue;
        }
        if trimmed == "}," {
            let builder = current.take().ok_or_else(|| {
                Error::Message(format!(
                    "unexpected registry entry terminator in {}",
                    scenes.display()
                ))
            })?;
            samples.push(builder.build()?);
            continue;
        }

        let Some(builder) = current.as_mut() else {
            continue;
        };
        if trimmed.contains("UpstreamSampleRef {") {
            let mut upstream = UpstreamSampleBuilder::default();
            read_upstream_fields(&mut upstream, trimmed);
            if trimmed.ends_with("},") || trimmed.ends_with("}],") {
                builder.upstream.push(upstream.build()?);
            } else {
                current_upstream = Some(upstream);
            }
        } else if let Some(value) = extract_string_field(trimmed, "id") {
            builder.id = Some(value);
        } else if let Some(value) = extract_string_field(trimmed, "category") {
            builder.category = Some(value);
        } else if let Some(value) = extract_string_field(trimmed, "name") {
            builder.name = Some(value);
        } else if let Some(value) = extract_string_field(trimmed, "description") {
            builder.description = Some(value);
        }
    }

    validate_registry_catalog(&samples)?;
    Ok(samples)
}

impl PageSampleBuilder {
    fn build(self) -> Result<RegistrySample> {
        Ok(RegistrySample {
            id: required_registry_field(self.id, "id")?,
            category: required_registry_field(self.category, "category")?,
            name: required_registry_field(self.name, "name")?,
            description: required_registry_field(self.description, "description")?,
            upstream: self.upstream,
        })
    }
}

impl UpstreamSampleBuilder {
    fn build(self) -> Result<RegistryUpstreamSample> {
        Ok(RegistryUpstreamSample {
            category: required_registry_field(self.category, "upstream.category")?,
            name: required_registry_field(self.name, "upstream.name")?,
            mode: required_registry_field(self.mode, "upstream.mode")?,
        })
    }
}

fn required_registry_field(value: Option<String>, field: &str) -> Result<String> {
    value.ok_or_else(|| Error::Message(format!("SCENE_REGISTRY entry is missing `{field}`")))
}

fn read_upstream_fields(builder: &mut UpstreamSampleBuilder, line: &str) {
    if let Some(value) = extract_string_field(line, "category") {
        builder.category = Some(value);
    }
    if let Some(value) = extract_string_field(line, "name") {
        builder.name = Some(value);
    }
    if let Some(value) = extract_parity_mode_field(line) {
        builder.mode = Some(value);
    }
}

fn extract_parity_mode_field(line: &str) -> Option<String> {
    let needle = "mode: ParityMode::";
    let start = line.find(needle)? + needle.len();
    let tail = &line[start..];
    let end = tail
        .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .unwrap_or(tail.len());
    Some(tail[..end].to_owned())
}

fn validate_registry_catalog(samples: &[RegistrySample]) -> Result<()> {
    if samples.is_empty() {
        return Err(Error::Message(
            "testbed registry must contain at least one entry".to_owned(),
        ));
    }

    let mut seen = BTreeSet::new();
    for sample in samples {
        validate_registry_field(sample, "id", &sample.id)?;
        validate_registry_field(sample, "category", &sample.category)?;
        validate_registry_field(sample, "name", &sample.name)?;
        validate_registry_field(sample, "description", &sample.description)?;
        if sample.upstream.is_empty() {
            return Err(Error::Message(format!(
                "testbed registry sample `{}` must include upstream sample references",
                sample.id
            )));
        }
        if !is_slug(&sample.id) {
            return Err(Error::Message(format!(
                "testbed registry id `{}` must be a lowercase ASCII slug",
                sample.id
            )));
        }
        if !seen.insert(sample.id.as_str()) {
            return Err(Error::Message(format!(
                "duplicate testbed registry id `{}`",
                sample.id
            )));
        }

        let mut upstream_seen = BTreeSet::new();
        for upstream in &sample.upstream {
            validate_registry_field(sample, "upstream.category", &upstream.category)?;
            validate_registry_field(sample, "upstream.name", &upstream.name)?;
            validate_registry_field(sample, "upstream.mode", &upstream.mode)?;
            if !matches!(
                upstream.mode.as_str(),
                "FaithfulPort" | "TeachingAdaptation"
            ) {
                return Err(Error::Message(format!(
                    "testbed registry sample `{}` uses unsupported upstream parity mode `{}`",
                    sample.id, upstream.mode
                )));
            }
            if !upstream_seen.insert((upstream.category.as_str(), upstream.name.as_str())) {
                return Err(Error::Message(format!(
                    "testbed registry sample `{}` duplicates upstream ref `{}` / `{}`",
                    sample.id, upstream.category, upstream.name
                )));
            }
        }
    }

    Ok(())
}

fn validate_registry_field(sample: &RegistrySample, field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(Error::Message(format!(
            "testbed registry sample `{}` has an empty `{field}` field",
            sample.id
        )))
    } else {
        Ok(())
    }
}

fn is_slug(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn reset_generated_examples_dir(pages_dir: &Path, examples_dir: &Path) -> Result<()> {
    if !examples_dir.exists() {
        fs::create_dir_all(examples_dir).map_err(|source| Error::io(examples_dir, source))?;
        return Ok(());
    }
    let pages_dir = pages_dir
        .canonicalize()
        .map_err(|source| Error::io(pages_dir, source))?;
    let examples_dir = examples_dir
        .canonicalize()
        .map_err(|source| Error::io(examples_dir, source))?;
    if !examples_dir.starts_with(&pages_dir)
        || examples_dir.file_name().and_then(|name| name.to_str()) != Some("examples")
    {
        return Err(Error::Message(format!(
            "refusing to replace unexpected generated examples dir: {}",
            examples_dir.display()
        )));
    }
    fs::remove_dir_all(&examples_dir).map_err(|source| Error::io(&examples_dir, source))?;
    fs::create_dir_all(&examples_dir).map_err(|source| Error::io(&examples_dir, source))
}

fn bevy_example_index_page(samples: &[RegistrySample], location: ExampleIndexLocation) -> String {
    let links = samples
        .iter()
        .map(|sample| {
            format!(
                "        <a class=\"card\" href=\"{href}\"><span>{category}</span><strong>{name}</strong><small>{description}</small><em>{upstream}</em></a>",
                href = location.example_href(&sample.id),
                category = escape_html(&sample.category),
                name = escape_html(&sample.name),
                description = escape_html(&sample.description),
                upstream = upstream_summary(&sample.upstream)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>boxdd Bevy Examples</title>
  <link rel="icon" href="data:,">
  <meta name="description" content="Direct Bevy Web examples for boxdd.">
  <style>{css}</style>
</head>
<body>
  <div class="directory">
    <header class="topbar">
      <a href="{home_href}">boxdd Examples</a>
      <nav>
        <a href="https://github.com/Latias94/boxdd">GitHub</a>
        <a href="https://docs.rs/boxdd">Docs.rs</a>
      </nav>
    </header>
    <main class="directory-main">
      <p class="eyebrow">Bevy Web examples</p>
      <h1>Run a Box2D scene</h1>
      <p class="lead">Each entry opens a dedicated Bevy + egui WASM page backed by the same Box2D provider runtime.</p>
      <section class="card-grid">
{links}
      </section>
    </main>
  </div>
</body>
</html>
"#,
        css = example_page_css(),
        home_href = location.home_href(),
        links = links
    )
}

fn bevy_testbed_page() -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>boxdd Bevy Testbed</title>
  <link rel="icon" href="data:,">
  <meta name="description" content="Bevy + egui WASM testbed for boxdd.">
  <style>{css}</style>
</head>
<body>
  <div class="shell">
    <header class="topbar">
      <div>
        <a href="../">boxdd Examples</a>
        <h1>Bevy Testbed</h1>
        <p><span>All scenes</span> Switch scenes from the egui panel.</p>
      </div>
      <nav>
        <a href="../examples/">All Bevy examples</a>
        <a href="https://github.com/Latias94/boxdd/tree/main/bevy_boxdd/examples/testbed_2d">Source</a>
      </nav>
    </header>
    <main id="bevy-app" data-scene-id="" data-scene-name="Bevy Testbed" data-scene-category="All scenes">
      <canvas id="bevy-canvas" tabindex="0"></canvas>
      <div id="bevy-status" role="status" aria-live="polite">
        <strong>Loading Bevy Testbed</strong>
        <span>Preparing the shared Box2D provider and the Rust Bevy wasm module.</span>
      </div>
    </main>
  </div>
  <script type="module" src="loader.js"></script>
</body>
</html>
"#,
        css = example_page_css()
    )
}

fn bevy_example_page(sample: &RegistrySample) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{name} - boxdd Bevy Example</title>
  <link rel="icon" href="data:,">
  <meta name="description" content="{description}">
  <style>{css}</style>
</head>
<body>
  <div class="shell">
    <header class="topbar">
      <div>
        <a href="../../">boxdd Examples</a>
        <h1>{name}</h1>
        <p><span>{category}</span>{description}</p>
        {upstream}
      </div>
      <nav>
        <a href="../">All Bevy examples</a>
        <a href="https://github.com/Latias94/boxdd/tree/main/bevy_boxdd/examples/testbed_2d">Source</a>
      </nav>
    </header>
    <main id="bevy-app" data-scene-id="{id}" data-scene-name="{name}" data-scene-category="{category}">
      <canvas id="bevy-canvas" tabindex="0"></canvas>
      <div id="bevy-status" role="status" aria-live="polite">
        <strong>Loading {name}</strong>
        <span>Preparing the shared Box2D provider and the Rust Bevy wasm module.</span>
      </div>
    </main>
  </div>
  <script type="module" src="../../bevy-testbed/loader.js"></script>
</body>
</html>
"#,
        id = escape_html(&sample.id),
        name = escape_html(&sample.name),
        category = escape_html(&sample.category),
        description = escape_html(&sample.description),
        upstream = source_list_html(sample),
        css = example_page_css()
    )
}

fn upstream_summary(upstream: &[RegistryUpstreamSample]) -> String {
    let mut labels = upstream
        .iter()
        .take(3)
        .map(|sample| format!("{} / {}", sample.category, sample.name))
        .collect::<Vec<_>>();
    if upstream.len() > labels.len() {
        labels.push(format!("+{} more", upstream.len() - labels.len()));
    }
    escape_html(&labels.join(", "))
}

fn source_list_html(sample: &RegistrySample) -> String {
    let mut items = String::new();
    for upstream in &sample.upstream {
        write!(
            items,
            "<span>{category} / {name} · {mode}</span>",
            category = escape_html(&upstream.category),
            name = escape_html(&upstream.name),
            mode = escape_html(&parity_mode_label(&upstream.mode))
        )
        .expect("writing to String cannot fail");
    }
    format!(r#"<div class="upstream-list">{items}</div>"#)
}

fn parity_mode_label(mode: &str) -> String {
    let mut label = String::new();
    for (index, ch) in mode.chars().enumerate() {
        if index > 0 && ch.is_ascii_uppercase() {
            label.push(' ');
        }
        label.push(ch.to_ascii_lowercase());
    }
    label
}

fn bevy_testbed_loader_js() -> &'static str {
    r##"const statusPanel = document.querySelector("#bevy-status");
const appRoot = document.querySelector("#bevy-app");
const sceneId = appRoot?.dataset.sceneId || "";
const sceneName = appRoot?.dataset.sceneName || "Bevy testbed";
const isExamplePage = Boolean(sceneId);

function setStatus(state, title, detail, progress) {
  statusPanel.dataset.state = state;
  statusPanel.replaceChildren();

  const titleNode = document.createElement("strong");
  titleNode.textContent = title;
  const detailNode = document.createElement("span");
  detailNode.textContent = detail;
  statusPanel.append(titleNode, detailNode);

  if (progress) {
    const progressNode = document.createElement("progress");
    progressNode.value = progress.loaded;
    if (progress.total) {
      progressNode.max = progress.total;
    } else {
      progressNode.removeAttribute("value");
    }

    const progressText = document.createElement("small");
    progressText.textContent = progressTextFor(progress.loaded, progress.total);
    statusPanel.append(progressNode, progressText);
  }
}

function generatedUrl(path) {
  return new URL(path, import.meta.url);
}

function progressTextFor(loaded, total) {
  if (total) {
    const percent = Math.min(100, Math.round((loaded / total) * 100));
    return `${formatBytes(loaded)} / ${formatBytes(total)} (${percent}%)`;
  }
  return `${formatBytes(loaded)} downloaded`;
}

function formatBytes(bytes) {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return "0 B";
  }
  const units = ["B", "KiB", "MiB", "GiB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return unit === 0 ? `${value} ${units[unit]}` : `${value.toFixed(2)} ${units[unit]}`;
}

async function fetchArrayBufferWithProgress(url, label) {
  setStatus("loading", `Downloading ${label}`, "Starting download.", { loaded: 0, total: 0 });
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`${label} download failed with HTTP ${response.status}`);
  }

  const total = Number(response.headers.get("Content-Length")) || 0;
  if (!response.body) {
    const buffer = await response.arrayBuffer();
    setStatus("loading", `Downloading ${label}`, "Download complete.", {
      loaded: buffer.byteLength,
      total: total || buffer.byteLength,
    });
    return buffer;
  }

  const reader = response.body.getReader();
  const chunks = [];
  let loaded = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) {
      break;
    }
    chunks.push(value);
    loaded += value.byteLength;
    setStatus("loading", `Downloading ${label}`, "Downloading runtime asset.", { loaded, total });
  }

  const bytes = new Uint8Array(loaded);
  let offset = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, offset);
    offset += chunk.byteLength;
  }
  setStatus("loading", `Downloading ${label}`, "Download complete.", { loaded, total: total || loaded });
  return bytes.buffer;
}

async function main() {
  const providerGenerated = new URL("../wasm/generated/", import.meta.url);
  const providerWasmUrl = new URL("box2d-sys-v1-single.wasm", providerGenerated);
  const bevyWasmUrl = generatedUrl("generated/bevy_boxdd_testbed_bg.wasm");

  setStatus("loading", "Loading JavaScript modules", `Preparing the browser runtime for ${sceneName}.`);
  const [
    { default: createProvider },
    { default: initBevyTestbed },
    { setBox2dProvider, setBoxddAppExports },
  ] =
    await Promise.all([
      import(new URL("box2d-sys-v1-single.js", providerGenerated).href),
      import(generatedUrl("generated/bevy_boxdd_testbed.js").href),
      import(generatedUrl("generated/box2d-provider-shim.js").href),
    ]);
  const memory = new WebAssembly.Memory({ initial: 4096, maximum: 8192 });

  const providerWasm = await fetchArrayBufferWithProgress(providerWasmUrl, "Box2D provider wasm");
  setStatus("loading", "Starting Box2D provider", `Instantiating the shared Box2D C provider for ${sceneName}.`);
  const provider = await createProvider({
    wasmMemory: memory,
    wasmBinary: providerWasm,
    locateFile: (path) => new URL(path, providerGenerated).href,
    print: (text) => console.log(`[box2d-sys-v1-single] ${text}`),
    printErr: (text) => console.warn(`[box2d-sys-v1-single] ${text}`),
  });

  if (provider.wasmMemory && provider.wasmMemory !== memory) {
    throw new Error("Box2D provider did not use the shared WebAssembly.Memory");
  }

  setBox2dProvider(provider);
  const bevyWasm = await fetchArrayBufferWithProgress(bevyWasmUrl, `${sceneName} Bevy wasm`);
  setStatus("loading", `Starting ${sceneName}`, "Instantiating the Rust Bevy + egui wasm module.");

  const bevyExports = await initBevyTestbed({
    module_or_path: bevyWasm,
    memory,
  });
  setBoxddAppExports(bevyExports);

  window.BOXDD_BEVY_TESTBED_READY = true;
  window.BOXDD_BEVY_EXAMPLE_READY = true;
  window.BOXDD_BEVY_SCENE_ID = sceneId;
  setStatus(
    "running",
    `${sceneName} running`,
    isExamplePage
      ? "This dedicated example page is running the selected Box2D scene in Bevy."
      : "The scene browser, egui controls, and Box2D simulation are running in this canvas.",
  );
}

main().catch((error) => {
  console.error(error);
  const message = error instanceof Error ? error.message : String(error);
  setStatus("error", `${sceneName} failed`, message);
});
"##
}

impl ExampleIndexLocation {
    fn home_href(self) -> &'static str {
        match self {
            Self::Root => "./",
            Self::ExamplesDirectory => "../",
        }
    }

    fn example_href(self, id: &str) -> String {
        match self {
            Self::Root => format!("examples/{id}/"),
            Self::ExamplesDirectory => format!("{id}/"),
        }
    }
}

fn example_page_css() -> &'static str {
    r#"
:root {
  color-scheme: dark;
  --background: #09090b;
  --foreground: #fafafa;
  --card: #0f0f12;
  --muted: #a1a1aa;
  --border: #27272a;
  --accent: #2dd4bf;
  --danger: #f87171;
}
* { box-sizing: border-box; }
html, body { width: 100%; height: 100%; margin: 0; background: var(--background); color: var(--foreground); font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
a { color: var(--foreground); text-decoration: none; }
a:hover { text-decoration: underline; text-underline-offset: 4px; }
.shell { display: grid; grid-template-rows: auto minmax(0, 1fr); width: 100%; height: 100%; }
.topbar { display: flex; flex-wrap: wrap; gap: 14px; align-items: center; justify-content: space-between; border-bottom: 1px solid var(--border); background: rgba(9, 9, 11, 0.94); padding: 14px 18px; }
.topbar h1 { margin: 4px 0 0; font-size: 20px; line-height: 1.2; letter-spacing: 0; }
.topbar p { display: flex; flex-wrap: wrap; gap: 8px; margin: 5px 0 0; color: var(--muted); font-size: 13px; }
.topbar p span, .eyebrow { color: var(--accent); font-weight: 700; text-transform: uppercase; }
.topbar nav { display: flex; flex-wrap: wrap; gap: 12px; color: var(--muted); font-size: 14px; }
#bevy-app { position: relative; min-width: 0; min-height: 0; background: #020617; }
#bevy-canvas { display: block; width: 100%; height: 100%; outline: none; touch-action: none; }
#bevy-status { position: absolute; left: 18px; bottom: 18px; max-width: min(560px, calc(100% - 36px)); border: 1px solid var(--border); border-radius: 8px; background: rgba(15, 15, 18, 0.94); padding: 12px 14px; color: var(--muted); font-size: 14px; line-height: 1.45; }
#bevy-status strong { display: block; margin-bottom: 4px; color: var(--foreground); font-size: 15px; }
#bevy-status progress { display: block; width: min(360px, 100%); height: 8px; margin-top: 10px; accent-color: var(--accent); }
#bevy-status small { display: block; margin-top: 6px; color: #d4d4d8; font-size: 12px; }
#bevy-status[data-state="error"] strong { color: var(--danger); }
#bevy-status[data-state="running"] { opacity: 0; pointer-events: none; transition: opacity 180ms ease; }
.directory { min-height: 100%; }
.directory-main { width: min(1180px, calc(100% - 32px)); margin: 0 auto; padding: 54px 0; }
.directory-main h1 { margin: 0; font-size: clamp(34px, 6vw, 58px); line-height: 1; letter-spacing: 0; }
.lead { max-width: 720px; color: var(--muted); font-size: 17px; }
.card-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); gap: 12px; margin-top: 28px; }
.card { display: grid; min-height: 150px; gap: 8px; border: 1px solid var(--border); border-radius: 8px; background: var(--card); padding: 16px; }
.card:hover { border-color: #52525b; text-decoration: none; }
.card span { color: var(--accent); font-size: 12px; font-weight: 700; text-transform: uppercase; }
.card strong { font-size: 18px; }
.card small { color: var(--muted); font-size: 13px; line-height: 1.5; }
.card em { color: #d4d4d8; font-size: 12px; font-style: normal; line-height: 1.45; }
.upstream-list { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 8px; }
.upstream-list span { border: 1px solid var(--border); border-radius: 999px; background: rgba(39, 39, 42, 0.7); padding: 4px 7px; color: #d4d4d8; font-size: 12px; line-height: 1.2; text-transform: none; }
"#
}

fn extract_string_field(line: &str, field: &str) -> Option<String> {
    let prefix = format!("{field}: ");
    let rest = line.strip_prefix(&prefix)?;
    extract_quoted_string(rest)
}

fn extract_quoted_string(value: &str) -> Option<String> {
    let start = value.find('"')?;
    let rest = &value[start + 1..];
    let end = rest.find('"')?;
    Some(rest[..end].to_owned())
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(crate) fn validate_pages(root: &Path) -> Result<()> {
    let pages_dir = root.join("docs/pages");
    let samples = read_testbed_registry(root)?;
    let expected_pages = expected_bevy_pages(root, &samples);
    let html_files = collect_html_files(&pages_dir)?;
    if html_files.is_empty() {
        return Err(Error::Message(format!(
            "no html pages found under {}",
            pages_dir.display()
        )));
    }

    let expected_paths: BTreeSet<PathBuf> = expected_pages.keys().cloned().collect();
    let actual_paths: BTreeSet<PathBuf> = html_files.iter().cloned().collect();
    let mut errors = Vec::new();
    for stale in actual_paths.difference(&expected_paths) {
        errors.push(format!(
            "{} is not generated by `cargo run -p xtask -- generate-pages`",
            stale.strip_prefix(root).unwrap_or(stale).display()
        ));
    }
    for (path, expected) in &expected_pages {
        if !path.exists() {
            errors.push(format!(
                "missing generated page {}",
                path.strip_prefix(root).unwrap_or(path).display()
            ));
            continue;
        }
        let actual = fs::read_to_string(path).map_err(|source| Error::io(path, source))?;
        if normalize_newlines(&actual) != normalize_newlines(expected) {
            errors.push(format!(
                "{} is stale; run `cargo run -p xtask -- generate-pages`",
                path.strip_prefix(root).unwrap_or(path).display()
            ));
        }
    }

    for file in &html_files {
        let content = fs::read_to_string(file).map_err(|source| Error::io(file, source))?;
        for link in extract_links(&content) {
            if should_skip_link(&link) {
                continue;
            }
            let without_fragment = link.split('#').next().unwrap_or_default();
            if without_fragment.is_empty() {
                continue;
            }
            let target = file.parent().unwrap_or(root).join(without_fragment);
            if !target.exists() {
                errors.push(format!(
                    "{} links to missing local target `{}`",
                    file.strip_prefix(root).unwrap_or(file).display(),
                    link
                ));
            }
        }
    }

    let loader = pages_bevy_testbed_dir(root).join("loader.js");
    if !loader.exists() {
        errors.push(
            "missing generated Bevy testbed loader docs/pages/bevy-testbed/loader.js".to_owned(),
        );
    } else {
        let actual = fs::read_to_string(&loader).map_err(|source| Error::io(&loader, source))?;
        if normalize_newlines(&actual) != normalize_newlines(bevy_testbed_loader_js()) {
            errors.push(
                "docs/pages/bevy-testbed/loader.js is stale; run `cargo run -p xtask -- generate-pages`".to_owned(),
            );
        }
        for required in [
            "box2d-provider-shim.js",
            "setBox2dProvider",
            "setBoxddAppExports",
            "bevyExports",
        ] {
            if !actual.contains(required) {
                errors.push(format!(
                    "{} is missing required Bevy provider glue `{required}`",
                    loader.strip_prefix(root).unwrap_or(&loader).display()
                ));
            }
        }
    }

    let wasm_generated = pages_wasm_generated_dir(root);
    if wasm_generated.exists() {
        for asset in ["box2d-sys-v1-single.js", "box2d-sys-v1-single.wasm"] {
            let path = wasm_generated.join(asset);
            if !path.is_file() {
                errors.push(format!(
                    "missing provider wasm asset {}; run `cargo run -p xtask -- build-pages-wasm`",
                    path.strip_prefix(root).unwrap_or(&path).display()
                ));
            }
        }
    }
    let bevy_generated = pages_bevy_generated_dir(root);
    if bevy_generated.exists() {
        for asset in [BEVY_WEB_JS, BEVY_WEB_WASM, BEVY_PROVIDER_SHIM] {
            let path = bevy_generated.join(asset);
            if !path.is_file() {
                errors.push(format!(
                    "missing Bevy wasm asset {}; run `cargo run -p xtask -- build-pages-wasm`",
                    path.strip_prefix(root).unwrap_or(&path).display()
                ));
            }
        }
    }

    if errors.is_empty() {
        println!(
            "pages ok: {} html files checked, {} Bevy WASM examples",
            html_files.len(),
            samples.len()
        );
        Ok(())
    } else {
        Err(Error::Message(errors.join("\n")))
    }
}

fn normalize_newlines(value: &str) -> String {
    value.replace("\r\n", "\n")
}

fn collect_html_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    collect_html_files_into(dir, &mut out)?;
    out.sort();
    Ok(out)
}

fn collect_html_files_into(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir).map_err(|source| Error::io(dir, source))? {
        let entry = entry.map_err(|source| Error::io(dir, source))?;
        let path = entry.path();
        if path.is_dir() {
            collect_html_files_into(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "html") {
            out.push(path);
        }
    }
    Ok(())
}

fn extract_links(content: &str) -> Vec<String> {
    let mut links = Vec::new();
    for attr in ["href=\"", "src=\""] {
        let mut rest = content;
        while let Some(index) = rest.find(attr) {
            rest = &rest[index + attr.len()..];
            let Some(end) = rest.find('"') else {
                break;
            };
            links.push(rest[..end].to_owned());
            rest = &rest[end + 1..];
        }
    }
    links
}

fn should_skip_link(link: &str) -> bool {
    link.starts_with('#')
        || link.starts_with("http://")
        || link.starts_with("https://")
        || link.starts_with("mailto:")
        || link.starts_with("data:")
        || link.starts_with('/')
}

#[cfg(test)]
mod tests {
    use super::{ProviderPrecision, format_bytes, validate_pages_precision};

    #[test]
    fn format_bytes_uses_binary_units() {
        assert_eq!(format_bytes(31), "31 B");
        assert_eq!(format_bytes(1536), "1.50 KiB");
        assert_eq!(format_bytes(2 * 1024 * 1024), "2.00 MiB");
    }

    #[test]
    fn pages_rejects_unimplemented_double_precision_loader() {
        assert!(validate_pages_precision(ProviderPrecision::Single).is_ok());
        assert!(validate_pages_precision(ProviderPrecision::Double).is_err());
    }
}
