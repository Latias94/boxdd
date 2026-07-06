use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    fmt::Write as _,
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};

type Result<T> = std::result::Result<T, Error>;

const PROVIDER_MODULE: &str = "box2d-sys-v0";
const WASM_TARGET: &str = "wasm32-unknown-unknown";
const PROVIDER_SMOKE_PACKAGE: &str = "boxdd-provider-smoke";
const PROVIDER_SMOKE_WASM: &str = "boxdd_provider_smoke.wasm";
const PAGES_WASM_DIR: &str = "wasm/generated";
const BEVY_EXAMPLES_DIR: &str = "examples";
const BEVY_WEB_EXAMPLE: &str = "testbed_2d";
const BEVY_WEB_OUT_DIR: &str = "bevy-testbed/generated";
const BEVY_WEB_OUT_NAME: &str = "bevy_boxdd_testbed";
const BEVY_WEB_JS: &str = "bevy_boxdd_testbed.js";
const BEVY_WEB_WASM: &str = "bevy_boxdd_testbed_bg.wasm";
const BEVY_PROVIDER_SHIM: &str = "box2d-provider-shim.js";
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

#[derive(Debug)]
enum Error {
    Io { path: PathBuf, source: io::Error },
    Message(String),
}

impl Error {
    fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "{}: {}", path.display(), source),
            Self::Message(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
struct Sample {
    category: String,
    name: String,
    source: String,
}

#[derive(Debug, Clone)]
struct MatrixRow {
    category: String,
    name: String,
    status: String,
    artifact: String,
    source: String,
}

struct SampleCoverage {
    status: &'static str,
    artifact: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
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

#[derive(Copy, Clone)]
enum ExampleIndexLocation {
    Root,
    ExamplesDirectory,
}

#[derive(Copy, Clone)]
enum BuildProfile {
    Debug,
    Release,
}

impl BuildProfile {
    fn from_env() -> Self {
        match env::var("BOXDD_PAGES_WASM_PROFILE").ok().as_deref() {
            Some("release") | Some("Release") | Some("RELEASE") => Self::Release,
            _ => Self::Debug,
        }
    }

    const fn cargo_args(self) -> &'static [&'static str] {
        match self {
            Self::Debug => &[],
            Self::Release => &["--release"],
        }
    }

    const fn target_dir(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Release => "release",
        }
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let root = workspace_root()?;
    let args: Vec<String> = env::args().skip(1).collect();
    match args.as_slice() {
        [] => {
            print_help();
            Ok(())
        }
        [arg] if arg == "help" || arg == "--help" || arg == "-h" => {
            print_help();
            Ok(())
        }
        [cmd, rest @ ..] if cmd == "api-coverage" => api_coverage(&root, rest),
        [cmd, rest @ ..] if cmd == "sample-parity" => sample_parity(&root, rest),
        [cmd] if cmd == "provider-smoke-app" => provider_smoke_app(&root),
        [cmd] if cmd == "provider-smoke" => provider_smoke(&root),
        [cmd] if cmd == "build-pages-wasm" => build_pages_wasm(&root),
        [cmd] if cmd == "generate-pages" => generate_pages(&root),
        [cmd] if cmd == "validate-pages" => validate_pages(&root),
        [cmd, ..] => Err(Error::Message(format!(
            "unknown xtask command `{cmd}`; run `cargo run -p xtask -- help`"
        ))),
    }
}

fn workspace_root() -> Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| Error::Message("xtask manifest has no parent directory".to_owned()))
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
  cargo run -p xtask -- provider-smoke-app
  cargo run -p xtask -- provider-smoke
  cargo run -p xtask -- build-pages-wasm
  cargo run -p xtask -- generate-pages
  cargo run -p xtask -- validate-pages

Commands:
  api-coverage  Validate or regenerate docs/api-coverage.md and its fixture
  sample-parity  Validate or regenerate docs/upstream-parity/box2d-sample-matrix.md
  provider-smoke-app  Build the Rust wasm provider-smoke app and export list
  provider-smoke  Build the Rust app, build the Box2D provider with emcc, and run Node smoke
  build-pages-wasm  Build browser provider and Bevy testbed assets into docs/pages
  generate-pages Generate the GitHub Pages Bevy example index from SCENE_REGISTRY
  validate-pages Validate generated pages and local links in docs/pages/**/*.html
"
    );
}

fn api_coverage(root: &Path, args: &[String]) -> Result<()> {
    let mode = match args {
        [arg] if arg == "--check" => ApiCoverageMode::Check,
        [arg] if arg == "--write" => ApiCoverageMode::Write,
        [] => ApiCoverageMode::Check,
        _ => {
            return Err(Error::Message(
                "api-coverage expects --check or --write".to_owned(),
            ));
        }
    };

    let symbols = discover_b2_api_symbols(root)?;
    let fixture_path = root.join("boxdd/tests/fixtures/api_coverage_symbols.txt");
    let docs_path = root.join("docs/api-coverage.md");

    match mode {
        ApiCoverageMode::Write => {
            let safe_source = collect_rust_source(root)?;
            let rows = classify_api_symbols(&symbols, &safe_source);
            write_api_fixture(&fixture_path, &rows)?;
            write_api_coverage_doc(&docs_path, &rows)?;
            println!(
                "wrote {} API coverage rows to {}",
                rows.len(),
                fixture_path.display()
            );
            Ok(())
        }
        ApiCoverageMode::Check => {
            let rows = read_api_fixture(&fixture_path)?;
            validate_api_coverage(&symbols, &rows)?;
            println!(
                "api coverage ok: {} vendored B2_API symbols classified",
                rows.len()
            );
            Ok(())
        }
    }
}

enum ApiCoverageMode {
    Check,
    Write,
}

#[derive(Debug, Clone)]
struct ApiRow {
    symbol: String,
    status: String,
    surface: String,
    notes: String,
}

