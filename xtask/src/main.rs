use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    fmt::Write as _,
    fs, io,
    path::{Path, PathBuf},
};

type Result<T> = std::result::Result<T, Error>;

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
  cargo run -p xtask -- validate-pages

Commands:
  api-coverage  Validate or regenerate docs/api-coverage.md and its fixture
  sample-parity  Validate or regenerate docs/upstream-parity/box2d-sample-matrix.md
  validate-pages Validate local links in docs/pages/**/*.html
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
        if !path.extension().is_some_and(|ext| ext == "h") {
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
        "Benchmark" => {
            return SampleCoverage {
                status: "UpstreamReference",
                artifact: "Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples.".to_owned(),
            };
        }
        "Bodies" => link_artifact("boxdd/examples/bodies.rs"),
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
        _ => link_artifact("boxdd/examples/continuous_bullet.rs"),
    }
}

fn events_sample_artifact(name: &str) -> String {
    match name {
        "Contact" | "Persistent Contact" => link_artifact("boxdd/examples/contacts.rs"),
        "Foot Sensor" | "Sensor Bookend" | "Sensor Funnel" | "Sensor Hits" | "Sensor Types" => {
            link_artifact("boxdd/examples/sensors.rs")
        }
        _ => link_artifact("boxdd/examples/events_summary.rs"),
    }
}

fn joints_sample_artifact(name: &str) -> String {
    match name {
        "Bridge" | "Cantilever" => link_artifact("boxdd/examples/bridge.rs"),
        "Driving" => link_artifact("boxdd/examples/car.rs"),
        "Doohickey" => link_artifact("boxdd/examples/doohickey.rs"),
        "Prismatic" | "Gear Lift" | "Scissor Lift" => {
            link_artifact("boxdd/examples/prismatic_elevator.rs")
        }
        "Revolute" => link_artifact("boxdd/examples/revolute_motor.rs"),
        "Wheel" => link_artifact("boxdd/examples/prismatic_wheel.rs"),
        _ => link_artifact("boxdd/examples/joints.rs"),
    }
}

fn shapes_sample_artifact(name: &str) -> String {
    match name {
        "Chain Link" | "Chain Shape" => link_artifact("boxdd/examples/chain_walkway.rs"),
        "Filter" | "Custom Filter" => link_artifact("boxdd/tests/world_callbacks.rs"),
        "Modify Geometry" => link_artifact("boxdd/examples/shapes_variety.rs"),
        "Tangent Speed" => link_artifact("boxdd/examples/contacts.rs"),
        _ => link_artifact("boxdd/examples/shapes_variety.rs"),
    }
}

fn stacking_sample_artifact(name: &str) -> String {
    match name {
        "Vertical Stack" | "Tilted Stack" => link_artifact("boxdd/examples/stacking.rs"),
        "Single Box" => link_artifact("boxdd/examples/basic.rs"),
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

fn validate_pages(root: &Path) -> Result<()> {
    let pages_dir = root.join("docs/pages");
    let html_files = collect_html_files(&pages_dir)?;
    if html_files.is_empty() {
        return Err(Error::Message(format!(
            "no html pages found under {}",
            pages_dir.display()
        )));
    }

    let mut errors = Vec::new();
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

    if errors.is_empty() {
        println!("pages ok: {} html files checked", html_files.len());
        Ok(())
    } else {
        Err(Error::Message(errors.join("\n")))
    }
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
    if let Some(start) = value.find("](") {
        if let Some(end) = value[start + 2..].find(')') {
            return &value[start + 2..start + 2 + end];
        }
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
