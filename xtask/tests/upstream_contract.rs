use std::{collections::BTreeMap, fs, path::PathBuf, process::Command};

use tempfile::TempDir;
use xtask::{
    commands::upstream_sync::{ArtifactKind, UpstreamManifest},
    paths::WorkspacePaths,
};

fn workspace() -> (PathBuf, WorkspacePaths) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a workspace parent")
        .to_owned();
    let paths = WorkspacePaths::new(&root);
    (root, paths)
}

#[test]
fn public_upstream_check_is_read_only() {
    let (root, paths) = workspace();
    let manifest = UpstreamManifest::load(&paths).expect("workspace upstream manifest");
    let mut managed_paths = manifest
        .artifacts
        .iter()
        .map(|artifact| root.join(&artifact.path))
        .collect::<Vec<_>>();
    managed_paths.push(paths.upstream_manifest());
    let managed_before = snapshot_files(&managed_paths);
    let root_index = git_path(&root, "index");
    let root_index_before = fs::read(&root_index).expect("root Git index");
    let submodule_head_before = git(&paths.box2d(), &["rev-parse", "HEAD"]);
    let gitlink_before = git(
        &root,
        &["ls-files", "--stage", "--", "boxdd-sys/third-party/box2d"],
    );

    xtask::run_in(&paths, ["upstream-sync".to_owned(), "--check".to_owned()])
        .expect("workspace upstream check");

    assert_eq!(snapshot_files(&managed_paths), managed_before);
    assert_eq!(
        fs::read(&root_index).expect("unchanged root Git index"),
        root_index_before
    );
    assert_eq!(
        git(&paths.box2d(), &["rev-parse", "HEAD"]),
        submodule_head_before
    );
    assert_eq!(
        git(
            &root,
            &["ls-files", "--stage", "--", "boxdd-sys/third-party/box2d"]
        ),
        gitlink_before
    );
}

#[test]
fn public_upstream_check_rejects_an_invalid_manifest_without_writing() {
    let (_, workspace_paths) = workspace();
    let mut manifest =
        UpstreamManifest::load(&workspace_paths).expect("workspace upstream manifest");
    manifest.schema_version += 1;

    let root = TempDir::new().unwrap();
    let paths = WorkspacePaths::new(root.path());
    fs::create_dir_all(paths.upstream_manifest().parent().unwrap()).unwrap();
    let content = toml::to_string_pretty(&manifest).unwrap();
    fs::write(paths.upstream_manifest(), &content).unwrap();

    let error = xtask::run_in(&paths, ["upstream-sync".to_owned(), "--check".to_owned()])
        .expect_err("unsupported manifest schema must fail");
    assert!(error.to_string().contains("schema"));
    assert_eq!(
        fs::read_to_string(paths.upstream_manifest()).unwrap(),
        content
    );
}

#[test]
fn manifest_has_only_release_relevant_artifacts() {
    let (_, paths) = workspace();
    let manifest = UpstreamManifest::load(&paths).expect("workspace upstream manifest");
    assert_eq!(manifest.artifacts.len(), 9);
    assert_eq!(
        manifest
            .artifacts
            .iter()
            .filter(|artifact| artifact.kind == ArtifactKind::Bindings)
            .count(),
        6
    );
    assert_eq!(
        manifest
            .artifacts
            .iter()
            .filter(|artifact| artifact.kind == ArtifactKind::RecordingWire)
            .count(),
        1
    );
    assert_eq!(
        manifest
            .artifacts
            .iter()
            .filter(|artifact| artifact.kind == ArtifactKind::ProviderIdentity)
            .count(),
        2
    );
}

#[test]
fn removed_state_machine_flags_are_rejected() {
    let (_, paths) = workspace();
    for flag in ["--prepare-next", "--check-next", "--refresh-routes"] {
        let error = xtask::run_in(&paths, ["upstream-sync".to_owned(), flag.to_owned()])
            .expect_err("removed state-machine flag must fail");
        assert!(error.to_string().contains("expects --check or --write"));
    }
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

fn git_path(root: &std::path::Path, name: &str) -> PathBuf {
    let output = git(root, &["rev-parse", "--git-path", name]);
    let path = PathBuf::from(output.trim());
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

fn git(root: &std::path::Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("run Git");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("Git output is UTF-8")
}
