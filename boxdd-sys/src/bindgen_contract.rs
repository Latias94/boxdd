use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fmt::Write as _;
use std::fs;
use std::path::{Component, Path, PathBuf};

use sha2::{Digest as _, Sha256};

pub const WASI_LIBC_VERSION: &str = "32";
pub const WASI_LIBC_HEADERS_SHA256: &str =
    "0e80041ea13b42db5bcd5dc92d737da7c26e4e5a60b902413a41e09924f37687";
pub const UNKNOWN_UNKNOWN_MATH_HEADER_SHA256: &str =
    "70e00e274e189af73ed321f6490ec3a0b0c58f00286e87fe7d257bb211bb367d";

const WASI_HEADERS_RELATIVE_PATH: &str = "include/wasm32-wasip1";
const WASI_HEADERS_HASH_DOMAIN: &[u8] = b"boxdd-wasi-libc-headers-v1\0";
const UNKNOWN_UNKNOWN_HEADERS_RELATIVE_PATH: &str = "src/bindgen_headers/wasm32_unknown_unknown";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedWasiSysroot {
    pub canonical_path: PathBuf,
    pub headers_root: PathBuf,
    pub headers_sha256: String,
}

impl ValidatedWasiSysroot {
    pub fn identity_sha256(&self) -> &str {
        &self.headers_sha256
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedFreestandingHeaders {
    pub canonical_path: PathBuf,
    pub math_header_sha256: String,
}

impl ValidatedFreestandingHeaders {
    pub fn identity_sha256(&self) -> &str {
        &self.math_header_sha256
    }
}

pub fn validate_bindgen_target_override(
    cargo_target: &str,
    configured: Option<&OsStr>,
) -> Result<(), String> {
    let Some(configured) = configured else {
        return Ok(());
    };
    if configured == OsStr::new(cargo_target) {
        return Ok(());
    }
    Err(format!(
        "BOXDD_SYS_BINDGEN_TARGET must exactly match Cargo TARGET {cargo_target:?}; found {:?}",
        configured.to_string_lossy()
    ))
}

// build.rs consumes this contract; xtask includes the module for route refresh without calling it.
#[allow(dead_code)]
pub fn validate_ambient_header_environment(
    cargo_target: &str,
    binding_generation_required: bool,
    variable: &str,
    configured: Option<&OsStr>,
) -> Result<(), String> {
    if !binding_generation_required
        || !matches!(cargo_target, "wasm32-unknown-unknown" | "wasm32-wasip1")
        || configured.is_none()
    {
        return Ok(());
    }
    Err(format!(
        "{variable} must be unset for reproducible {cargo_target} binding generation; use the repository-pinned header contract"
    ))
}

pub fn resolve_wasi_sysroot(
    cargo_target: &str,
    binding_generation_required: bool,
    configured: Option<&Path>,
) -> Result<Option<ValidatedWasiSysroot>, String> {
    match cargo_target {
        "wasm32-unknown-unknown" => {
            if configured.is_some() {
                Err(
                    "BOXDD_SYS_WASI_SYSROOT is not valid for wasm32-unknown-unknown binding generation"
                        .to_owned(),
                )
            } else {
                Ok(None)
            }
        }
        "wasm32-wasip1" if binding_generation_required => {
            let requested = configured.ok_or_else(|| {
                "wasm32-wasip1 binding generation requires explicit BOXDD_SYS_WASI_SYSROOT"
                    .to_owned()
            })?;
            validate_wasi_sysroot(requested).map(Some)
        }
        _ => Ok(None),
    }
}

pub fn resolve_unknown_unknown_headers(
    manifest_dir: &Path,
    cargo_target: &str,
    binding_generation_required: bool,
) -> Result<Option<ValidatedFreestandingHeaders>, String> {
    if cargo_target != "wasm32-unknown-unknown" || !binding_generation_required {
        return Ok(None);
    }
    validate_unknown_unknown_headers(manifest_dir).map(Some)
}

pub fn validate_unknown_unknown_headers(
    manifest_dir: &Path,
) -> Result<ValidatedFreestandingHeaders, String> {
    let canonical_manifest = fs::canonicalize(manifest_dir).map_err(|error| {
        format!(
            "failed to canonicalize CARGO_MANIFEST_DIR {}: {error}",
            manifest_dir.display()
        )
    })?;
    let requested = canonical_manifest.join(UNKNOWN_UNKNOWN_HEADERS_RELATIVE_PATH);
    let requested_metadata = fs::symlink_metadata(&requested).map_err(|error| {
        format!(
            "failed to inspect wasm32-unknown-unknown bindgen headers {}: {error}",
            requested.display()
        )
    })?;
    if requested_metadata.file_type().is_symlink() || !requested_metadata.is_dir() {
        return Err(format!(
            "wasm32-unknown-unknown bindgen headers {} must be a non-symlink directory",
            requested.display()
        ));
    }
    let canonical_path = fs::canonicalize(&requested).map_err(|error| {
        format!(
            "wasm32-unknown-unknown bindgen headers must provide {}: {error}",
            requested.display()
        )
    })?;
    if !canonical_path.is_dir() || !canonical_path.starts_with(&canonical_manifest) {
        return Err(format!(
            "wasm32-unknown-unknown bindgen headers {} must be a directory inside CARGO_MANIFEST_DIR",
            requested.display()
        ));
    }

    let mut entries = fs::read_dir(&canonical_path)
        .map_err(|error| {
            format!(
                "failed to enumerate wasm32-unknown-unknown bindgen headers {}: {error}",
                canonical_path.display()
            )
        })?
        .map(|entry| {
            entry.map_err(|error| {
                format!(
                    "failed to enumerate wasm32-unknown-unknown bindgen headers {}: {error}",
                    canonical_path.display()
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(fs::DirEntry::file_name);
    if entries.len() != 1 || entries[0].file_name() != OsStr::new("math.h") {
        let inventory = entries
            .iter()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "wasm32-unknown-unknown bindgen headers must contain exactly one regular file named math.h; found [{inventory}]"
        ));
    }

    let math_header = entries.remove(0).path();
    let math_metadata = fs::symlink_metadata(&math_header).map_err(|error| {
        format!(
            "failed to inspect freestanding bindgen header {}: {error}",
            math_header.display()
        )
    })?;
    if !math_metadata.file_type().is_file() {
        return Err(format!(
            "wasm32-unknown-unknown bindgen header {} must be a regular non-symlink file",
            math_header.display()
        ));
    }
    let math_header = canonical_regular_file(&math_header, &canonical_path)?;
    let content = fs::read(&math_header).map_err(|error| {
        format!(
            "failed to read freestanding bindgen header {}: {error}",
            math_header.display()
        )
    })?;
    let math_header_sha256 = lower_hex(&Sha256::digest(content));
    if math_header_sha256 != UNKNOWN_UNKNOWN_MATH_HEADER_SHA256 {
        return Err(format!(
            "wasm32-unknown-unknown math.h identity mismatch: expected SHA-256 {UNKNOWN_UNKNOWN_MATH_HEADER_SHA256}, found {math_header_sha256}"
        ));
    }

    Ok(ValidatedFreestandingHeaders {
        canonical_path,
        math_header_sha256,
    })
}

pub fn validate_wasi_sysroot(requested: &Path) -> Result<ValidatedWasiSysroot, String> {
    if requested.as_os_str().is_empty() {
        return Err("BOXDD_SYS_WASI_SYSROOT cannot be empty".to_owned());
    }
    let canonical_path = fs::canonicalize(requested).map_err(|error| {
        format!(
            "failed to canonicalize BOXDD_SYS_WASI_SYSROOT {}: {error}",
            requested.display()
        )
    })?;
    if !canonical_path.is_dir() {
        return Err(format!(
            "BOXDD_SYS_WASI_SYSROOT {} is not a directory",
            canonical_path.display()
        ));
    }

    let requested_headers = canonical_path.join(WASI_HEADERS_RELATIVE_PATH);
    let headers_root = fs::canonicalize(&requested_headers).map_err(|error| {
        format!(
            "wasi-libc {WASI_LIBC_VERSION} sysroot must provide {}: {error}",
            requested_headers.display()
        )
    })?;
    if !headers_root.is_dir() || !headers_root.starts_with(&canonical_path) {
        return Err(format!(
            "WASI headers {} must be a directory inside BOXDD_SYS_WASI_SYSROOT",
            requested_headers.display()
        ));
    }

    let math_header = headers_root.join("math.h");
    canonical_regular_file(&math_header, &headers_root).map_err(|error| {
        format!(
            "wasi-libc {WASI_LIBC_VERSION} sysroot must provide {}: {error}",
            math_header.display()
        )
    })?;

    let headers_sha256 = hash_regular_files(&headers_root)?;
    if headers_sha256 != WASI_LIBC_HEADERS_SHA256 {
        return Err(format!(
            "wasi-libc sysroot identity mismatch for {}: expected version {WASI_LIBC_VERSION} SHA-256 {WASI_LIBC_HEADERS_SHA256}, found {headers_sha256}",
            headers_root.display()
        ));
    }

    Ok(ValidatedWasiSysroot {
        canonical_path,
        headers_root,
        headers_sha256,
    })
}

fn hash_regular_files(root: &Path) -> Result<String, String> {
    let root = fs::canonicalize(root).map_err(|error| {
        format!(
            "failed to canonicalize WASI headers directory {}: {error}",
            root.display()
        )
    })?;
    let mut files = BTreeMap::new();
    collect_regular_files(&root, &root, &mut files)?;
    if files.is_empty() {
        return Err(format!(
            "WASI headers directory {} contains no regular files",
            root.display()
        ));
    }

    let mut hasher = Sha256::new();
    hasher.update(WASI_HEADERS_HASH_DOMAIN);
    for (relative_path, canonical_file) in files {
        let content = fs::read(&canonical_file).map_err(|error| {
            format!(
                "failed to read WASI header {}: {error}",
                canonical_file.display()
            )
        })?;
        hasher.update(relative_path.as_bytes());
        hasher.update([0]);
        hasher.update(content.len().to_string().as_bytes());
        hasher.update([0]);
        hasher.update(content);
    }
    Ok(lower_hex(&hasher.finalize()))
}

fn collect_regular_files(
    root: &Path,
    directory: &Path,
    files: &mut BTreeMap<String, PathBuf>,
) -> Result<(), String> {
    let entries = fs::read_dir(directory).map_err(|error| {
        format!(
            "failed to enumerate WASI headers directory {}: {error}",
            directory.display()
        )
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "failed to enumerate WASI headers directory {}: {error}",
                directory.display()
            )
        })?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|error| {
            format!("failed to inspect WASI header {}: {error}", path.display())
        })?;
        if metadata.is_dir() {
            let canonical = fs::canonicalize(&path).map_err(|error| {
                format!(
                    "failed to canonicalize WASI headers directory {}: {error}",
                    path.display()
                )
            })?;
            if !canonical.starts_with(root) {
                return Err(format!(
                    "WASI headers directory {} escapes {}",
                    path.display(),
                    root.display()
                ));
            }
            collect_regular_files(root, &path, files)?;
            continue;
        }

        let canonical = canonical_regular_file(&path, root)?;
        let relative = normalized_relative_path(root, &path)?;
        if files.insert(relative.clone(), canonical).is_some() {
            return Err(format!(
                "WASI headers contain duplicate relative path {relative:?}"
            ));
        }
    }
    Ok(())
}

fn canonical_regular_file(path: &Path, root: &Path) -> Result<PathBuf, String> {
    let canonical = fs::canonicalize(path).map_err(|error| {
        format!(
            "failed to canonicalize WASI header {}: {error}",
            path.display()
        )
    })?;
    if !canonical.starts_with(root) {
        return Err(format!(
            "WASI header {} escapes {}",
            path.display(),
            root.display()
        ));
    }
    if !canonical.is_file() {
        return Err(format!(
            "WASI header {} is not a regular file",
            path.display()
        ));
    }
    Ok(canonical)
}

fn normalized_relative_path(root: &Path, path: &Path) -> Result<String, String> {
    let relative = path.strip_prefix(root).map_err(|error| {
        format!(
            "WASI header {} is not below {}: {error}",
            path.display(),
            root.display()
        )
    })?;
    let mut rendered = String::new();
    for component in relative.components() {
        let Component::Normal(component) = component else {
            return Err(format!(
                "WASI header has non-normal relative path {}",
                relative.display()
            ));
        };
        let component = component.to_str().ok_or_else(|| {
            format!(
                "WASI header relative path is not UTF-8: {}",
                relative.display()
            )
        })?;
        if !rendered.is_empty() {
            rendered.push('/');
        }
        rendered.push_str(component);
    }
    if rendered.is_empty() {
        return Err("WASI header relative path cannot be empty".to_owned());
    }
    Ok(rendered)
}

fn lower_hex(bytes: &[u8]) -> String {
    let mut rendered = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut rendered, "{byte:02x}").expect("writing to a String cannot fail");
    }
    rendered
}

