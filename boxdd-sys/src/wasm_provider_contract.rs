//! Checked WASM provider ABI identities consumed by the sys build script.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

pub(crate) const SCHEMA_VERSION: u64 = 1;
pub(crate) const SCHEMA_NAME: &str = "boxdd-wasm-provider-identity-v1";
pub(crate) const PROVIDER_ABI: &str = "box2d-sys-v1";
#[allow(dead_code)] // Used by xtask's shared copy; build.rs receives Cargo's target dynamically.
pub(crate) const CONSUMER_TARGET: &str = "wasm32-unknown-unknown";
pub(crate) const COMPILER_TARGET: &str = "wasm32-unknown-emscripten";
pub(crate) const SIMD_MODE: &str = "disabled";
pub(crate) const POINTER_WIDTH: u64 = 32;
pub(crate) const ENDIANNESS: &str = "little";

const FIELDS: &[&str] = &[
    "schema_version",
    "schema",
    "provider_abi",
    "target",
    "compiler_target",
    "precision",
    "upstream_sha",
    "source_tree",
    "effective_source_sha256",
    "adapter_abi_version",
    "adapter_source_sha256",
    "recording_contract_blake3",
    "validation_enabled",
    "simd",
    "pointer_width",
    "endianness",
    "bindings_sha256",
    "private_abi_hash",
    "snapshot_layout_hash",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WasmProviderIdentity {
    pub(crate) provider_abi: String,
    pub(crate) target: String,
    pub(crate) compiler_target: String,
    pub(crate) precision: String,
    pub(crate) upstream_sha: String,
    pub(crate) source_tree: String,
    pub(crate) effective_source_sha256: String,
    pub(crate) adapter_abi_version: u64,
    pub(crate) adapter_source_sha256: String,
    pub(crate) recording_contract_blake3: String,
    pub(crate) validation_enabled: bool,
    pub(crate) simd: String,
    pub(crate) pointer_width: u64,
    pub(crate) endianness: String,
    pub(crate) bindings_sha256: String,
    pub(crate) private_abi_hash: [u8; 32],
    pub(crate) snapshot_layout_hash: u32,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct WasmProviderExpectation<'a> {
    pub(crate) provider_abi: &'a str,
    pub(crate) target: &'a str,
    pub(crate) compiler_target: &'a str,
    pub(crate) precision: &'a str,
    pub(crate) upstream_sha: &'a str,
    pub(crate) source_tree: &'a str,
    pub(crate) effective_source_sha256: &'a str,
    pub(crate) adapter_abi_version: u64,
    pub(crate) adapter_source_sha256: &'a str,
    pub(crate) recording_contract_blake3: &'a str,
    pub(crate) validation_enabled: bool,
    pub(crate) simd: &'a str,
    pub(crate) pointer_width: u64,
    pub(crate) endianness: &'a str,
    pub(crate) bindings_sha256: &'a str,
}

impl WasmProviderIdentity {
    #[allow(dead_code)] // Used by xtask's shared copy; build.rs only consumes checked contracts.
    pub(crate) fn from_compiled(
        expected: &WasmProviderExpectation<'_>,
        private_abi_hash: [u8; 32],
        snapshot_layout_hash: u32,
    ) -> Result<Self, String> {
        if private_abi_hash.iter().all(|byte| *byte == 0) {
            return Err("private_abi_hash must be non-zero".to_owned());
        }
        if snapshot_layout_hash == 0 {
            return Err("snapshot_layout_hash must be non-zero".to_owned());
        }
        let identity = Self {
            provider_abi: expected.provider_abi.to_owned(),
            target: expected.target.to_owned(),
            compiler_target: expected.compiler_target.to_owned(),
            precision: expected.precision.to_owned(),
            upstream_sha: expected.upstream_sha.to_owned(),
            source_tree: expected.source_tree.to_owned(),
            effective_source_sha256: expected.effective_source_sha256.to_owned(),
            adapter_abi_version: expected.adapter_abi_version,
            adapter_source_sha256: expected.adapter_source_sha256.to_owned(),
            recording_contract_blake3: expected.recording_contract_blake3.to_owned(),
            validation_enabled: expected.validation_enabled,
            simd: expected.simd.to_owned(),
            pointer_width: expected.pointer_width,
            endianness: expected.endianness.to_owned(),
            bindings_sha256: expected.bindings_sha256.to_owned(),
            private_abi_hash,
            snapshot_layout_hash,
        };
        identity.validate(expected)?;
        let reparsed = Self::parse(&identity.render())?;
        if reparsed == identity {
            Ok(identity)
        } else {
            Err("rendered WASM provider identity did not round-trip".to_owned())
        }
    }

    pub(crate) fn load(
        root: &Path,
        relative: &Path,
        expected: &WasmProviderExpectation<'_>,
    ) -> Result<Self, String> {
        Self::load_with_source_bytes(root, relative, expected).map(|(identity, _)| identity)
    }

    #[allow(dead_code)] // Used by xtask's shared copy; build.rs only consumes the identity.
    pub(crate) fn load_with_source_bytes(
        root: &Path,
        relative: &Path,
        expected: &WasmProviderExpectation<'_>,
    ) -> Result<(Self, Vec<u8>), String> {
        let path = regular_file_within(root, relative)?;
        let source = fs::read_to_string(&path).map_err(|error| {
            format!(
                "failed to read WASM provider identity contract {}: {error}",
                path.display()
            )
        })?;
        let identity = Self::parse(&source)?;
        if source != identity.render() {
            return Err(format!(
                "WASM provider identity contract is not in canonical form: {}",
                path.display()
            ));
        }
        identity.validate(expected)?;
        Ok((identity, source.into_bytes()))
    }

    pub(crate) fn parse(source: &str) -> Result<Self, String> {
        let value: toml::Value = toml::from_str(source)
            .map_err(|error| format!("invalid WASM provider identity TOML: {error}"))?;
        let table = value
            .as_table()
            .ok_or_else(|| "WASM provider identity root must be a TOML table".to_owned())?;
        let fields = table.keys().map(String::as_str).collect::<BTreeSet<_>>();
        let expected_fields = FIELDS.iter().copied().collect::<BTreeSet<_>>();
        if fields != expected_fields {
            return Err(format!(
                "WASM provider identity fields do not match the closed schema: expected {expected_fields:?}, found {fields:?}"
            ));
        }
        if required_integer(table, "schema_version")? != SCHEMA_VERSION {
            return Err("unsupported WASM provider identity schema_version".to_owned());
        }
        require_equal(
            "WASM provider identity schema",
            &required_string(table, "schema")?,
            SCHEMA_NAME,
        )?;
        let snapshot_layout_hash = u32::try_from(required_integer(table, "snapshot_layout_hash")?)
            .map_err(|_| "snapshot_layout_hash exceeds u32".to_owned())?;
        if snapshot_layout_hash == 0 {
            return Err("snapshot_layout_hash must be non-zero".to_owned());
        }
        let private_abi_hash = parse_sha256(
            "private_abi_hash",
            &required_string(table, "private_abi_hash")?,
        )?;
        if private_abi_hash.iter().all(|byte| *byte == 0) {
            return Err("private_abi_hash must be non-zero".to_owned());
        }
        Ok(Self {
            provider_abi: required_string(table, "provider_abi")?,
            target: required_string(table, "target")?,
            compiler_target: required_string(table, "compiler_target")?,
            precision: required_string(table, "precision")?,
            upstream_sha: required_string(table, "upstream_sha")?,
            source_tree: required_string(table, "source_tree")?,
            effective_source_sha256: required_string(table, "effective_source_sha256")?,
            adapter_abi_version: required_integer(table, "adapter_abi_version")?,
            adapter_source_sha256: required_string(table, "adapter_source_sha256")?,
            recording_contract_blake3: required_string(table, "recording_contract_blake3")?,
            validation_enabled: required_bool(table, "validation_enabled")?,
            simd: required_string(table, "simd")?,
            pointer_width: required_integer(table, "pointer_width")?,
            endianness: required_string(table, "endianness")?,
            bindings_sha256: required_string(table, "bindings_sha256")?,
            private_abi_hash,
            snapshot_layout_hash,
        })
    }

    pub(crate) fn validate(&self, expected: &WasmProviderExpectation<'_>) -> Result<(), String> {
        require_equal(
            "WASM provider ABI",
            &self.provider_abi,
            expected.provider_abi,
        )?;
        require_equal("WASM provider target", &self.target, expected.target)?;
        require_equal(
            "WASM provider compiler target",
            &self.compiler_target,
            expected.compiler_target,
        )?;
        require_equal(
            "WASM provider precision",
            &self.precision,
            expected.precision,
        )?;
        require_equal(
            "WASM provider upstream SHA",
            &self.upstream_sha,
            expected.upstream_sha,
        )?;
        require_equal(
            "WASM provider source tree",
            &self.source_tree,
            expected.source_tree,
        )?;
        require_equal(
            "WASM provider effective-source SHA-256",
            &self.effective_source_sha256,
            expected.effective_source_sha256,
        )?;
        if self.adapter_abi_version != expected.adapter_abi_version {
            return Err(format!(
                "WASM provider adapter ABI version {} does not match {}",
                self.adapter_abi_version, expected.adapter_abi_version
            ));
        }
        require_equal(
            "WASM provider adapter-source SHA-256",
            &self.adapter_source_sha256,
            expected.adapter_source_sha256,
        )?;
        require_equal(
            "WASM provider recording contract BLAKE3",
            &self.recording_contract_blake3,
            expected.recording_contract_blake3,
        )?;
        if self.validation_enabled != expected.validation_enabled {
            return Err(format!(
                "WASM provider validation mode {} does not match {}",
                self.validation_enabled, expected.validation_enabled
            ));
        }
        require_equal("WASM provider SIMD mode", &self.simd, expected.simd)?;
        if self.pointer_width != expected.pointer_width {
            return Err(format!(
                "WASM provider pointer width {} does not match {}",
                self.pointer_width, expected.pointer_width
            ));
        }
        require_equal(
            "WASM provider endianness",
            &self.endianness,
            expected.endianness,
        )?;
        require_equal(
            "WASM provider bindings SHA-256",
            &self.bindings_sha256,
            expected.bindings_sha256,
        )
    }

    #[allow(dead_code)] // Used by xtask's shared copy; build.rs only consumes checked contracts.
    pub(crate) fn expectation(&self) -> WasmProviderExpectation<'_> {
        WasmProviderExpectation {
            provider_abi: &self.provider_abi,
            target: &self.target,
            compiler_target: &self.compiler_target,
            precision: &self.precision,
            upstream_sha: &self.upstream_sha,
            source_tree: &self.source_tree,
            effective_source_sha256: &self.effective_source_sha256,
            adapter_abi_version: self.adapter_abi_version,
            adapter_source_sha256: &self.adapter_source_sha256,
            recording_contract_blake3: &self.recording_contract_blake3,
            validation_enabled: self.validation_enabled,
            simd: &self.simd,
            pointer_width: self.pointer_width,
            endianness: &self.endianness,
            bindings_sha256: &self.bindings_sha256,
        }
    }

    pub(crate) fn render(&self) -> String {
        format!(
            "schema_version = {SCHEMA_VERSION}\n\
schema = {}\n\
provider_abi = {}\n\
target = {}\n\
compiler_target = {}\n\
precision = {}\n\
upstream_sha = {}\n\
source_tree = {}\n\
effective_source_sha256 = {}\n\
adapter_abi_version = {}\n\
adapter_source_sha256 = {}\n\
recording_contract_blake3 = {}\n\
validation_enabled = {}\n\
simd = {}\n\
pointer_width = {}\n\
endianness = {}\n\
bindings_sha256 = {}\n\
private_abi_hash = {}\n\
snapshot_layout_hash = {}\n",
            toml_string(SCHEMA_NAME),
            toml_string(&self.provider_abi),
            toml_string(&self.target),
            toml_string(&self.compiler_target),
            toml_string(&self.precision),
            toml_string(&self.upstream_sha),
            toml_string(&self.source_tree),
            toml_string(&self.effective_source_sha256),
            self.adapter_abi_version,
            toml_string(&self.adapter_source_sha256),
            toml_string(&self.recording_contract_blake3),
            self.validation_enabled,
            toml_string(&self.simd),
            self.pointer_width,
            toml_string(&self.endianness),
            toml_string(&self.bindings_sha256),
            toml_string(&hex_digest(&self.private_abi_hash)),
            self.snapshot_layout_hash,
        )
    }
}

