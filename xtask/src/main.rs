use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs, io,
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
            let (status, notes) = if intentionally_omitted_symbol(symbol) {
                (
                    "omitted",
                    "Intentionally omitted from the safe API; use upstream diagnostics tooling when needed.",
                )
            } else if safe_source.contains(symbol) {
                ("safe", "Referenced by the Rust safe layer.")
            } else {
                (
                    "raw",
                    "Available through boxdd_sys::ffi; safe wrapper not assigned yet.",
                )
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

fn intentionally_omitted_symbol(symbol: &str) -> bool {
    matches!(
        symbol,
        "b2World_DumpMemoryStats" | "b2World_RebuildStaticTree"
    )
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
            write_sample_matrix(&matrix_path, &samples)?;
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
                "sample parity ok: {} upstream samples covered by {} matrix rows",
                samples.len(),
                rows.len()
            );
            Ok(())
        }
    }
}

enum SampleParityMode {
    Check,
    Write,
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

fn write_sample_matrix(path: &Path, samples: &BTreeSet<Sample>) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::io(parent, source))?;
    }

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
    output.push_str("## Matrix\n\n");
    output.push_str("| Category | Sample | Status | Artifact | Source |\n");
    output.push_str("|---|---|---|---|---|\n");
    for sample in samples {
        output.push_str(&format!(
            "| `{}` | `{}` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `{}` |\n",
            escape_table_cell(&sample.category),
            escape_table_cell(&sample.name),
            escape_table_cell(&sample.source)
        ));
    }

    fs::write(path, output).map_err(|source| Error::io(path, source))
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
        if matches!(
            row.status.as_str(),
            "FaithfulPort" | "TeachingAdaptation" | "TestOnly"
        ) {
            let artifact = strip_markdown_link_target(&row.artifact);
            let artifact_path = root.join(artifact);
            if !artifact_path.exists() {
                errors.push(format!(
                    "artifact `{}` for `{}` / `{}` does not exist",
                    artifact, row.category, row.name
                ));
            }
        }
        if row.status == "Deferred" && row.artifact.trim().is_empty() {
            errors.push(format!(
                "deferred row for `{}` / `{}` needs a rationale",
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

#[allow(dead_code)]
fn group_by_category(samples: &BTreeSet<Sample>) -> BTreeMap<&str, Vec<&Sample>> {
    let mut grouped: BTreeMap<&str, Vec<&Sample>> = BTreeMap::new();
    for sample in samples {
        grouped.entry(&sample.category).or_default().push(sample);
    }
    grouped
}