#[cfg(test)]
mod tests {
    use super::{
        UNKNOWN_UNKNOWN_MATH_HEADER_SHA256, WASI_LIBC_HEADERS_SHA256, hash_regular_files,
        resolve_unknown_unknown_headers, resolve_wasi_sysroot, validate_ambient_header_environment,
        validate_bindgen_target_override, validate_unknown_unknown_headers, validate_wasi_sysroot,
    };
    use std::collections::BTreeSet;
    use std::ffi::OsStr;
    use std::fs;
    use std::path::Path;

    #[test]
    fn bindgen_target_override_is_only_an_exact_assertion() {
        assert!(validate_bindgen_target_override("wasm32-wasip1", None).is_ok());
        assert!(
            validate_bindgen_target_override("wasm32-wasip1", Some(OsStr::new("wasm32-wasip1")))
                .is_ok()
        );
        assert!(
            validate_bindgen_target_override(
                "wasm32-wasip1",
                Some(OsStr::new("wasm32-unknown-unknown"))
            )
            .is_err()
        );
    }

    #[test]
    fn ambient_header_inputs_are_rejected_only_for_wasm_generation() {
        let configured = Some(OsStr::new("-I/unreviewed"));
        assert!(
            validate_ambient_header_environment(
                "wasm32-unknown-unknown",
                true,
                "BINDGEN_EXTRA_CLANG_ARGS",
                configured
            )
            .is_err()
        );
        assert!(
            validate_ambient_header_environment("wasm32-wasip1", true, "CPATH", configured)
                .is_err()
        );
        assert!(
            validate_ambient_header_environment("wasm32-wasip1", false, "CPATH", configured)
                .is_ok()
        );
        assert!(
            validate_ambient_header_environment(
                "x86_64-unknown-linux-gnu",
                true,
                "CPATH",
                configured
            )
            .is_ok()
        );
    }

