use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::commands::upstream_sync::{ArtifactKind, Precision, UpstreamManifest};
use crate::config::read_toml;
use crate::paths::WorkspacePaths;
use crate::{Error, Result};

const INVENTORY_PATH: &str = "xtask/api-inventory.toml";
const INVENTORY_SCHEMA: u32 = 1;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ApiInventory {
    schema_version: u32,
    upstream_revision: String,
    #[serde(rename = "safe")]
    reviewed_safe: Vec<String>,
    exceptions: Vec<ApiException>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum Classification {
    Raw,
    Omitted,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ApiException {
    name: String,
    classification: Classification,
    rationale: String,
}

pub fn run(paths: &WorkspacePaths, args: &[String]) -> Result<()> {
    match args {
        [] => {}
        [flag] if flag == "--check" => {}
        _ => {
            return Err(Error::message(
                "api-inventory accepts no arguments or --check",
            ));
        }
    }

    let manifest = UpstreamManifest::load(paths)?;
    let inventory_path = paths.root().join(INVENTORY_PATH);
    let reviewed: ApiInventory = read_toml(&inventory_path)?;
    let summary = validate_inventory(paths, &manifest, &reviewed)?;
    println!(
        "API disposition inventory is current: {} functions ({} Safe disposition intents, {} raw, {} omitted); Rust implementation coverage is compiler- and test-owned",
        summary.total, summary.safe, summary.raw, summary.omitted
    );
    Ok(())
}

/// Returns the public C function set after validating headers against every bindings artifact.
///
/// This deliberately does not read the Safe/Raw/Omitted inventory. Provider ABI membership is a C
/// surface concern; Rust API disposition is validated separately by [`run`].
pub(super) fn validated_c_api_function_names(paths: &WorkspacePaths) -> Result<BTreeSet<String>> {
    let manifest = UpstreamManifest::load(paths)?;
    let header_names = exported_header_function_names(&paths.box2d_headers())?;
    let mut errors = Vec::new();
    validate_binding_function_sets(paths.root(), &manifest, &header_names, &mut errors)?;
    if errors.is_empty() {
        Ok(header_names)
    } else {
        Err(Error::message(format!(
            "public C API validation failed:\n- {}",
            errors.join("\n- ")
        )))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct InventorySummary {
    total: usize,
    safe: usize,
    raw: usize,
    omitted: usize,
}

fn validate_inventory(
    paths: &WorkspacePaths,
    manifest: &UpstreamManifest,
    reviewed: &ApiInventory,
) -> Result<InventorySummary> {
    let mut errors = Vec::new();
    if reviewed.schema_version != INVENTORY_SCHEMA {
        errors.push(format!(
            "API inventory schema {} is unsupported; expected {INVENTORY_SCHEMA}",
            reviewed.schema_version
        ));
    }
    if reviewed.upstream_revision != manifest.active_revision {
        errors.push(format!(
            "API inventory revision {} does not match upstream revision {}",
            reviewed.upstream_revision, manifest.active_revision
        ));
    }

    let header_names = exported_header_function_names(&paths.box2d_headers())?;

    let mut reviewed_names = BTreeSet::new();
    for name in &reviewed.reviewed_safe {
        if !reviewed_names.insert(name.as_str()) {
            errors.push(format!(
                "API function `{name}` is classified more than once"
            ));
        }
    }

    let mut raw = 0;
    let mut omitted = 0;
    for exception in &reviewed.exceptions {
        if !reviewed_names.insert(exception.name.as_str()) {
            errors.push(format!(
                "API function `{}` is classified more than once",
                exception.name
            ));
        }
        if exception.rationale.trim().is_empty() {
            errors.push(format!(
                "API exception `{}` must include a rationale",
                exception.name
            ));
        }
        match exception.classification {
            Classification::Raw => raw += 1,
            Classification::Omitted => omitted += 1,
        }
    }

    let classified_names = reviewed_names
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let missing = header_names
        .difference(&classified_names)
        .cloned()
        .collect::<Vec<_>>();
    let unknown = classified_names
        .difference(&header_names)
        .cloned()
        .collect::<Vec<_>>();
    if !missing.is_empty() || !unknown.is_empty() {
        errors.push(format!(
            "API inventory does not exactly cover the vendored public headers; unclassified={missing:?}, unknown={unknown:?}"
        ));
    }

    validate_binding_function_sets(paths.root(), manifest, &header_names, &mut errors)?;

    if !errors.is_empty() {
        return Err(Error::message(format!(
            "API inventory validation failed:\n- {}",
            errors.join("\n- ")
        )));
    }

    Ok(InventorySummary {
        total: header_names.len(),
        safe: reviewed.reviewed_safe.len(),
        raw,
        omitted,
    })
}

fn exported_header_function_names(headers: &Path) -> Result<BTreeSet<String>> {
    let mut header_paths = fs::read_dir(headers)
        .map_err(|error| Error::io(headers, error))?
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(|error| Error::io(headers, error))
        })
        .collect::<Result<Vec<PathBuf>>>()?;
    header_paths.retain(|path| path.extension().and_then(|value| value.to_str()) == Some("h"));
    header_paths.sort();

    let mut names = BTreeSet::new();
    for path in header_paths {
        let source = fs::read_to_string(&path).map_err(|error| Error::io(&path, error))?;
        let source = strip_c_comments(&source);
        for (line_index, line) in source.lines().enumerate() {
            let line = line.trim();
            if line.starts_with('#') || !line.contains("B2_API") {
                continue;
            }
            let name = exported_function_name(line).ok_or_else(|| {
                Error::message(format!(
                    "{}:{} has an unsupported B2_API declaration",
                    path.display(),
                    line_index + 1
                ))
            })?;
            if !names.insert(name.to_owned()) {
                return Err(Error::message(format!(
                    "public C function `{name}` is declared more than once"
                )));
            }
        }
    }
    if names.is_empty() {
        return Err(Error::message(
            "Box2D public headers export no B2_API functions",
        ));
    }
    Ok(names)
}

fn exported_function_name(line: &str) -> Option<&str> {
    let declaration = line.split_once("B2_API")?.1;
    let open = declaration.find('(')?;
    let name = declaration[..open]
        .rsplit(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .find(|part| !part.is_empty())?;
    (name.starts_with("b2")
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_'))
    .then_some(name)
}

fn strip_c_comments(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut characters = source.chars().peekable();
    let mut in_block_comment = false;
    while let Some(character) = characters.next() {
        if in_block_comment {
            if character == '*' && characters.peek() == Some(&'/') {
                characters.next();
                in_block_comment = false;
            } else if character == '\n' {
                output.push('\n');
            }
            continue;
        }
        if character == '/' && characters.peek() == Some(&'*') {
            characters.next();
            in_block_comment = true;
        } else if character == '/' && characters.peek() == Some(&'/') {
            characters.next();
            for character in characters.by_ref() {
                if character == '\n' {
                    output.push('\n');
                    break;
                }
            }
        } else {
            output.push(character);
        }
    }
    output
}

fn validate_binding_function_sets(
    root: &Path,
    manifest: &UpstreamManifest,
    header_names: &BTreeSet<String>,
    errors: &mut Vec<String>,
) -> Result<()> {
    let artifacts = manifest
        .artifacts
        .iter()
        .filter(|artifact| artifact.kind == ArtifactKind::Bindings)
        .collect::<Vec<_>>();
    if artifacts.is_empty() {
        errors.push("upstream manifest declares no bindings artifacts".to_owned());
        return Ok(());
    }

    let mut seen_paths = BTreeSet::new();
    for artifact in artifacts {
        if !seen_paths.insert(artifact.path.as_str()) {
            continue;
        }
        let binding_names = binding_function_names(&root.join(&artifact.path))?
            .into_iter()
            .map(|name| normalize_binding_name(artifact.precision, &name).to_owned())
            .collect::<BTreeSet<_>>();
        if binding_names != *header_names {
            let missing = header_names
                .difference(&binding_names)
                .cloned()
                .collect::<Vec<_>>();
            let extra = binding_names
                .difference(header_names)
                .cloned()
                .collect::<Vec<_>>();
            errors.push(format!(
                "bindings artifact `{}` differs from the vendored public C API; missing={missing:?}, extra={extra:?}",
                artifact.name
            ));
        }
    }
    Ok(())
}

fn binding_function_names(path: &Path) -> Result<BTreeSet<String>> {
    let source = fs::read_to_string(path).map_err(|error| Error::io(path, error))?;
    let mut names = BTreeSet::new();
    for line in source.lines() {
        let Some(declaration) = line.trim().strip_prefix("pub fn ") else {
            continue;
        };
        let Some((name, _)) = declaration.split_once('(') else {
            continue;
        };
        if name.starts_with("b2") && !names.insert(name.to_owned()) {
            return Err(Error::message(format!(
                "generated binding function `{name}` is declared more than once in {}",
                path.display()
            )));
        }
    }
    if names.is_empty() {
        return Err(Error::message(format!(
            "generated bindings {} declare no public Box2D functions",
            path.display()
        )));
    }
    Ok(names)
}

fn normalize_binding_name(precision: Option<Precision>, name: &str) -> &str {
    match (precision, name) {
        (Some(Precision::Double), "b2CreateWorldDoublePrecision") => "b2CreateWorld",
        _ => name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::upstream_sync::{
        ArtifactProducer, ArtifactProvider, ArtifactTarget, GeneratedArtifact, SourceInventory,
    };

    struct InventoryFixture {
        _root: tempfile::TempDir,
        paths: WorkspacePaths,
        manifest: UpstreamManifest,
        inventory: ApiInventory,
        single_bindings: PathBuf,
        double_bindings: PathBuf,
    }

    impl InventoryFixture {
        fn new() -> Self {
            let root = tempfile::tempdir().unwrap();
            let paths = WorkspacePaths::new(root.path());
            let headers = paths.box2d_headers();
            fs::create_dir_all(&headers).unwrap();
            fs::write(
                headers.join("box2d.h"),
                "B2_API void b2CreateWorld(void);\nB2_API void b2Step(void);\n",
            )
            .unwrap();

            let single_relative = "boxdd-sys/src/test-bindings-single.rs";
            let double_relative = "boxdd-sys/src/test-bindings-double.rs";
            let single_bindings = root.path().join(single_relative);
            let double_bindings = root.path().join(double_relative);
            fs::create_dir_all(single_bindings.parent().unwrap()).unwrap();
            fs::write(
                &single_bindings,
                "pub fn b2CreateWorld();\npub fn b2Step();\n",
            )
            .unwrap();
            fs::write(
                &double_bindings,
                "pub fn b2CreateWorldDoublePrecision();\npub fn b2Step();\n",
            )
            .unwrap();

            let artifact = |name: &str, path: &str, precision| GeneratedArtifact {
                name: name.to_owned(),
                kind: ArtifactKind::Bindings,
                path: path.to_owned(),
                precision: Some(precision),
                target: ArtifactTarget::Universal,
                provider: ArtifactProvider::Universal,
                producer: ArtifactProducer::Bindgen,
                content_blake3: "0".repeat(64),
            };
            let revision = "a".repeat(40);
            let manifest = UpstreamManifest {
                schema_version: 1,
                repository: "https://example.invalid/box2d".to_owned(),
                active_revision: revision.clone(),
                recording_revision: revision.clone(),
                recording_inputs: Vec::new(),
                artifacts: vec![
                    artifact("single", single_relative, Precision::Single),
                    artifact("double", double_relative, Precision::Double),
                ],
                source_inventory: SourceInventory {
                    tree: "b".repeat(40),
                    c_sources: Vec::new(),
                    private_headers: Vec::new(),
                    inline_files: Vec::new(),
                    public_headers: vec!["include/box2d/box2d.h".to_owned()],
                },
            };
            let inventory = ApiInventory {
                schema_version: INVENTORY_SCHEMA,
                upstream_revision: revision,
                reviewed_safe: vec!["b2CreateWorld".to_owned(), "b2Step".to_owned()],
                exceptions: Vec::new(),
            };
            Self {
                _root: root,
                paths,
                manifest,
                inventory,
                single_bindings,
                double_bindings,
            }
        }

        fn error(&self, inventory: &ApiInventory) -> String {
            validate_inventory(&self.paths, &self.manifest, inventory)
                .unwrap_err()
                .to_string()
        }
    }

    #[test]
    fn normalizes_the_upstream_double_precision_entry_point() {
        assert_eq!(
            normalize_binding_name(Some(Precision::Double), "b2CreateWorldDoublePrecision"),
            "b2CreateWorld"
        );
        assert_eq!(
            normalize_binding_name(Some(Precision::Single), "b2CreateWorld"),
            "b2CreateWorld"
        );
    }

    #[test]
    fn extracts_only_live_exported_header_functions() {
        let source = r#"
            #define B2_API extern
            // B2_API void b2Commented(void);
            /* B2_API void b2AlsoCommented(void); */
            B2_API b2WorldId b2CreateWorld(const b2WorldDef* def);
            B2_API b2Recording* b2CreateRecording(void);
        "#;
        let source = strip_c_comments(source);
        let names = source
            .lines()
            .filter_map(exported_function_name)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            names,
            BTreeSet::from(["b2CreateRecording", "b2CreateWorld"])
        );
    }

    #[test]
    fn extracts_bindgen_function_lines_without_parsing_rust_types() {
        let source = r#"
            pub type b2Callback = Option<unsafe extern "C" fn()>;
            pub fn b2GetVersion() -> b2Version;
            pub fn b2CreateWorld(
                def: *const b2WorldDef,
            ) -> b2WorldId;
        "#;
        let names = source
            .lines()
            .filter_map(|line| line.trim().strip_prefix("pub fn "))
            .filter_map(|line| line.split_once('(').map(|(name, _)| name))
            .collect::<BTreeSet<_>>();
        assert_eq!(names, BTreeSet::from(["b2CreateWorld", "b2GetVersion"]));
    }

    #[test]
    fn full_inventory_validation_fails_closed_for_disposition_mutations() {
        let fixture = InventoryFixture::new();
        assert_eq!(
            validate_inventory(&fixture.paths, &fixture.manifest, &fixture.inventory).unwrap(),
            InventorySummary {
                total: 2,
                safe: 2,
                raw: 0,
                omitted: 0,
            }
        );

        let mut mutated = fixture.inventory.clone();
        mutated.schema_version += 1;
        assert!(fixture.error(&mutated).contains("schema"));

        let mut mutated = fixture.inventory.clone();
        mutated.upstream_revision = "c".repeat(40);
        assert!(fixture.error(&mutated).contains("revision"));

        let mut mutated = fixture.inventory.clone();
        mutated.reviewed_safe.push("b2Step".to_owned());
        assert!(
            fixture
                .error(&mutated)
                .contains("classified more than once")
        );

        let mut mutated = fixture.inventory.clone();
        mutated.exceptions.push(ApiException {
            name: "b2Step".to_owned(),
            classification: Classification::Raw,
            rationale: "raw only".to_owned(),
        });
        assert!(
            fixture
                .error(&mutated)
                .contains("classified more than once")
        );

        let mut mutated = fixture.inventory.clone();
        mutated.reviewed_safe.pop();
        mutated.exceptions.extend([
            ApiException {
                name: "b2Step".to_owned(),
                classification: Classification::Raw,
                rationale: "raw only".to_owned(),
            },
            ApiException {
                name: "b2Step".to_owned(),
                classification: Classification::Omitted,
                rationale: "omitted instead".to_owned(),
            },
        ]);
        assert!(
            fixture
                .error(&mutated)
                .contains("classified more than once")
        );

        let mut mutated = fixture.inventory.clone();
        mutated.reviewed_safe.pop();
        mutated.exceptions.push(ApiException {
            name: "b2Step".to_owned(),
            classification: Classification::Omitted,
            rationale: "   ".to_owned(),
        });
        assert!(fixture.error(&mutated).contains("must include a rationale"));

        let mut mutated = fixture.inventory.clone();
        mutated.reviewed_safe.pop();
        assert!(fixture.error(&mutated).contains("unclassified"));

        let mut mutated = fixture.inventory.clone();
        mutated.reviewed_safe.push("b2Unknown".to_owned());
        assert!(fixture.error(&mutated).contains("unknown"));
    }

    #[test]
    fn full_inventory_validation_rejects_binding_set_drift() {
        let fixture = InventoryFixture::new();

        fs::write(&fixture.single_bindings, "pub fn b2CreateWorld();\n").unwrap();
        let error = fixture.error(&fixture.inventory);
        assert!(error.contains("missing=[\"b2Step\"]"), "{error}");

        fs::write(
            &fixture.single_bindings,
            "pub fn b2CreateWorld();\npub fn b2Step();\npub fn b2Unknown();\n",
        )
        .unwrap();
        let error = fixture.error(&fixture.inventory);
        assert!(error.contains("extra=[\"b2Unknown\"]"), "{error}");

        fs::write(
            &fixture.single_bindings,
            "pub fn b2CreateWorld();\npub fn b2Step();\n",
        )
        .unwrap();
        fs::write(&fixture.double_bindings, "pub fn b2Step();\n").unwrap();
        let error = fixture.error(&fixture.inventory);
        assert!(error.contains("missing=[\"b2CreateWorld\"]"), "{error}");

        fs::write(
            &fixture.double_bindings,
            "pub fn b2CreateWorldDoublePrecision();\npub fn b2Step();\npub fn b2Unknown();\n",
        )
        .unwrap();
        let error = fixture.error(&fixture.inventory);
        assert!(error.contains("extra=[\"b2Unknown\"]"), "{error}");

        let mut manifest = fixture.manifest.clone();
        manifest.artifacts.clear();
        let error = validate_inventory(&fixture.paths, &manifest, &fixture.inventory)
            .unwrap_err()
            .to_string();
        assert!(error.contains("no bindings artifacts"), "{error}");
    }
}