fn discover_b2_api_symbols(root: &Path) -> Result<BTreeSet<String>> {
    let include_dir = root.join("boxdd-sys/third-party/box2d/include/box2d");
    let mut symbols = BTreeSet::new();
    for entry in fs::read_dir(&include_dir).map_err(|source| Error::io(&include_dir, source))? {
        let entry = entry.map_err(|source| Error::io(&include_dir, source))?;
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "h") {
            continue;
        }
        let content = fs::read_to_string(&path).map_err(|source| Error::io(&path, source))?;
        let mut decl = String::new();
        for line in content.lines() {
            if line.contains("B2_API") || !decl.is_empty() {
                decl.push(' ');
                decl.push_str(line.trim());
                if line.contains(';') {
                    if let Some(symbol) = parse_b2_api_symbol(&decl) {
                        symbols.insert(symbol);
                    }
                    decl.clear();
                }
            }
        }
    }
    if symbols.is_empty() {
        return Err(Error::Message(format!(
            "no B2_API symbols found under {}",
            include_dir.display()
        )));
    }
    Ok(symbols)
}

fn parse_b2_api_symbol(decl: &str) -> Option<String> {
    let before_paren = decl.split('(').next()?;
    let name = before_paren
        .split_whitespace()
        .last()?
        .trim_start_matches('*')
        .trim();
    name.starts_with("b2").then(|| name.to_owned())
}

fn collect_rust_source(root: &Path) -> Result<String> {
    let src_dir = root.join("boxdd/src");
    let mut source = String::new();
    collect_rust_source_into(&src_dir, &mut source)?;
    Ok(source)
}

fn collect_rust_source_into(dir: &Path, out: &mut String) -> Result<()> {
    for entry in fs::read_dir(dir).map_err(|source| Error::io(dir, source))? {
        let entry = entry.map_err(|source| Error::io(dir, source))?;
        let path = entry.path();
        if path.is_dir() {
            collect_rust_source_into(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push_str(&fs::read_to_string(&path).map_err(|source| Error::io(&path, source))?);
            out.push('\n');
        }
    }
    Ok(())
}

fn classify_api_symbols(symbols: &BTreeSet<String>, safe_source: &str) -> Vec<ApiRow> {
    symbols
        .iter()
        .map(|symbol| {
            let (status, notes) = if let Some(notes) = omitted_symbol_note(symbol) {
                ("omitted", notes)
            } else if safe_source.contains(symbol) {
                ("safe", "Referenced by the Rust safe layer.")
            } else {
                ("raw", raw_symbol_note(symbol))
            };
            ApiRow {
                symbol: symbol.clone(),
                status: status.to_owned(),
                surface: api_surface(symbol).to_owned(),
                notes: notes.to_owned(),
            }
        })
        .collect()
}

fn omitted_symbol_note(symbol: &str) -> Option<&'static str> {
    match symbol {
        "b2World_DumpMemoryStats" => Some(
            "Intentionally omitted: upstream writes fixed diagnostic output, so callers should use upstream diagnostics tooling explicitly.",
        ),
        "b2World_RebuildStaticTree" => Some(
            "Intentionally omitted: upstream labels this as internal testing support, not stable runtime API.",
        ),
        _ => None,
    }
}

fn raw_symbol_note(symbol: &str) -> &'static str {
    match symbol {
        "b2InternalAssertFcn" => {
            "Raw only: upstream internal assert implementation, not a stable safe API surface."
        }
        "b2SetAllocator" => {
            "Raw only: process-global allocator hook needs a scoped startup guard before safe exposure."
        }
        "b2SetAssertFcn" => {
            "Raw only: process-global assert callback has panic/callback unwinding semantics that need a dedicated design."
        }
        "b2SetLogFcn" => {
            "Raw only: process-global log callback needs a scoped callback guard before safe exposure."
        }
        _ => "Available through boxdd_sys::ffi; safe wrapper not assigned yet.",
    }
}

fn api_surface(symbol: &str) -> &'static str {
    if symbol.starts_with("b2World") || symbol == "b2CreateWorld" || symbol == "b2DestroyWorld" {
        "World"
    } else if symbol.starts_with("b2Body") || symbol == "b2CreateBody" || symbol == "b2DestroyBody"
    {
        "Body"
    } else if symbol.starts_with("b2Shape") || symbol.contains("Shape") {
        "Shape"
    } else if symbol.starts_with("b2Chain") || symbol.contains("Chain") {
        "Chain"
    } else if symbol.starts_with("b2DynamicTree") {
        "DynamicTree"
    } else if symbol.contains("Joint") {
        "Joint"
    } else if symbol.contains("RayCast")
        || symbol.contains("Collide")
        || symbol.contains("Distance")
        || symbol.contains("TOI")
        || symbol.contains("Hull")
        || symbol.contains("Manifold")
        || symbol.contains("AABB")
    {
        "Collision"
    } else if symbol.contains("Vec")
        || symbol.contains("Rot")
        || symbol.contains("Transform")
        || symbol.contains("LengthUnits")
        || symbol == "b2Atan2"
    {
        "Math"
    } else {
        "Foundation"
    }
}

fn write_api_fixture(path: &Path, rows: &[ApiRow]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::io(parent, source))?;
    }
    let mut output = String::new();
    output.push_str("# symbol|status|surface|notes\n");
    output.push_str("# status is one of safe, raw, omitted, deferred\n");
    for row in rows {
        output.push_str(&format!(
            "{}|{}|{}|{}\n",
            row.symbol, row.status, row.surface, row.notes
        ));
    }
    fs::write(path, output).map_err(|source| Error::io(path, source))
}

