use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use tempfile::TempDir;
use xtask::{
    Error,
    commands::upstream_sync::{
        ArtifactKind, ArtifactProducer, ArtifactProvider, ArtifactTarget, BindingRoute,
        GeneratedArtifact, ManagedArtifactWrite, Precision, RecordingInputIdentity, RustTarget,
        SourceInventory, UpstreamManifest, install_managed_artifact_writes,
    },
    config::{UPSTREAM_MANIFEST_SCHEMA, render_toml},
    paths::WorkspacePaths,
    recording_wire::REVIEWED_RECORDING_INPUT_PATHS,
};

const ACTIVE_REVISION: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const NEXT_REVISION: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[test]
fn public_upstream_check_never_writes_managed_or_git_state() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a workspace parent")
        .to_owned();
    let paths = WorkspacePaths::new(&root);
    let manifest = UpstreamManifest::load(&paths).expect("workspace upstream manifest");
    let mut managed_paths = manifest
        .artifacts
        .iter()
        .flat_map(|artifact| {
            [
                Some(artifact.path.as_str()),
                artifact.candidate_path.as_deref(),
            ]
            .into_iter()
            .flatten()
        })
        .map(|path| root.join(path))
        .collect::<Vec<_>>();
    managed_paths.push(paths.upstream_manifest());
    let managed_before = snapshot_files(&managed_paths);
    let index_path = git_path(&root, "index");
    let index_before = fs::read(&index_path).expect("root Git index");
    let submodule_index_path = git_path(&paths.box2d(), "index");
    let submodule_index_before =
        fs::read(&submodule_index_path).expect("Box2D submodule Git index");
    let gitlink_before = git(
        &root,
        &["ls-files", "--stage", "--", "boxdd-sys/third-party/box2d"],
    );
    let checkout_before = git(&paths.box2d(), &["rev-parse", "HEAD"]);
    let checkout_head_before = fs::read(paths.box2d().join(".git")).ok();

    let result = xtask::run_in(&paths, ["upstream-sync".to_owned(), "--check".to_owned()]);
    if manifest.artifact_digests_initialized {
        result.expect("an initialized repository must pass the public upstream check");
    } else {
        let error = result.expect_err("an uninitialized repository must fail closed");
        assert!(
            error
                .to_string()
                .contains("artifact digests are not initialized"),
            "unexpected uninitialized-check error: {error}"
        );
    }

    assert_eq!(snapshot_files(&managed_paths), managed_before);
    assert_eq!(
        fs::read(&index_path).expect("unchanged root Git index"),
        index_before
    );
    assert_eq!(
        fs::read(&submodule_index_path).expect("unchanged Box2D submodule Git index"),
        submodule_index_before
    );
    assert_eq!(
        git(
            &root,
            &["ls-files", "--stage", "--", "boxdd-sys/third-party/box2d"]
        ),
        gitlink_before
    );
    assert_eq!(git(&paths.box2d(), &["rev-parse", "HEAD"]), checkout_before);
    assert_eq!(
        fs::read(paths.box2d().join(".git")).ok(),
        checkout_head_before
    );
}

#[test]
fn managed_write_installs_manifest_last_then_rolls_every_file_back() {
    let fixture = TransactionFixture::create();
    let paths = WorkspacePaths::new(fixture.root.path());
    let wire_path = fixture.root.path().join("xtask/tests/wire.toml");
    let report_path = fixture.root.path().join("docs/report.md");
    let manifest_path = paths.upstream_manifest();
    let before = snapshot_files(&[
        wire_path.clone(),
        report_path.clone(),
        manifest_path.clone(),
    ]);
    let wire = format!("upstream_sha = \"{ACTIVE_REVISION}\"\nwire = \"new\"\n").into_bytes();
    let report = format!("Pinned active upstream: `{ACTIVE_REVISION}`.\nnew report\n").into_bytes();
    let writes = [
        ManagedArtifactWrite::active("recording-wire", wire.clone()),
        ManagedArtifactWrite::active("api-coverage-report", report.clone()),
    ];

    let error = install_managed_artifact_writes(&paths, &writes, || {
        assert_eq!(fs::read(&wire_path).expect("installed wire"), wire);
        assert_eq!(fs::read(&report_path).expect("installed report"), report);
        let installed = UpstreamManifest::load(&paths)?;
        assert_eq!(
            installed
                .artifact(ArtifactKind::RecordingWire)?
                .content_blake3,
            digest(&wire)
        );
        assert_eq!(
            installed
                .artifact(ArtifactKind::ApiCoverageReport)?
                .content_blake3,
            digest(&report)
        );
        Err(Error::message("injected integration terminal failure"))
    })
    .expect_err("terminal failure must roll back the complete file transaction");

    assert!(error.to_string().contains("integration terminal failure"));
    assert_eq!(
        snapshot_files(&[wire_path, report_path, manifest_path]),
        before
    );
}

struct TransactionFixture {
    root: TempDir,
}

