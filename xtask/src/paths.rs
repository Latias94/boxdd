use std::path::{Path, PathBuf};

use crate::{Error, Result};

#[derive(Clone, Debug)]
pub struct WorkspacePaths {
    root: PathBuf,
}

impl WorkspacePaths {
    pub fn discover() -> Result<Self> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let root = manifest_dir
            .parent()
            .ok_or_else(|| Error::message("xtask manifest has no parent directory"))?;
        Ok(Self::new(root))
    }

    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn box2d(&self) -> PathBuf {
        self.root.join("boxdd-sys/third-party/box2d")
    }

    pub fn box2d_headers(&self) -> PathBuf {
        self.box2d().join("include/box2d")
    }

    pub fn upstream_manifest(&self) -> PathBuf {
        self.root.join("boxdd-sys/upstream.toml")
    }
}