fn write_api_coverage_doc(path: &Path, rows: &[ApiRow]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::io(parent, source))?;
    }
    let counts = api_counts(rows);
    let mut by_surface: BTreeMap<&str, ApiCounts> = BTreeMap::new();
    for row in rows {
        by_surface.entry(&row.surface).or_default().add(&row.status);
    }

    let mut output = String::new();
    output.push_str("# Box2D API Coverage\n\n");
    output.push_str(&format!(
        "<!-- api-coverage: total={} safe={} raw={} omitted={} deferred={} -->\n\n",
        counts.total, counts.safe, counts.raw, counts.omitted, counts.deferred
    ));
    output.push_str("This document summarizes how `boxdd` accounts for every vendored `B2_API` function under `boxdd-sys/third-party/box2d/include/box2d`.\n");
    output.push_str("The authoritative per-symbol fixture is `boxdd/tests/fixtures/api_coverage_symbols.txt`, and `cargo nextest run -p boxdd --test api_coverage` validates that it matches the vendored headers and this summary.\n\n");
    output.push_str("## Status Values\n\n");
    output.push_str("- `safe`: represented by the Rust safe layer.\n");
    output.push_str(
        "- `raw`: available through `boxdd_sys::ffi`; no stable safe wrapper is assigned yet.\n",
    );
    output.push_str("- `omitted`: intentionally excluded from the safe layer with rationale.\n");
    output.push_str("- `deferred`: planned but not yet implemented.\n\n");
    output.push_str("## Summary\n\n");
    output.push_str("| Status | Count |\n|---|---:|\n");
    output.push_str(&format!("| `safe` | {} |\n", counts.safe));
    output.push_str(&format!("| `raw` | {} |\n", counts.raw));
    output.push_str(&format!("| `omitted` | {} |\n", counts.omitted));
    output.push_str(&format!("| `deferred` | {} |\n", counts.deferred));
    output.push_str(&format!("| Total | {} |\n\n", counts.total));
    output.push_str("## By Surface\n\n");
    output.push_str(
        "| Surface | Safe | Raw | Omitted | Deferred | Total |\n|---|---:|---:|---:|---:|---:|\n",
    );
    for (surface, counts) in by_surface {
        output.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            surface, counts.safe, counts.raw, counts.omitted, counts.deferred, counts.total
        ));
    }
    output.push_str("\n## Maintenance\n\n");
    output.push_str("- Run `cargo run -p xtask -- api-coverage --write` after changing vendored Box2D or adding safe wrappers.\n");
    output.push_str(
        "- Review every `raw`, `omitted`, and `deferred` row before a breaking release.\n",
    );
    output.push_str("- Treat vendored headers as the exact API source; online Box2D docs may describe a nearby but different release.\n");

    fs::write(path, output).map_err(|source| Error::io(path, source))
}

fn read_api_fixture(path: &Path) -> Result<Vec<ApiRow>> {
    let content = fs::read_to_string(path).map_err(|source| Error::io(path, source))?;
    let mut rows = Vec::new();
    for (line_index, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let cells: Vec<_> = line.splitn(4, '|').collect();
        if cells.len() != 4 {
            return Err(Error::Message(format!(
                "{}:{} expected four pipe-separated columns",
                path.display(),
                line_index + 1
            )));
        }
        rows.push(ApiRow {
            symbol: cells[0].trim().to_owned(),
            status: cells[1].trim().to_owned(),
            surface: cells[2].trim().to_owned(),
            notes: cells[3].trim().to_owned(),
        });
    }
    Ok(rows)
}

fn validate_api_coverage(symbols: &BTreeSet<String>, rows: &[ApiRow]) -> Result<()> {
    let allowed = ["safe", "raw", "omitted", "deferred"];
    let mut row_symbols = BTreeSet::new();
    let mut errors = Vec::new();

    for row in rows {
        if !row_symbols.insert(row.symbol.clone()) {
            errors.push(format!("duplicate fixture row for `{}`", row.symbol));
        }
        if !symbols.contains(&row.symbol) {
            errors.push(format!(
                "fixture row has no vendored B2_API symbol: `{}`",
                row.symbol
            ));
        }
        if !allowed.contains(&row.status.as_str()) {
            errors.push(format!(
                "invalid status `{}` for `{}`",
                row.status, row.symbol
            ));
        }
        if row.surface.is_empty() || row.notes.is_empty() {
            errors.push(format!(
                "fixture row for `{}` needs surface and notes",
                row.symbol
            ));
        }
    }

    for symbol in symbols {
        if !row_symbols.contains(symbol) {
            errors.push(format!(
                "missing fixture row for vendored symbol `{symbol}`"
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::Message(errors.join("\n")))
    }
}

#[derive(Default)]
struct ApiCounts {
    total: usize,
    safe: usize,
    raw: usize,
    omitted: usize,
    deferred: usize,
}

impl ApiCounts {
    fn add(&mut self, status: &str) {
        self.total += 1;
        match status {
            "safe" => self.safe += 1,
            "raw" => self.raw += 1,
            "omitted" => self.omitted += 1,
            "deferred" => self.deferred += 1,
            _ => {}
        }
    }
}

fn api_counts(rows: &[ApiRow]) -> ApiCounts {
    let mut counts = ApiCounts::default();
    for row in rows {
        counts.add(&row.status);
    }
    counts
}

fn sample_parity(root: &Path, args: &[String]) -> Result<()> {
    let mode = match args {
        [arg] if arg == "--check" => SampleParityMode::Check,
        [arg] if arg == "--write" => SampleParityMode::Write,
        [] => SampleParityMode::Check,
        _ => {
            return Err(Error::Message(
                "sample-parity expects --check or --write".to_owned(),
            ));
        }
    };

    let samples = discover_upstream_samples(root)?;
    let matrix_path = root.join("docs/upstream-parity/box2d-sample-matrix.md");

    match mode {
        SampleParityMode::Write => {
            let existing_rows = if matrix_path.exists() {
                read_sample_matrix(&matrix_path)?
            } else {
                Vec::new()
            };
            write_sample_matrix(&matrix_path, &samples, &existing_rows)?;
            println!(
                "wrote {} upstream sample rows to {}",
                samples.len(),
                matrix_path.display()
            );
            Ok(())
        }
        SampleParityMode::Check => {
            let rows = read_sample_matrix(&matrix_path)?;
            validate_sample_matrix(root, &samples, &rows)?;
            println!(
                "sample parity ok: {} upstream samples covered by {} matrix rows ({})",
                samples.len(),
                rows.len(),
                sample_status_summary(&rows)
            );
            Ok(())
        }
    }
}

enum SampleParityMode {
    Check,
    Write,
}

fn sample_status_summary(rows: &[MatrixRow]) -> String {
    let mut counts = BTreeMap::<&str, usize>::new();
    for row in rows {
        *counts.entry(&row.status).or_default() += 1;
    }

    let mut summary = String::new();
    for (index, (status, count)) in counts.into_iter().enumerate() {
        if index > 0 {
            summary.push_str(", ");
        }
        let _ = write!(&mut summary, "{status}: {count}");
    }
    summary
}

fn discover_upstream_samples(root: &Path) -> Result<BTreeSet<Sample>> {
    let samples_dir = root.join("boxdd-sys/third-party/box2d/samples");
    let mut samples = BTreeSet::new();
    for entry in fs::read_dir(&samples_dir).map_err(|source| Error::io(&samples_dir, source))? {
        let entry = entry.map_err(|source| Error::io(&samples_dir, source))?;
        let path = entry.path();
        if !path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("sample_") && name.ends_with(".cpp"))
        {
            continue;
        }

        let content = fs::read_to_string(&path).map_err(|source| Error::io(&path, source))?;
        for (line_index, line) in content.lines().enumerate() {
            if !line.contains("RegisterSample(") && !line.contains("RegisterReplay(") {
                continue;
            }
            let strings = quoted_strings(line);
            if strings.len() < 2 {
                continue;
            }
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            samples.insert(Sample {
                category: strings[0].clone(),
                name: strings[1].clone(),
                source: format!("{}:{}", relative, line_index + 1),
            });
        }
    }
    if samples.is_empty() {
        return Err(Error::Message(format!(
            "no upstream samples found under {}",
            samples_dir.display()
        )));
    }
    Ok(samples)
}

fn quoted_strings(line: &str) -> Vec<String> {
    let mut strings = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' if in_string => {
                strings.push(current.clone());
                current.clear();
                in_string = false;
            }
            '"' => in_string = true,
            '\\' if in_string => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            _ if in_string => current.push(ch),
            _ => {}
        }
    }

    strings
}

