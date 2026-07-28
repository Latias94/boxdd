use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    Error, Result,
    config::{AtomicFileUpdate, write_atomic_batch_checked},
    source_overlay::{
        BUILD_POLICY_SOURCE_PATHS, BuildPolicySourceIdentity, build_policy_source_identity,
        canonicalize_build_policy_source, is_rust_build_policy_source,
    },
};

use super::{UpdateMode, parse_update_mode, upstream_sync::UpdateLock};

const RUST_POLICY_SOURCE_COUNT: usize = 10;

#[derive(Debug)]
struct CapturedPolicySource {
    relative_path: &'static str,
    path: PathBuf,
    baseline: Vec<u8>,
    canonical: Vec<u8>,
    identity: BuildPolicySourceIdentity,
}

pub fn run(root: &Path, args: &[String]) -> Result<()> {
    let mode = parse_update_mode("build-policy-sources", args)?;
    let _lock = UpdateLock::acquire(root)?;
    let captured = capture_policy_sources(root)?;
    revalidate_captured_sources(&captured)?;

    match mode {
        UpdateMode::Check => {
            require_current_sources(&captured)?;
            println!("all 10 Rust build-policy source identities are canonical and current");
            Ok(())
        }
        UpdateMode::Write => {
            let updates = captured
                .iter()
                .map(|source| {
                    AtomicFileUpdate::checked(&source.path, &source.baseline, &source.canonical)
                })
                .collect::<Vec<_>>();
            write_atomic_batch_checked(root, &updates, || validate_installed_sources(root))?;
            println!("atomically refreshed all 10 Rust build-policy source identities");
            Ok(())
        }
    }
}

fn rust_policy_source_paths() -> Result<Vec<&'static str>> {
    let paths = BUILD_POLICY_SOURCE_PATHS
        .iter()
        .copied()
        .filter(|path| is_rust_build_policy_source(path))
        .collect::<Vec<_>>();
    if paths.len() != RUST_POLICY_SOURCE_COUNT {
        return Err(Error::message(format!(
            "build-policy inventory contains {} Rust sources; expected {RUST_POLICY_SOURCE_COUNT}",
            paths.len()
        )));
    }
    if !paths.windows(2).all(|pair| pair[0] < pair[1]) {
        return Err(Error::message(
            "Rust build-policy source inventory must be unique and sorted",
        ));
    }
    Ok(paths)
}

fn capture_policy_sources(root: &Path) -> Result<Vec<CapturedPolicySource>> {
    let manifest_root = root.join("boxdd-sys");
    let mut captured = Vec::with_capacity(RUST_POLICY_SOURCE_COUNT);
    let mut errors = Vec::new();
    for relative_path in rust_policy_source_paths()? {
        match capture_policy_source(&manifest_root, relative_path) {
            Ok(source) => captured.push(source),
            Err(error) => errors.push(format!("{relative_path}: {error}")),
        }
    }
    if errors.is_empty() {
        Ok(captured)
    } else {
        Err(Error::message(format!(
            "Rust build-policy source validation failed:\n- {}",
            errors.join("\n- ")
        )))
    }
}

fn capture_policy_source(
    manifest_root: &Path,
    relative_path: &'static str,
) -> Result<CapturedPolicySource> {
    let path = manifest_root.join(relative_path);
    let metadata = fs::symlink_metadata(&path).map_err(|source| Error::io(&path, source))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(Error::message(format!(
            "{} must be a regular non-symlink file",
            path.display()
        )));
    }
    let baseline = fs::read(&path).map_err(|source| Error::io(&path, source))?;
    let identity =
        build_policy_source_identity(relative_path, &baseline).map_err(Error::message)?;
    let canonical =
        canonicalize_build_policy_source(relative_path, &baseline).map_err(Error::message)?;
    Ok(CapturedPolicySource {
        relative_path,
        path,
        baseline,
        canonical,
        identity,
    })
}

fn revalidate_captured_sources(captured: &[CapturedPolicySource]) -> Result<()> {
    for source in captured {
        let metadata =
            fs::symlink_metadata(&source.path).map_err(|error| Error::io(&source.path, error))?;
        if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
            return Err(Error::message(format!(
                "captured Rust build-policy source was replaced by a non-file: {}",
                source.path.display()
            )));
        }
        let current = fs::read(&source.path).map_err(|error| Error::io(&source.path, error))?;
        if current != source.baseline {
            return Err(Error::message(format!(
                "Rust build-policy source changed after capture: {}",
                source.path.display()
            )));
        }
    }
    Ok(())
}

fn require_current_sources(captured: &[CapturedPolicySource]) -> Result<()> {
    let drift = captured
        .iter()
        .filter(|source| source.baseline != source.canonical)
        .map(|source| {
            format!(
                "{}: declared {}, expected {}",
                source.relative_path,
                source.identity.declared_sha256,
                source.identity.normalized_sha256
            )
        })
        .collect::<Vec<_>>();
    if drift.is_empty() {
        Ok(())
    } else {
        Err(Error::message(format!(
            "Rust build-policy source identity drift:\n- {}\nrun `cargo run -p xtask -- build-policy-sources --write`",
            drift.join("\n- ")
        )))
    }
}

fn validate_installed_sources(root: &Path) -> Result<()> {
    let installed = capture_policy_sources(root)?;
    require_current_sources(&installed)?;
    revalidate_captured_sources(&installed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_policy_inventory_is_exact_and_sorted() {
        let paths = rust_policy_source_paths().unwrap();
        assert_eq!(paths.len(), RUST_POLICY_SOURCE_COUNT);
        assert_eq!(paths.first(), Some(&"build.rs"));
        assert_eq!(paths.last(), Some(&"src/wasm_provider_contract.rs"));
    }
}