fn toml_string(value: &str) -> String {
    toml::Value::String(value.to_owned()).to_string()
}

fn hex_digest(digest: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut rendered = String::with_capacity(64);
    for byte in digest {
        rendered.push(char::from(HEX[usize::from(byte >> 4)]));
        rendered.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    rendered
}

fn regular_file_within(root: &Path, relative: &Path) -> Result<PathBuf, String> {
    if relative.as_os_str().is_empty()
        || relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "WASM provider identity contract path must be a non-empty normalized relative path: {}",
            relative.display()
        ));
    }
    let root_metadata = fs::symlink_metadata(root).map_err(|error| {
        format!(
            "failed to inspect WASM provider identity root {}: {error}",
            root.display()
        )
    })?;
    if !root_metadata.file_type().is_dir() || root_metadata.file_type().is_symlink() {
        return Err(format!(
            "WASM provider identity root must be a real directory: {}",
            root.display()
        ));
    }
    let canonical_root = fs::canonicalize(root).map_err(|error| {
        format!(
            "failed to resolve WASM provider identity root {}: {error}",
            root.display()
        )
    })?;
    let components = relative.components().collect::<Vec<_>>();
    let mut current = root.to_owned();
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(component) = component else {
            unreachable!("contract path components were validated");
        };
        current.push(component);
        let metadata = fs::symlink_metadata(&current).map_err(|error| {
            format!(
                "failed to inspect WASM provider identity path {}: {error}",
                current.display()
            )
        })?;
        let is_file = index + 1 == components.len();
        let valid_kind = if is_file {
            metadata.file_type().is_file()
        } else {
            metadata.file_type().is_dir()
        };
        if !valid_kind || metadata.file_type().is_symlink() {
            return Err(format!(
                "WASM provider identity path contains a symlink or unexpected file type: {}",
                current.display()
            ));
        }
        let canonical = fs::canonicalize(&current).map_err(|error| {
            format!(
                "failed to resolve WASM provider identity path {}: {error}",
                current.display()
            )
        })?;
        if !canonical.starts_with(&canonical_root) {
            return Err(format!(
                "WASM provider identity path escaped {}: {}",
                canonical_root.display(),
                canonical.display()
            ));
        }
    }
    Ok(current)
}

