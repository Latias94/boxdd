use std::collections::BTreeSet;
use std::fmt;
use std::path::{Component, PathBuf};

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum SourceInventoryError {
    Empty,
    Duplicate(PathBuf),
    InvalidPath { path: String, reason: &'static str },
}

impl fmt::Display for SourceInventoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("C source inventory must not be empty"),
            Self::Duplicate(path) => {
                write!(
                    formatter,
                    "C source inventory contains duplicate path {path:?}"
                )
            }
            Self::InvalidPath { path, reason } => {
                write!(
                    formatter,
                    "invalid C source inventory path {path:?}: {reason}"
                )
            }
        }
    }
}

pub(crate) fn validate_c_source_paths<'a>(
    sources: impl IntoIterator<Item = &'a str>,
) -> Result<Vec<PathBuf>, SourceInventoryError> {
    let mut seen = BTreeSet::new();
    let mut validated = Vec::new();

    for source in sources {
        let source_path = validate_c_source_path(source)?;
        if !seen.insert(source_path.clone()) {
            return Err(SourceInventoryError::Duplicate(source_path));
        }
        validated.push(source_path);
    }

    if validated.is_empty() {
        return Err(SourceInventoryError::Empty);
    }
    Ok(validated)
}

fn validate_c_source_path(source: &str) -> Result<PathBuf, SourceInventoryError> {
    if source.contains('\\') {
        return Err(invalid_path(source, "paths must use forward slashes"));
    }

    let segments = source.split('/').collect::<Vec<_>>();
    if segments.len() < 2
        || segments[0] != "src"
        || segments
            .iter()
            .any(|segment| segment.is_empty() || *segment == "." || *segment == "..")
    {
        return Err(invalid_path(
            source,
            "paths must be normalized, relative, and below src/",
        ));
    }

    let source_path = PathBuf::from(source);
    if !source_path
        .extension()
        .is_some_and(|extension| extension == "c")
        || !source_path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(invalid_path(source, "paths must name a .c file"));
    }
    Ok(source_path)
}

fn invalid_path(path: &str, reason: &'static str) -> SourceInventoryError {
    SourceInventoryError::InvalidPath {
        path: path.to_owned(),
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::{SourceInventoryError, validate_c_source_paths};
    use std::path::PathBuf;

    #[test]
    fn accepts_normalized_nested_c_sources_in_manifest_order() {
        let sources = ["src/core.c", "src/solver/contact.c"];

        assert_eq!(
            validate_c_source_paths(sources).unwrap(),
            sources.map(PathBuf::from)
        );
    }

    #[test]
    fn rejects_empty_and_duplicate_inventories() {
        assert_eq!(
            validate_c_source_paths(std::iter::empty()).unwrap_err(),
            SourceInventoryError::Empty
        );
        assert_eq!(
            validate_c_source_paths(["src/core.c", "src/core.c"]).unwrap_err(),
            SourceInventoryError::Duplicate(PathBuf::from("src/core.c"))
        );
    }

    #[test]
    fn rejects_paths_outside_the_reviewed_source_tree() {
        for source in [
            "/src/core.c",
            "core.c",
            "test/core.c",
            "src/../test/core.c",
            "src/./core.c",
            "src//core.c",
            "src\\core.c",
            "src/core.cpp",
        ] {
            assert!(
                matches!(
                    validate_c_source_paths([source]),
                    Err(SourceInventoryError::InvalidPath { .. })
                ),
                "unexpectedly accepted {source:?}"
            );
        }
    }
}