impl TransactionFixture {
    fn create() -> Self {
        let root = tempfile::tempdir().expect("transaction fixture directory");
        command(root.path(), &["git", "init", "--quiet"]);
        write(
            root.path(),
            "boxdd/Cargo.toml",
            b"[package]\nname = \"boxdd-fixture\"\nversion = \"0.0.0\"\n\n[features]\ndefault = []\n",
        );

        let binding = binding_fixture();
        let api = format!("upstream_sha = \"{ACTIVE_REVISION}\"\n").into_bytes();
        let wire = format!("upstream_sha = \"{ACTIVE_REVISION}\"\n").into_bytes();
        let report = format!("Pinned active upstream: `{ACTIVE_REVISION}`.\n").into_bytes();
        let files = [
            ("boxdd-sys/src/bindings.rs", binding.as_slice()),
            ("boxdd/tests/api.toml", api.as_slice()),
            ("xtask/tests/wire.toml", wire.as_slice()),
            ("docs/report.md", report.as_slice()),
        ];
        for (path, content) in files {
            write(root.path(), path, content);
        }

        let manifest = UpstreamManifest {
            schema_version: UPSTREAM_MANIFEST_SCHEMA,
            repository: "https://github.com/erincatto/box2d.git".to_owned(),
            active_revision: ACTIVE_REVISION.to_owned(),
            next_revision: Some(NEXT_REVISION.to_owned()),
            recording_revision: ACTIVE_REVISION.to_owned(),
            artifact_digests_initialized: true,
            binding_routes: vec![BindingRoute {
                mode: Precision::Single,
                provider: ArtifactProvider::Source,
                artifact: "bindings-single".to_owned(),
                rust_target: RustTarget::X86_64UnknownLinuxGnu,
                rust_features: Vec::new(),
            }],
            next_binding_routes: Vec::new(),
            recording_inputs: REVIEWED_RECORDING_INPUT_PATHS
                .iter()
                .map(|path| RecordingInputIdentity {
                    path: (*path).to_owned(),
                    git_blob: "1".repeat(40),
                    blake3: "2".repeat(64),
                })
                .collect(),
            artifacts: vec![
                artifact(
                    "bindings-single",
                    ArtifactKind::Bindings,
                    "boxdd-sys/src/bindings.rs",
                    Some(Precision::Single),
                    ArtifactProducer::Bindgen,
                    &binding,
                ),
                artifact(
                    "api-contract",
                    ArtifactKind::ApiContract,
                    "boxdd/tests/api.toml",
                    None,
                    ArtifactProducer::Reviewed,
                    &api,
                ),
                artifact(
                    "recording-wire",
                    ArtifactKind::RecordingWire,
                    "xtask/tests/wire.toml",
                    None,
                    ArtifactProducer::ApiCoverage,
                    &wire,
                ),
                artifact(
                    "api-coverage-report",
                    ArtifactKind::ApiCoverageReport,
                    "docs/report.md",
                    None,
                    ArtifactProducer::ApiCoverage,
                    &report,
                ),
            ],
            next_artifacts: Vec::new(),
            source_inventory: SourceInventory {
                tree: "3".repeat(40),
                c_sources: vec!["src/a.c".to_owned()],
                private_headers: vec!["src/a.h".to_owned()],
                inline_files: vec!["src/a.inl".to_owned()],
                public_headers: vec!["include/box2d/a.h".to_owned()],
            },
            next_inventory: Some(SourceInventory {
                tree: "4".repeat(40),
                c_sources: vec!["src/a.c".to_owned()],
                private_headers: vec!["src/a.h".to_owned()],
                inline_files: vec!["src/a.inl".to_owned()],
                public_headers: vec!["include/box2d/a.h".to_owned()],
            }),
        };
        write(
            root.path(),
            "boxdd-sys/upstream.toml",
            render_toml(&manifest)
                .expect("render transaction manifest")
                .as_bytes(),
        );
        Self { root }
    }
}

fn artifact(
    name: &str,
    kind: ArtifactKind,
    path: &str,
    precision: Option<Precision>,
    producer: ArtifactProducer,
    content: &[u8],
) -> GeneratedArtifact {
    GeneratedArtifact {
        name: name.to_owned(),
        kind,
        path: path.to_owned(),
        precision,
        target: ArtifactTarget::Universal,
        provider: ArtifactProvider::Universal,
        producer,
        content_blake3: digest(content),
        candidate_path: None,
        candidate_blake3: None,
    }
}

fn binding_fixture() -> Vec<u8> {
    format!(
        "// AUTOGENERATED: pregenerated bindings for docs.rs/offline builds\n\
// boxdd-upstream-revision: {ACTIVE_REVISION}\n\
// boxdd-artifact-name: bindings-single\n\
// boxdd-artifact-precision: single\n\
// boxdd-artifact-target: universal\n\
// boxdd-artifact-provider: universal\n\
// boxdd-artifact-producer: bindgen\n\
// boxdd-artifact-rust-target: x86_64-unknown-linux-gnu\n\
// boxdd-wasi-libc-version: none\n\
// boxdd-wasi-headers-sha256: none\n\
// boxdd-freestanding-math-header-sha256: none\n\
// Authority: boxdd-sys/upstream.toml\n\
// Refresh with: cargo run -p xtask -- upstream-sync --refresh-routes\n\n\
pub const GENERATED: bool = true;\n"
    )
    .into_bytes()
}

fn snapshot_files(paths: &[PathBuf]) -> BTreeMap<PathBuf, Vec<u8>> {
    paths
        .iter()
        .map(|path| {
            (
                path.clone(),
                fs::read(path).unwrap_or_else(|error| {
                    panic!("could not snapshot {}: {error}", path.display())
                }),
            )
        })
        .collect()
}

fn write(root: &Path, relative: &str, content: &[u8]) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("fixture file parent"))
        .expect("fixture parent directory");
    fs::write(path, content).expect("fixture content");
}

fn digest(content: &[u8]) -> String {
    blake3::hash(content).to_hex().to_string()
}

fn git_path(root: &Path, name: &str) -> PathBuf {
    let path = PathBuf::from(git(root, &["rev-parse", "--git-path", name]).trim());
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(root)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args(args)
        .output()
        .expect("run Git fixture command");
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("Git output is UTF-8")
}

fn command(root: &Path, command: &[&str]) {
    let status = Command::new(command[0])
        .current_dir(root)
        .args(&command[1..])
        .status()
        .expect("run fixture command");
    assert!(status.success(), "{} failed", command.join(" "));
}