pub(crate) fn contract_relative_path(precision: &str) -> Result<&'static str, String> {
    match precision {
        "single" => Ok("abi/wasm32-unknown-unknown-single.toml"),
        "double" => Ok("abi/wasm32-unknown-unknown-double.toml"),
        value => Err(format!(
            "unsupported WASM provider identity precision {value:?}"
        )),
    }
}

fn required_string(table: &toml::Table, key: &str) -> Result<String, String> {
    table
        .get(key)
        .and_then(toml::Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("WASM provider identity field `{key}` must be a non-empty string"))
}

fn required_integer(table: &toml::Table, key: &str) -> Result<u64, String> {
    table
        .get(key)
        .and_then(toml::Value::as_integer)
        .and_then(|value| u64::try_from(value).ok())
        .ok_or_else(|| {
            format!("WASM provider identity field `{key}` must be a non-negative integer")
        })
}

fn required_bool(table: &toml::Table, key: &str) -> Result<bool, String> {
    table
        .get(key)
        .and_then(toml::Value::as_bool)
        .ok_or_else(|| format!("WASM provider identity field `{key}` must be a boolean"))
}

fn parse_sha256(label: &str, value: &str) -> Result<[u8; 32], String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!(
            "{label} must be 64 lowercase hexadecimal characters"
        ));
    }
    let mut digest = [0_u8; 32];
    for (index, byte) in digest.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|error| format!("invalid {label}: {error}"))?;
    }
    Ok(digest)
}

