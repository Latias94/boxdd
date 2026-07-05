use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug)]
struct ApiRow {
    symbol: String,
    status: String,
    surface: String,
    notes: String,
}

#[test]
fn vendored_b2_api_symbols_are_classified() {
    let root = workspace_root();
    let symbols = discover_b2_api_symbols(&root);
    let rows = read_fixture(&root.join("boxdd/tests/fixtures/api_coverage_symbols.txt"));
    let allowed = ["safe", "raw", "omitted", "deferred"];
    let mut row_symbols = BTreeSet::new();
    let mut errors = Vec::new();

    for row in &rows {
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

    for symbol in &symbols {
        if !row_symbols.contains(symbol) {
            errors.push(format!(
                "missing fixture row for vendored symbol `{symbol}`"
            ));
        }
    }

    assert!(
        errors.is_empty(),
        "API coverage fixture drifted:\n{}",
        errors.join("\n")
    );
}

#[test]
fn api_coverage_document_counts_match_fixture() {
    let root = workspace_root();
    let rows = read_fixture(&root.join("boxdd/tests/fixtures/api_coverage_symbols.txt"));
    let expected = Counts::from_rows(&rows);
    let docs =
        fs::read_to_string(root.join("docs/api-coverage.md")).expect("read docs/api-coverage.md");
    let actual = Counts::from_marker(&docs).expect("api-coverage marker");

    assert_eq!(actual, expected);
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("boxdd crate parent")
        .to_path_buf()
}

fn discover_b2_api_symbols(root: &Path) -> BTreeSet<String> {
    let include_dir = root.join("boxdd-sys/third-party/box2d/include/box2d");
    let mut symbols = BTreeSet::new();
    for entry in fs::read_dir(&include_dir).expect("read Box2D include dir") {
        let path = entry.expect("include entry").path();
        if path.extension().is_none_or(|ext| ext != "h") {
            continue;
        }
        let content = fs::read_to_string(&path).unwrap_or_else(|err| {
            panic!("read {}: {err}", path.display());
        });
        let mut decl = String::new();
        for line in content.lines() {
            if line.contains("B2_API") || !decl.is_empty() {
                decl.push(' ');
                decl.push_str(line.trim());
                if line.contains(';') {
                    if let Some(symbol) = parse_symbol(&decl) {
                        symbols.insert(symbol);
                    }
                    decl.clear();
                }
            }
        }
    }
    assert!(!symbols.is_empty(), "no B2_API symbols found");
    symbols
}

fn parse_symbol(decl: &str) -> Option<String> {
    let before_paren = decl.split('(').next()?;
    let name = before_paren
        .split_whitespace()
        .last()?
        .trim_start_matches('*')
        .trim();
    name.starts_with("b2").then(|| name.to_owned())
}

fn read_fixture(path: &Path) -> Vec<ApiRow> {
    let content = fs::read_to_string(path).unwrap_or_else(|err| {
        panic!("read {}: {err}", path.display());
    });
    content
        .lines()
        .enumerate()
        .filter_map(|(line_index, line)| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let cells: Vec<_> = line.splitn(4, '|').collect();
            assert_eq!(
                cells.len(),
                4,
                "{}:{} must have four pipe-separated columns",
                path.display(),
                line_index + 1
            );
            Some(ApiRow {
                symbol: cells[0].trim().to_owned(),
                status: cells[1].trim().to_owned(),
                surface: cells[2].trim().to_owned(),
                notes: cells[3].trim().to_owned(),
            })
        })
        .collect()
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct Counts {
    total: usize,
    safe: usize,
    raw: usize,
    omitted: usize,
    deferred: usize,
}

impl Counts {
    fn from_rows(rows: &[ApiRow]) -> Self {
        let mut counts = Self {
            total: rows.len(),
            safe: 0,
            raw: 0,
            omitted: 0,
            deferred: 0,
        };
        for row in rows {
            match row.status.as_str() {
                "safe" => counts.safe += 1,
                "raw" => counts.raw += 1,
                "omitted" => counts.omitted += 1,
                "deferred" => counts.deferred += 1,
                _ => {}
            }
        }
        counts
    }

    fn from_marker(docs: &str) -> Option<Self> {
        let marker = docs
            .lines()
            .find(|line| line.trim_start().starts_with("<!-- api-coverage:"))?;
        Some(Self {
            total: marker_value(marker, "total")?,
            safe: marker_value(marker, "safe")?,
            raw: marker_value(marker, "raw")?,
            omitted: marker_value(marker, "omitted")?,
            deferred: marker_value(marker, "deferred")?,
        })
    }
}

fn marker_value(marker: &str, key: &str) -> Option<usize> {
    marker
        .split_whitespace()
        .find_map(|token| token.strip_prefix(&format!("{key}=")))
        .and_then(|value| value.trim_end_matches("-->").parse().ok())
}