fn write_sample_matrix(
    path: &Path,
    samples: &BTreeSet<Sample>,
    existing_rows: &[MatrixRow],
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::io(parent, source))?;
    }

    let existing_by_key: BTreeMap<_, _> = existing_rows
        .iter()
        .map(|row| {
            (
                (
                    row.category.as_str(),
                    row.name.as_str(),
                    row.source.as_str(),
                ),
                row,
            )
        })
        .collect();

    let mut output = String::new();
    output.push_str("# Box2D Sample Parity Matrix\n\n");
    output.push_str("This matrix maps every official Box2D sample registered in `boxdd-sys/third-party/box2d/samples/sample_*.cpp` to the Rust artifact that covers it.\n");
    output.push_str("Rows are validated by `cargo run -p xtask -- sample-parity --check`.\n\n");
    output.push_str("## Status Values\n\n");
    output.push_str("- `FaithfulPort` means the Rust artifact is intended to match the official sample behavior.\n");
    output.push_str("- `TeachingAdaptation` means the Rust artifact teaches the same API surface with Rust-specific simplification.\n");
    output.push_str("- `TestOnly` means the sample is represented by a regression or API test rather than a user-facing example.\n");
    output.push_str("- `Deferred` means the sample is intentionally not covered yet and must carry a rationale in the artifact column.\n");
    output.push_str("- `UpstreamReference` means the upstream sample is indexed for traceability but has no Rust port yet.\n\n");
    output.push_str("`UpstreamReference` is allowed only for benchmark rows. All non-benchmark rows must name a Rust artifact or an explicit deferral rationale.\n\n");
    output.push_str("## Matrix\n\n");
    output.push_str("| Category | Sample | Status | Artifact | Source |\n");
    output.push_str("|---|---|---|---|---|\n");
    for sample in samples {
        let key = (
            sample.category.as_str(),
            sample.name.as_str(),
            sample.source.as_str(),
        );
        let seeded_coverage;
        let (status, artifact) = if let Some(row) = existing_by_key
            .get(&key)
            .filter(|row| !is_unassigned_sample_row(row))
        {
            (row.status.as_str(), row.artifact.as_str())
        } else {
            seeded_coverage = sample_coverage(sample);
            (seeded_coverage.status, seeded_coverage.artifact.as_str())
        };
        output.push_str(&format!(
            "| `{}` | `{}` | `{}` | {} | `{}` |\n",
            escape_table_cell(&sample.category),
            escape_table_cell(&sample.name),
            escape_table_cell(status),
            escape_table_cell(artifact),
            escape_table_cell(&sample.source)
        ));
    }

    fs::write(path, output).map_err(|source| Error::io(path, source))
}

fn sample_coverage(sample: &Sample) -> SampleCoverage {
    let artifact = match sample.category.as_str() {
        "Benchmark" if sample.name == "Large Pyramid" => link_artifacts(&[
            "bevy_boxdd/examples/testbed_2d/scenes.rs",
            "boxdd/examples/pyramid.rs",
        ]),
        "Benchmark" => {
            return SampleCoverage {
                status: "UpstreamReference",
                artifact: "Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples.".to_owned(),
            };
        }
        "Bodies" => bodies_sample_artifact(&sample.name),
        "Character" => link_artifact("boxdd/examples/character_mover.rs"),
        "Collision" => collision_sample_artifact(&sample.name),
        "Continuous" => continuous_sample_artifact(&sample.name),
        "Determinism" => link_artifact("boxdd/examples/determinism.rs"),
        "Events" => events_sample_artifact(&sample.name),
        "Geometry" => link_artifact("boxdd/examples/convex_hull.rs"),
        "Issues" => link_artifact("boxdd/examples/issues.rs"),
        "Joints" => joints_sample_artifact(&sample.name),
        "Robustness" => link_artifact("boxdd/examples/robustness.rs"),
        "Shapes" => shapes_sample_artifact(&sample.name),
        "Stacking" => stacking_sample_artifact(&sample.name),
        "World" => link_artifact("boxdd/examples/world_basics.rs"),
        _ => {
            return SampleCoverage {
                status: "Deferred",
                artifact: format!(
                    "No Rust artifact has been assigned for the `{}` category yet.",
                    sample.category
                ),
            };
        }
    };

    SampleCoverage {
        status: "TeachingAdaptation",
        artifact,
    }
}