fn require_equal(label: &str, actual: &str, expected: &str) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{label} does not match the checked contract: expected {expected:?}, found {actual:?}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expected(precision: &'static str) -> WasmProviderExpectation<'static> {
        WasmProviderExpectation {
            provider_abi: PROVIDER_ABI,
            target: CONSUMER_TARGET,
            compiler_target: COMPILER_TARGET,
            precision,
            upstream_sha: "56edae79f2949d86142b03450d5d60f63bcf5a6f",
            source_tree: "63a1ab02e3d2bf7c4d86b257b78976842b8c5ddb",
            effective_source_sha256: "9948291f4ea6e14b01304d19473e4539f47313133b4c2e7c6f3ae312d4f2c112",
            adapter_abi_version: 2,
            adapter_source_sha256: "3c985fa213a9ccb43934798bddbb018668e8691e9762873cd2531ea24cdcf337",
            recording_contract_blake3: "26e9ed79e7e4d7ac00d927be5e9c184f2058c585c7369c589ced11da14ddefe2",
            validation_enabled: false,
            simd: SIMD_MODE,
            pointer_width: POINTER_WIDTH,
            endianness: ENDIANNESS,
            bindings_sha256: if precision == "single" {
                "d0fd5f8504352210e6b6d21f9e75bb435c82ab6c2fe3912fd753ef864993ad3e"
            } else {
                "6f5128d028fb47497b48e5db6fb4926517a459901b7707d02011f9615c440dd1"
            },
        }
    }

    #[test]
    fn checked_contracts_are_closed_and_precision_specific() {
        for (precision, source) in [
            (
                "single",
                include_str!("../abi/wasm32-unknown-unknown-single.toml"),
            ),
            (
                "double",
                include_str!("../abi/wasm32-unknown-unknown-double.toml"),
            ),
        ] {
            let temp = tempfile::tempdir().unwrap();
            let relative = contract_relative_path(precision).unwrap();
            let path = temp.path().join(Path::new(relative).file_name().unwrap());
            fs::write(&path, source).unwrap();
            let identity = WasmProviderIdentity::load(
                temp.path(),
                Path::new(path.file_name().unwrap()),
                &expected(precision),
            )
            .unwrap();
            assert_eq!(source, identity.render());
            fs::write(&path, format!("\n{source}")).unwrap();
            assert!(
                WasmProviderIdentity::load(
                    temp.path(),
                    Path::new(path.file_name().unwrap()),
                    &expected(precision),
                )
                .is_err(),
                "non-canonical whitespace must be rejected"
            );
            fs::write(&path, source).unwrap();
            assert_ne!(identity.private_abi_hash, [0; 32]);
            assert_ne!(identity.snapshot_layout_hash, 0);
            assert!(WasmProviderIdentity::parse(&format!("{source}\nunknown = true\n")).is_err());
            let other = if precision == "single" {
                "double"
            } else {
                "single"
            };
            assert!(identity.validate(&expected(other)).is_err());
        }
        assert!(contract_relative_path("extended").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn contract_loader_rejects_symbolic_links() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("provider-contract-target.toml");
        fs::write(
            &target,
            include_str!("../abi/wasm32-unknown-unknown-single.toml"),
        )
        .unwrap();
        let link = temp.path().join("provider-contract.toml");
        symlink(&target, &link).unwrap();

        assert!(
            WasmProviderIdentity::load(
                temp.path(),
                Path::new("provider-contract.toml"),
                &expected("single")
            )
            .is_err()
        );
    }

    #[cfg(unix)]
    #[test]
    fn contract_loader_rejects_symbolic_linked_parent_directories() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(
            outside.path().join("provider-contract.toml"),
            include_str!("../abi/wasm32-unknown-unknown-single.toml"),
        )
        .unwrap();
        symlink(outside.path(), temp.path().join("abi")).unwrap();

        assert!(
            WasmProviderIdentity::load(
                temp.path(),
                Path::new("abi/provider-contract.toml"),
                &expected("single"),
            )
            .is_err()
        );
    }
}
