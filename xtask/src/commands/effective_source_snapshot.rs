//! Immutable effective-source inputs shared by native artifact validators.

use std::{
    collections::BTreeSet,
    path::{Component, Path, PathBuf},
};

use tempfile::TempDir;

use crate::{
    Error, Result,
    build_support::VerifiedFileSnapshot,
    provider_manifest::MAX_PROVIDER_HEADER_BYTES,
    source_overlay::{
        BOX2D_PUBLIC_HEADER, EffectiveSourceIdentity, MaterializedEffectiveSources,
        effective_source_identity, materialize_effective_box2d_sources,
    },
};

const PUBLIC_HEADER_PREFIX: &str = "include/box2d/";
const MAX_EFFECTIVE_PUBLIC_HEADERS: usize = 64;

/// A verified public header materialized from one effective-source generation.
///
/// The temporary directory keeps the immutable header alive while artifact validation runs. The
/// caller must revalidate after consuming the header so concurrent changes to the live source
/// cohort cannot be accepted under an identity captured before the change.
pub(crate) struct EffectiveHeaderSnapshot {
    manifest_dir: PathBuf,
    _materialization: TempDir,
    materialized: MaterializedEffectiveSources,
    header: VerifiedFileSnapshot,
}

impl EffectiveHeaderSnapshot {
    pub(crate) fn capture(manifest_dir: &Path, context: &str) -> Result<Self> {
        let materialization = tempfile::Builder::new()
            .prefix("boxdd-effective-source-")
            .tempdir()
            .map_err(|error| Error::io("create effective-source materialization", error))?;
        let effective_sources =
            materialize_effective_box2d_sources(manifest_dir, materialization.path()).map_err(
                |error| {
                    Error::message(format!(
                        "cannot materialize the {context} effective source: {error}"
                    ))
                },
            )?;
        let header = VerifiedFileSnapshot::read(
            &effective_sources.root.join(BOX2D_PUBLIC_HEADER),
            MAX_PROVIDER_HEADER_BYTES,
            "effective public Box2D header",
        )
        .map_err(Error::message)?;
        let snapshot = Self {
            manifest_dir: manifest_dir.to_path_buf(),
            _materialization: materialization,
            materialized: effective_sources,
            header,
        };
        snapshot.revalidate(context)?;
        Ok(snapshot)
    }

    pub(crate) fn identity(&self) -> &EffectiveSourceIdentity {
        &self.materialized.identity
    }

    pub(crate) fn header_path(&self) -> &Path {
        self.header.path()
    }

    pub(crate) fn public_header_paths(&self) -> Result<BTreeSet<String>> {
        let root = self.materialized.public_include.join("box2d");
        let mut paths = BTreeSet::new();
        collect_public_header_paths(&root, &root, &mut paths)?;
        if paths.is_empty() {
            return Err(Error::message(format!(
                "the effective public header cohort at {} is empty",
                root.display()
            )));
        }
        Ok(paths)
    }

    pub(crate) fn verify_packaged_header(&self, packaged: &str, bytes: &[u8]) -> Result<()> {
        let source = self.public_header_path(packaged)?;
        let expected = VerifiedFileSnapshot::read(
            &source,
            MAX_PROVIDER_HEADER_BYTES,
            "effective public Box2D header",
        )
        .map_err(Error::message)?;
        if expected.bytes() != bytes {
            return Err(Error::message(format!(
                "packaged {packaged} does not exactly match materialized effective header {}",
                source.display()
            )));
        }
        Ok(())
    }

    pub(crate) fn revalidate(&self, context: &str) -> Result<()> {
        self.materialized.revalidate().map_err(|error| {
            Error::message(format!(
                "cannot revalidate the {context} materialized effective source: {error}"
            ))
        })?;
        self.header
            .revalidate("effective public Box2D header")
            .map_err(Error::message)?;
        let live_identity = effective_source_identity(&self.manifest_dir).map_err(|error| {
            Error::message(format!(
                "cannot revalidate the {context} effective source: {error}"
            ))
        })?;
        if live_identity != self.materialized.identity {
            return Err(Error::message(format!(
                "the {context} effective-source cohort changed while its artifact was being validated"
            )));
        }
        Ok(())
    }

    fn public_header_path(&self, packaged: &str) -> Result<PathBuf> {
        let relative = packaged.strip_prefix(PUBLIC_HEADER_PREFIX).ok_or_else(|| {
            Error::message(format!("invalid effective public header path {packaged:?}"))
        })?;
        let relative_path = Path::new(relative);
        if relative.is_empty()
            || packaged.contains('\\')
            || relative_path
                .extension()
                .and_then(|extension| extension.to_str())
                != Some("h")
            || !relative_path
                .components()
                .all(|component| matches!(component, Component::Normal(_)))
        {
            return Err(Error::message(format!(
                "invalid effective public header path {packaged:?}"
            )));
        }
        Ok(self
            .materialized
            .public_include
            .join("box2d")
            .join(relative_path))
    }
}

fn collect_public_header_paths(
    root: &Path,
    directory: &Path,
    paths: &mut BTreeSet<String>,
) -> Result<()> {
    let entries = std::fs::read_dir(directory).map_err(|error| Error::io(directory, error))?;
    for entry in entries {
        let entry = entry.map_err(|error| Error::io(directory, error))?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| Error::io(&path, error))?;
        if file_type.is_symlink() {
            return Err(Error::message(format!(
                "effective public header cohort contains a symlink: {}",
                path.display()
            )));
        }
        if file_type.is_dir() {
            collect_public_header_paths(root, &path, paths)?;
            continue;
        }
        if !file_type.is_file() {
            return Err(Error::message(format!(
                "effective public header cohort contains a non-regular path: {}",
                path.display()
            )));
        }
        if path.extension().and_then(|extension| extension.to_str()) != Some("h") {
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|_| Error::message("effective public header escaped its source root"))?
            .to_str()
            .ok_or_else(|| Error::message("effective public header path is not UTF-8"))?;
        let packaged = format!("{PUBLIC_HEADER_PREFIX}{}", relative.replace('\\', "/"));
        if paths.insert(packaged) && paths.len() > MAX_EFFECTIVE_PUBLIC_HEADERS {
            return Err(Error::message(format!(
                "effective public header cohort exceeds {MAX_EFFECTIVE_PUBLIC_HEADERS} files"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::EffectiveHeaderSnapshot;
    use std::fs;

    #[test]
    fn materialized_header_is_bound_to_the_effective_source_identity() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../boxdd-sys");
        let snapshot = EffectiveHeaderSnapshot::capture(&manifest_dir, "test fixture").unwrap();

        assert_eq!(snapshot.identity().upstream_sha.len(), 40);
        assert!(snapshot.header_path().is_file());
        snapshot.revalidate("test fixture").unwrap();
    }

    #[test]
    fn public_header_cohort_uses_materialized_effective_bytes() {
        let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask manifest has a workspace parent");
        let manifest_dir = workspace.join("boxdd-sys");
        let snapshot = EffectiveHeaderSnapshot::capture(&manifest_dir, "test fixture").unwrap();
        let packaged = "include/box2d/box2d.h";
        let effective = fs::read(snapshot.header_path()).unwrap();
        let vendored =
            fs::read(manifest_dir.join("third-party/box2d/include/box2d/box2d.h")).unwrap();

        assert!(snapshot.public_header_paths().unwrap().contains(packaged));
        assert_ne!(effective, vendored);
        snapshot
            .verify_packaged_header(packaged, &effective)
            .unwrap();
        assert!(
            snapshot
                .verify_packaged_header(packaged, &vendored)
                .is_err()
        );
    }
}