fn is_unassigned_sample_row(row: &MatrixRow) -> bool {
    row.status == "UpstreamReference"
        && row
            .artifact
            .contains("Upstream sample indexed; Rust port not assigned yet.")
}

fn link_artifact(path: &str) -> String {
    format!("[`{path}`]({path})")
}

fn link_artifacts(paths: &[&str]) -> String {
    paths
        .iter()
        .map(|path| link_artifact(path))
        .collect::<Vec<_>>()
        .join(", ")
}

fn with_bevy_testbed(path: &str) -> String {
    link_artifacts(&["bevy_boxdd/examples/testbed_2d/scenes.rs", path])
}

fn bodies_sample_artifact(name: &str) -> String {
    match name {
        "Body Type" | "Kinematic" => with_bevy_testbed("boxdd/examples/bodies.rs"),
        _ => link_artifact("boxdd/examples/bodies.rs"),
    }
}

fn collision_sample_artifact(name: &str) -> String {
    match name {
        "Ray Cast" => link_artifact("boxdd/examples/raycast.rs"),
        "Shape Cast" => link_artifact("boxdd/examples/shapecast.rs"),
        "Cast World" => link_artifact("boxdd/examples/query_casts.rs"),
        "Overlap World" => link_artifact("boxdd/examples/queries.rs"),
        "Dynamic Tree" => link_artifact("boxdd/examples/dynamic_tree.rs"),
        "Manifold" | "Smooth Manifold" => link_artifact("boxdd/tests/manifold_collision.rs"),
        "Shape Distance" => link_artifact("boxdd/tests/distance.rs"),
        "Time of Impact" => link_artifact("boxdd/examples/continuous_bullet.rs"),
        _ => link_artifact("boxdd/examples/collision_basics.rs"),
    }
}

fn continuous_sample_artifact(name: &str) -> String {
    match name {
        "Chain Drop" | "Chain Slide" | "Segment Slide" => {
            link_artifact("boxdd/examples/chain_walkway.rs")
        }
        "Speculative Fallback" | "Speculative Ghost" | "Speculative Sliver" => {
            link_artifact("boxdd/examples/robustness.rs")
        }
        "Skinny Box" => with_bevy_testbed("boxdd/examples/continuous_bullet.rs"),
        _ => link_artifact("boxdd/examples/continuous_bullet.rs"),
    }
}

fn events_sample_artifact(name: &str) -> String {
    match name {
        "Contact" => with_bevy_testbed("boxdd/examples/contacts.rs"),
        "Persistent Contact" => link_artifact("boxdd/examples/contacts.rs"),
        "Sensor Funnel" => with_bevy_testbed("boxdd/examples/sensors.rs"),
        "Foot Sensor" | "Sensor Bookend" | "Sensor Hits" | "Sensor Types" => {
            link_artifact("boxdd/examples/sensors.rs")
        }
        _ => link_artifact("boxdd/examples/events_summary.rs"),
    }
}

fn joints_sample_artifact(name: &str) -> String {
    match name {
        "Bridge" => with_bevy_testbed("boxdd/examples/bridge.rs"),
        "Cantilever" => link_artifact("boxdd/examples/bridge.rs"),
        "Driving" => link_artifact("boxdd/examples/car.rs"),
        "Doohickey" => link_artifact("boxdd/examples/doohickey.rs"),
        "Prismatic" | "Gear Lift" | "Scissor Lift" => {
            link_artifact("boxdd/examples/prismatic_elevator.rs")
        }
        "Distance Joint" => with_bevy_testbed("boxdd/examples/joints.rs"),
        "Revolute" => with_bevy_testbed("boxdd/examples/revolute_motor.rs"),
        "Wheel" => link_artifact("boxdd/examples/prismatic_wheel.rs"),
        _ => link_artifact("boxdd/examples/joints.rs"),
    }
}

fn shapes_sample_artifact(name: &str) -> String {
    match name {
        "Chain Link" | "Chain Shape" => link_artifact("boxdd/examples/chain_walkway.rs"),
        "Filter" => with_bevy_testbed("boxdd/tests/world_callbacks.rs"),
        "Custom Filter" => link_artifact("boxdd/tests/world_callbacks.rs"),
        "Modify Geometry" => link_artifact("boxdd/examples/shapes_variety.rs"),
        "Friction" | "Restitution" => with_bevy_testbed("boxdd/examples/shapes_variety.rs"),
        "Tangent Speed" => link_artifact("boxdd/examples/contacts.rs"),
        _ => link_artifact("boxdd/examples/shapes_variety.rs"),
    }
}

fn stacking_sample_artifact(name: &str) -> String {
    match name {
        "Tilted Stack" => with_bevy_testbed("boxdd/examples/stacking.rs"),
        "Vertical Stack" => link_artifact("boxdd/examples/stacking.rs"),
        "Single Box" => with_bevy_testbed("boxdd/examples/basic.rs"),
        "Circle Stack" => with_bevy_testbed("boxdd/examples/pyramid.rs"),
        _ => link_artifact("boxdd/examples/pyramid.rs"),
    }
}

fn read_sample_matrix(path: &Path) -> Result<Vec<MatrixRow>> {
    let content = fs::read_to_string(path).map_err(|source| Error::io(path, source))?;
    let mut rows = Vec::new();
    let mut in_matrix = false;

    for line in content.lines() {
        if line.trim() == "## Matrix" {
            in_matrix = true;
            continue;
        }
        if !in_matrix || !line.starts_with('|') {
            continue;
        }
        if line.contains("|---") || line.contains("| Category ") {
            continue;
        }
        let cells: Vec<String> = line
            .trim_matches('|')
            .split('|')
            .map(|cell| strip_code_ticks(cell.trim()).to_owned())
            .collect();
        if cells.len() < 5 {
            continue;
        }
        rows.push(MatrixRow {
            category: cells[0].clone(),
            name: cells[1].clone(),
            status: cells[2].clone(),
            artifact: cells[3].clone(),
            source: cells[4].clone(),
        });
    }

    if rows.is_empty() {
        return Err(Error::Message(format!(
            "no matrix rows found in {}",
            path.display()
        )));
    }

    Ok(rows)
}

