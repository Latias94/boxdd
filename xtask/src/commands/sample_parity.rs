use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs,
    path::Path,
};

use crate::{Error, Result};

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

pub(crate) fn run(root: &Path, args: &[String]) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        env,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

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