    #[test]
    fn header_identity_uses_sorted_relative_paths_lengths_and_contents() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir(directory.path().join("nested")).unwrap();
        fs::write(directory.path().join("nested/z.h"), "omega\n").unwrap();
        fs::write(directory.path().join("a.h"), "alpha\n").unwrap();

        assert_eq!(
            hash_regular_files(directory.path()).unwrap(),
            "7b035df000b909f8745b24bb2d88f2e7e82815ed2f99891f9ee204ab13f689d5"
        );
    }

    #[test]
    fn wasi_sysroot_scope_tracks_whether_generation_is_required() {
        let missing = Path::new("does-not-need-to-exist");
        assert_eq!(
            resolve_wasi_sysroot("x86_64-unknown-linux-gnu", true, Some(missing)).unwrap(),
            None
        );
        assert_eq!(
            resolve_wasi_sysroot("wasm32-wasip1", false, Some(missing)).unwrap(),
            None
        );
        assert!(resolve_wasi_sysroot("wasm32-wasip1", true, None).is_err());
        assert!(resolve_wasi_sysroot("wasm32-unknown-unknown", false, Some(missing)).is_err());
    }

    #[test]
    fn repository_freestanding_header_matches_its_pinned_identity() {
        let manifest_dir = boxdd_sys_manifest_dir();
        let validated = validate_unknown_unknown_headers(&manifest_dir).unwrap();
        assert_eq!(
            validated.identity_sha256(),
            UNKNOWN_UNKNOWN_MATH_HEADER_SHA256
        );
        assert!(
            resolve_unknown_unknown_headers(&manifest_dir, "wasm32-unknown-unknown", true)
                .unwrap()
                .is_some()
        );
        assert_eq!(
            resolve_unknown_unknown_headers(&manifest_dir, "wasm32-unknown-unknown", false)
                .unwrap(),
            None
        );
        assert_eq!(
            resolve_unknown_unknown_headers(&manifest_dir, "wasm32-wasip1", true).unwrap(),
            None
        );
    }

    #[test]
    fn freestanding_headers_reject_additional_inventory() {
        let manifest = freestanding_manifest_fixture();
        let headers = manifest
            .path()
            .join("src/bindgen_headers/wasm32_unknown_unknown");
        fs::write(headers.join("stdint.h"), "/* ambient substitute */\n").unwrap();

        let error = validate_unknown_unknown_headers(manifest.path()).unwrap_err();
        assert!(error.contains("exactly one regular file"), "{error}");
    }

    #[cfg(unix)]
    #[test]
    fn freestanding_headers_reject_symlinked_math_header() {
        use std::os::unix::fs::symlink;

        let manifest = freestanding_manifest_fixture();
        let math_header = manifest
            .path()
            .join("src/bindgen_headers/wasm32_unknown_unknown/math.h");
        fs::remove_file(&math_header).unwrap();
        symlink(
            boxdd_sys_manifest_dir().join("src/bindgen_headers/wasm32_unknown_unknown/math.h"),
            &math_header,
        )
        .unwrap();

        let error = validate_unknown_unknown_headers(manifest.path()).unwrap_err();
        assert!(error.contains("regular non-symlink file"), "{error}");
    }

    #[test]
    fn public_math_calls_and_freestanding_declarations_match_exactly() {
        let manifest_dir = boxdd_sys_manifest_dir();
        let public_headers_root = manifest_dir.join("third-party/box2d/include/box2d");
        let mut public_headers = Vec::new();
        collect_public_headers(&public_headers_root, &mut public_headers);
        public_headers.sort();
        let shim = fs::read_to_string(
            manifest_dir.join("src/bindgen_headers/wasm32_unknown_unknown/math.h"),
        )
        .unwrap();
        let expected = ["nextafterf", "remainderf", "sqrtf"]
            .into_iter()
            .map(str::to_owned)
            .collect::<BTreeSet<_>>();
        let public_calls = public_headers
            .into_iter()
            .flat_map(|path| callable_identifiers(&fs::read_to_string(path).unwrap()))
            .filter(|identifier| is_c17_math_callable(identifier))
            .collect::<BTreeSet<_>>();
        let shim_declarations = callable_identifiers(&shim)
            .into_iter()
            .filter(|identifier| is_c17_math_callable(identifier))
            .collect::<BTreeSet<_>>();

        assert_eq!(public_calls, expected);
        assert_eq!(shim_declarations, expected);
    }

    #[test]
    fn wasi_sysroot_rejects_missing_math_header_and_identity_drift() {
        let directory = tempfile::tempdir().unwrap();
        let headers = directory.path().join("include/wasm32-wasip1");
        fs::create_dir_all(&headers).unwrap();
        assert!(validate_wasi_sysroot(directory.path()).is_err());

        fs::write(headers.join("math.h"), "/* unpinned */\n").unwrap();
        let error = validate_wasi_sysroot(directory.path()).unwrap_err();
        assert!(error.contains("identity mismatch"), "{error}");
    }

    #[cfg(unix)]
    #[test]
    fn wasi_sysroot_rejects_symlink_escape() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let headers = directory.path().join("include/wasm32-wasip1");
        fs::create_dir_all(&headers).unwrap();
        fs::write(outside.path().join("math.h"), "/* outside */\n").unwrap();
        symlink(outside.path().join("math.h"), headers.join("math.h")).unwrap();

        let error = validate_wasi_sysroot(directory.path()).unwrap_err();
        assert!(error.contains("escapes"), "{error}");
    }

    #[test]
    fn installed_wasi_libc_32_matches_the_pinned_identity_when_available() {
        let installed = Path::new("/opt/homebrew/opt/wasi-libc/share/wasi-sysroot");
        if !installed.exists() {
            return;
        }

        let validated = validate_wasi_sysroot(installed).unwrap();
        assert_eq!(validated.identity_sha256(), WASI_LIBC_HEADERS_SHA256);
    }

    fn callable_identifiers(source: &str) -> BTreeSet<String> {
        let source = strip_c_comments_and_literals(source);
        let bytes = source.as_bytes();
        let mut identifiers = BTreeSet::new();
        let mut index = 0;
        while index < bytes.len() {
            if !is_identifier_start(bytes[index]) {
                index += 1;
                continue;
            }
            let start = index;
            index += 1;
            while index < bytes.len() && is_identifier_continue(bytes[index]) {
                index += 1;
            }
            let identifier = &source[start..index];
            let mut next = index;
            while next < bytes.len() && bytes[next].is_ascii_whitespace() {
                next += 1;
            }
            if bytes.get(next) == Some(&b'(') {
                identifiers.insert(identifier.to_owned());
            }
        }
        identifiers
    }

    fn boxdd_sys_manifest_dir() -> std::path::PathBuf {
        let current = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        if current.join("src/bindgen_contract.rs").is_file() {
            current
        } else {
            current
                .parent()
                .expect("xtask and boxdd-sys have a workspace parent")
                .join("boxdd-sys")
        }
    }

    fn freestanding_manifest_fixture() -> tempfile::TempDir {
        let manifest = tempfile::tempdir().unwrap();
        let headers = manifest
            .path()
            .join("src/bindgen_headers/wasm32_unknown_unknown");
        fs::create_dir_all(&headers).unwrap();
        fs::copy(
            boxdd_sys_manifest_dir().join("src/bindgen_headers/wasm32_unknown_unknown/math.h"),
            headers.join("math.h"),
        )
        .unwrap();
        manifest
    }

    fn collect_public_headers(directory: &Path, headers: &mut Vec<std::path::PathBuf>) {
        let mut entries = fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                collect_public_headers(&path, headers);
            } else if path.extension() == Some(OsStr::new("h")) {
                headers.push(path);
            }
        }
    }

    fn strip_c_comments_and_literals(source: &str) -> String {
        #[derive(Clone, Copy)]
        enum State {
            Code,
            LineComment,
            BlockComment,
            String(u8),
        }

        let bytes = source.as_bytes();
        let mut output = vec![b' '; bytes.len()];
        let mut state = State::Code;
        let mut index = 0;
        while index < bytes.len() {
            match state {
                State::Code if bytes[index..].starts_with(b"//") => {
                    state = State::LineComment;
                    index += 2;
                }
                State::Code if bytes[index..].starts_with(b"/*") => {
                    state = State::BlockComment;
                    index += 2;
                }
                State::Code if matches!(bytes[index], b'\"' | b'\'') => {
                    state = State::String(bytes[index]);
                    index += 1;
                }
                State::Code => {
                    output[index] = bytes[index];
                    index += 1;
                }
                State::LineComment if bytes[index] == b'\n' => {
                    output[index] = b'\n';
                    state = State::Code;
                    index += 1;
                }
                State::LineComment => index += 1,
                State::BlockComment if bytes[index..].starts_with(b"*/") => {
                    state = State::Code;
                    index += 2;
                }
                State::BlockComment => index += 1,
                State::String(_) if bytes[index] == b'\\' => {
                    index = (index + 2).min(bytes.len());
                }
                State::String(delimiter) if bytes[index] == delimiter => {
                    state = State::Code;
                    index += 1;
                }
                State::String(_) => index += 1,
            }
        }
        String::from_utf8(output).unwrap()
    }

    fn is_identifier_start(byte: u8) -> bool {
        byte == b'_' || byte.is_ascii_alphabetic()
    }

    fn is_identifier_continue(byte: u8) -> bool {
        is_identifier_start(byte) || byte.is_ascii_digit()
    }

    fn is_c17_math_callable(identifier: &str) -> bool {
        let base = identifier
            .strip_suffix('f')
            .or_else(|| identifier.strip_suffix('l'))
            .unwrap_or(identifier);
        matches!(
            base,
            "acos"
                | "acosh"
                | "asin"
                | "asinh"
                | "atan"
                | "atan2"
                | "atanh"
                | "cbrt"
                | "ceil"
                | "copysign"
                | "cos"
                | "cosh"
                | "erf"
                | "erfc"
                | "exp"
                | "exp2"
                | "expm1"
                | "fabs"
                | "fdim"
                | "floor"
                | "fma"
                | "fmax"
                | "fmin"
                | "fmod"
                | "frexp"
                | "hypot"
                | "ilogb"
                | "ldexp"
                | "lgamma"
                | "llrint"
                | "llround"
                | "log"
                | "log10"
                | "log1p"
                | "log2"
                | "logb"
                | "lrint"
                | "lround"
                | "modf"
                | "nan"
                | "nearbyint"
                | "nextafter"
                | "nexttoward"
                | "pow"
                | "remainder"
                | "remquo"
                | "rint"
                | "round"
                | "scalbln"
                | "scalbn"
                | "sin"
                | "sinh"
                | "sqrt"
                | "tan"
                | "tanh"
                | "tgamma"
                | "trunc"
        )
    }
}