fn validate_sample_matrix(
    root: &Path,
    samples: &BTreeSet<Sample>,
    rows: &[MatrixRow],
) -> Result<()> {
    let allowed_statuses = [
        "FaithfulPort",
        "TeachingAdaptation",
        "TestOnly",
        "Deferred",
        "UpstreamReference",
    ];
    let upstream_keys: BTreeSet<_> = samples
        .iter()
        .map(|sample| {
            (
                sample.category.as_str(),
                sample.name.as_str(),
                sample.source.as_str(),
            )
        })
        .collect();
    let mut row_keys = BTreeSet::new();
    let mut errors = Vec::new();

    for row in rows {
        let key = (
            row.category.as_str(),
            row.name.as_str(),
            row.source.as_str(),
        );
        if !row_keys.insert((row.category.clone(), row.name.clone(), row.source.clone())) {
            errors.push(format!(
                "duplicate matrix row for `{}` / `{}` at {}",
                row.category, row.name, row.source
            ));
        }
        if !upstream_keys.contains(&key) {
            errors.push(format!(
                "matrix row has no upstream sample: `{}` / `{}` at {}",
                row.category, row.name, row.source
            ));
        }
        if !allowed_statuses.contains(&row.status.as_str()) {
            errors.push(format!(
                "invalid status `{}` for `{}` / `{}`",
                row.status, row.category, row.name
            ));
        }
        if row.source.is_empty() || !row.source.contains("sample_") {
            errors.push(format!(
                "missing upstream source for `{}` / `{}`",
                row.category, row.name
            ));
        }
        if row.status == "UpstreamReference" && row.category != "Benchmark" {
            errors.push(format!(
                "`{}` / `{}` must map to a Rust artifact or Deferred rationale; UpstreamReference is reserved for Benchmark rows",
                row.category, row.name
            ));
        }
        if matches!(
            row.status.as_str(),
            "FaithfulPort" | "TeachingAdaptation" | "TestOnly"
        ) {
            let artifacts = artifact_paths(&row.artifact);
            if artifacts.is_empty() {
                errors.push(format!(
                    "{} row for `{}` / `{}` needs at least one Rust artifact",
                    row.status, row.category, row.name
                ));
            }
            for artifact in &artifacts {
                let artifact_path = root.join(artifact);
                if !artifact_path.exists() {
                    errors.push(format!(
                        "artifact `{}` for `{}` / `{}` does not exist",
                        artifact, row.category, row.name
                    ));
                }
            }
            if row.status == "TestOnly"
                && !artifacts.iter().any(|artifact| {
                    artifact.starts_with("boxdd/tests/")
                        || artifact.starts_with("boxdd-sys/tests/")
                        || artifact.starts_with("bevy_boxdd/tests/")
                })
            {
                errors.push(format!(
                    "TestOnly row for `{}` / `{}` must name a tests/ artifact",
                    row.category, row.name
                ));
            }
        }
        if row.status == "Deferred" && !has_deferred_rationale(&row.artifact) {
            errors.push(format!(
                "deferred row for `{}` / `{}` needs a specific rationale",
                row.category, row.name
            ));
        }
    }

    for sample in samples {
        if !row_keys.contains(&(
            sample.category.clone(),
            sample.name.clone(),
            sample.source.clone(),
        )) {
            errors.push(format!(
                "missing matrix row for upstream sample `{}` / `{}` from {}",
                sample.category, sample.name, sample.source
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::Message(errors.join("\n")))
    }
}

fn provider_smoke_app(root: &Path) -> Result<()> {
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

fn provider_smoke(root: &Path) -> Result<()> {
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

fn build_pages_wasm(root: &Path) -> Result<()> {
    generate_pages(root)?;
    let bevy_artifacts = build_bevy_web_app(root)?;
    let out_dir = provider_smoke_dir(root);
    let exports = write_exports_json(&out_dir, &bevy_artifacts.imports)?;
    let provider = build_box2d_provider(root, &out_dir, &exports)?;
    let provider_wasm = provider.with_extension("wasm");
    ensure_file(&provider, "Box2D provider module")?;
    ensure_file(&provider_wasm, "Box2D provider wasm")?;

    let generated = pages_wasm_generated_dir(root);
    replace_dir_under(&generated, &root.join("docs/pages"))?;
    copy_file(&provider, &generated.join("box2d-sys-v0.js"))?;
    copy_file(&provider_wasm, &generated.join("box2d-sys-v0.wasm"))?;
    copy_bevy_web_artifacts(root, &bevy_artifacts)?;

    println!(
        "pages wasm assets ready: {} and {} ({} Bevy imports)",
        generated.display(),
        pages_bevy_generated_dir(root).display(),
        bevy_artifacts.imports.len()
    );
    Ok(())
}

fn provider_smoke_dir(root: &Path) -> PathBuf {
    root.join("target").join("boxdd-provider-smoke")
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

fn build_provider_smoke_app(root: &Path) -> Result<PathBuf> {
    let profile = BuildProfile::from_env();
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

fn add_wasm_app_link_args(command: &mut Command, export_groups: &[&[&str]]) {
    command.arg("--").arg("-C").arg("link-arg=--import-memory");
    for export in export_groups.iter().flat_map(|exports| exports.iter()) {
        command.arg("-C").arg(format!("link-arg=--export={export}"));
    }
}

fn build_bevy_web_app(root: &Path) -> Result<BevyWebArtifacts> {
    ensure_runnable_tool(
        "wasm-bindgen",
        "--version",
        "wasm-bindgen-cli is required for Bevy Web examples",
    )?;

    let out_dir = root.join("target").join("boxdd-bevy-testbed-web");
    replace_dir_under(&out_dir, &root.join("target"))?;

    let mut command = Command::new("cargo");
    command
        .arg("rustc")
        .arg("-p")
        .arg("bevy_boxdd")
        .arg("--example")
        .arg(BEVY_WEB_EXAMPLE)
        .arg("--target")
        .arg(WASM_TARGET)
        .arg("--release")
        .env("BOXDD_SYS_WASM_MODE", "provider");
    add_wasm_app_link_args(&mut command, &[]);
    run_command(&mut command, "build Bevy testbed wasm")?;

    let wasm = root
        .join("target")
        .join(WASM_TARGET)
        .join("release")
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

    patch_bevy_bindgen_imports(&out_dir.join(BEVY_WEB_JS))?;
    let bevy_wasm = out_dir.join(BEVY_WEB_WASM);
    let imports = collect_provider_imports(&bevy_wasm)?;
    write_browser_provider_shim(&out_dir, &imports)?;

    Ok(BevyWebArtifacts { out_dir, imports })
}

fn patch_bevy_bindgen_imports(js: &Path) -> Result<()> {
    let source = fs::read_to_string(js).map_err(|source| Error::io(js, source))?;
    let patched_imports = source.replace(
        &format!("from \"{PROVIDER_MODULE}\""),
        &format!("from \"./{BEVY_PROVIDER_SHIM}\""),
    );
    if patched_imports == source {
        return Err(Error::Message(format!(
            "wasm-bindgen output does not import {PROVIDER_MODULE}: {}",
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
    fs::write(js, patched).map_err(|source| Error::io(js, source))
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

fn collect_provider_imports(wasm: &Path) -> Result<Vec<String>> {
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

fn write_exports_json(out_dir: &Path, imports: &[String]) -> Result<PathBuf> {
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

fn build_box2d_provider(root: &Path, out_dir: &Path, exports_json: &Path) -> Result<PathBuf> {
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

fn run_command(command: &mut Command, label: &str) -> Result<()> {
    let status = command
        .status()
        .map_err(|source| Error::io(label, source))?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::Message(format!(
            "{label} failed with status {status}"
        )))
    }
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

fn replace_dir_under(dir: &Path, allowed_root: &Path) -> Result<()> {
    fs::create_dir_all(allowed_root).map_err(|source| Error::io(allowed_root, source))?;
    if dir.exists() {
        let canonical_dir = dir
            .canonicalize()
            .map_err(|source| Error::io(dir, source))?;
        let canonical_root = allowed_root
            .canonicalize()
            .map_err(|source| Error::io(allowed_root, source))?;
        if !canonical_dir.starts_with(&canonical_root) {
            return Err(Error::Message(format!(
                "refusing to remove directory outside {}: {}",
                canonical_root.display(),
                canonical_dir.display()
            )));
        }
        fs::remove_dir_all(&canonical_dir).map_err(|source| Error::io(&canonical_dir, source))?;
    }
    fs::create_dir_all(dir).map_err(|source| Error::io(dir, source))
}

fn copy_file(from: &Path, to: &Path) -> Result<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::io(parent, source))?;
    }
    fs::copy(from, to).map_err(|source| Error::io(to, source))?;
    Ok(())
}

fn ensure_file(path: &Path, label: &str) -> Result<PathBuf> {
    if path.is_file() {
        Ok(path.to_path_buf())
    } else {
        Err(Error::Message(format!(
            "{label} not found: {}",
            path.display()
        )))
    }
}

fn ensure_runnable_tool(tool: &str, version_arg: &str, message: &str) -> Result<()> {
    if runnable_tool(tool, version_arg).is_some() {
        Ok(())
    } else {
        Err(Error::Message(message.to_owned()))
    }
}

fn runnable_tool(tool: &str, version_arg: &str) -> Option<PathBuf> {
    Command::new(tool)
        .arg(version_arg)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|_| PathBuf::from(tool))
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

fn generate_pages(root: &Path) -> Result<()> {
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

function setStatus(state, title, detail) {
  statusPanel.dataset.state = state;
  statusPanel.replaceChildren();

  const titleNode = document.createElement("strong");
  titleNode.textContent = title;
  const detailNode = document.createElement("span");
  detailNode.textContent = detail;
  statusPanel.append(titleNode, detailNode);
}

function generatedUrl(path) {
  return new URL(path, import.meta.url);
}

async function main() {
  const providerGenerated = new URL("../wasm/generated/", import.meta.url);
  const [
    { default: createProvider },
    { default: initBevyTestbed },
    { setBox2dProvider, setBoxddAppExports },
  ] =
    await Promise.all([
      import(new URL("box2d-sys-v0.js", providerGenerated).href),
      import(generatedUrl("generated/bevy_boxdd_testbed.js").href),
      import(generatedUrl("generated/box2d-provider-shim.js").href),
    ]);
  const memory = new WebAssembly.Memory({ initial: 4096, maximum: 8192 });

  setStatus("loading", "Loading Box2D provider", `Preparing the shared Box2D C provider for ${sceneName}.`);
  const provider = await createProvider({
    wasmMemory: memory,
    locateFile: (path) => new URL(path, providerGenerated).href,
    print: (text) => console.log(`[box2d-sys-v0] ${text}`),
    printErr: (text) => console.warn(`[box2d-sys-v0] ${text}`),
  });

  if (provider.wasmMemory && provider.wasmMemory !== memory) {
    throw new Error("Box2D provider did not use the shared WebAssembly.Memory");
  }

  setBox2dProvider(provider);
  setStatus("loading", `Loading ${sceneName}`, "Starting the Rust Bevy + egui wasm module.");

  const bevyExports = await initBevyTestbed({
    module_or_path: generatedUrl("generated/bevy_boxdd_testbed_bg.wasm"),
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

fn validate_pages(root: &Path) -> Result<()> {
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
        for asset in ["box2d-sys-v0.js", "box2d-sys-v0.wasm"] {
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

fn escape_table_cell(value: &str) -> String {
    value.replace('|', "\\|")
}

fn strip_code_ticks(value: &str) -> &str {
    value.trim().trim_matches('`').trim()
}

fn strip_markdown_link_target(value: &str) -> &str {
    if let Some(start) = value.find("](")
        && let Some(end) = value[start + 2..].find(')')
    {
        return &value[start + 2..start + 2 + end];
    }
    strip_code_ticks(value)
}

fn artifact_paths(value: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let mut rest = value;
    while let Some(start) = rest.find("](") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find(')') else {
            break;
        };
        push_artifact_path(rest[..end].trim(), &mut paths);
        rest = &rest[end + 1..];
    }

    if paths.is_empty() {
        for part in value.split([',', ';']) {
            push_artifact_path(strip_markdown_link_target(part).trim(), &mut paths);
        }
    }

    paths
}

fn push_artifact_path(value: &str, paths: &mut Vec<String>) {
    let value = strip_code_ticks(value);
    if value.is_empty() || value.contains(' ') || value.contains(':') {
        return;
    }
    if value.ends_with(".rs") || value.ends_with(".md") || value.ends_with(".html") {
        paths.push(value.replace('\\', "/"));
    }
}

fn has_deferred_rationale(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value.len() >= 24
        && !value.eq_ignore_ascii_case("tbd")
        && !value.eq_ignore_ascii_case("todo")
        && !value.eq_ignore_ascii_case("deferred")
}

#[allow(dead_code)]
fn group_by_category(samples: &BTreeSet<Sample>) -> BTreeMap<&str, Vec<&Sample>> {
    let mut grouped: BTreeMap<&str, Vec<&Sample>> = BTreeMap::new();
    for sample in samples {
        grouped.entry(&sample.category).or_default().push(sample);
    }
    grouped
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn sample(category: &str, name: &str, source: &str) -> Sample {
        Sample {
            category: category.to_owned(),
            name: name.to_owned(),
            source: source.to_owned(),
        }
    }

    fn row(category: &str, name: &str, status: &str, artifact: &str, source: &str) -> MatrixRow {
        MatrixRow {
            category: category.to_owned(),
            name: name.to_owned(),
            status: status.to_owned(),
            artifact: artifact.to_owned(),
            source: source.to_owned(),
        }
    }

    fn unique_test_root(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        env::temp_dir().join(format!("boxdd-xtask-{name}-{}-{nonce}", std::process::id()))
    }

    #[test]
    fn non_benchmark_upstream_reference_fails() {
        let root = unique_test_root("strict-reference");
        fs::create_dir_all(&root).expect("test root should be created");
        let source = "boxdd-sys/third-party/box2d/samples/sample_collision.cpp:1201";
        let mut samples = BTreeSet::new();
        samples.insert(sample("Collision", "Ray Cast", source));
        let rows = [row(
            "Collision",
            "Ray Cast",
            "UpstreamReference",
            "Upstream sample indexed; Rust port not assigned yet.",
            source,
        )];

        let error = validate_sample_matrix(&root, &samples, &rows)
            .expect_err("non-benchmark upstream reference must fail");
        assert!(error.to_string().contains("UpstreamReference is reserved"));
        fs::remove_dir_all(&root).expect("test root should be cleaned up");
    }

    #[test]
    fn mapped_artifacts_allow_multiple_paths() {
        let root = unique_test_root("multiple-artifacts");
        let example = root.join("boxdd/examples/raycast.rs");
        let test = root.join("boxdd/tests/world_and_queries.rs");
        fs::create_dir_all(example.parent().expect("example parent")).expect("example parent");
        fs::create_dir_all(test.parent().expect("test parent")).expect("test parent");
        fs::write(&example, "").expect("example should be written");
        fs::write(&test, "").expect("test should be written");

        let source = "boxdd-sys/third-party/box2d/samples/sample_collision.cpp:1201";
        let mut samples = BTreeSet::new();
        samples.insert(sample("Collision", "Ray Cast", source));
        let rows = [row(
            "Collision",
            "Ray Cast",
            "TeachingAdaptation",
            "[`boxdd/examples/raycast.rs`](boxdd/examples/raycast.rs), `boxdd/tests/world_and_queries.rs`",
            source,
        )];

        validate_sample_matrix(&root, &samples, &rows).expect("all mapped artifacts exist");
        fs::remove_dir_all(&root).expect("test root should be cleaned up");
    }

    #[test]
    fn write_preserves_existing_manual_mapping() {
        let root = unique_test_root("preserve-write");
        let matrix = root.join("docs/upstream-parity/box2d-sample-matrix.md");
        let source = "boxdd-sys/third-party/box2d/samples/sample_collision.cpp:1201";
        let mut samples = BTreeSet::new();
        samples.insert(sample("Collision", "Ray Cast", source));
        let rows = [row(
            "Collision",
            "Ray Cast",
            "TeachingAdaptation",
            "`boxdd/examples/raycast.rs`",
            source,
        )];

        write_sample_matrix(&matrix, &samples, &rows).expect("matrix should be written");
        let content = fs::read_to_string(&matrix).expect("matrix should be readable");
        assert!(content.contains("`TeachingAdaptation`"));
        assert!(content.contains("`boxdd/examples/raycast.rs`"));
        fs::remove_dir_all(&root).expect("test root should be cleaned up");
    }

    #[test]
    fn write_replaces_default_unassigned_mapping() {
        let root = unique_test_root("replace-default");
        let matrix = root.join("docs/upstream-parity/box2d-sample-matrix.md");
        let source = "boxdd-sys/third-party/box2d/samples/sample_collision.cpp:1201";
        let mut samples = BTreeSet::new();
        samples.insert(sample("Collision", "Ray Cast", source));
        let rows = [row(
            "Collision",
            "Ray Cast",
            "UpstreamReference",
            "Upstream sample indexed; Rust port not assigned yet.",
            source,
        )];

        write_sample_matrix(&matrix, &samples, &rows).expect("matrix should be written");
        let content = fs::read_to_string(&matrix).expect("matrix should be readable");
        assert!(content.contains("`TeachingAdaptation`"));
        assert!(content.contains("boxdd/examples/raycast.rs"));
        fs::remove_dir_all(&root).expect("test root should be cleaned up");
    }
}
