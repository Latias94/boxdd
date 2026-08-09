//! Discovery for the Emscripten tools used by repository WASM commands.

use std::{
    env,
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    isolated_git::{remove_matching_environment, remove_process_injection_environment},
    subprocess_policy::run_output,
};

pub(crate) const EMSCRIPTEN_VERSION: &str = "6.0.4";
pub(crate) const NODE_VERSION: &str = "22.16.0";

#[derive(Debug)]
pub(crate) struct EmscriptenTools {
    emcc: PathBuf,
    node: PathBuf,
    npm: PathBuf,
    wasm_opt: PathBuf,
}

impl EmscriptenTools {
    pub(crate) fn discover() -> Result<Self, String> {
        let emsdk = env::var_os("EMSDK")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        let emcc = resolve_tool(
            env::var_os("EMCC").filter(|value| !value.is_empty()),
            emsdk.as_deref().map(emsdk_emcc),
            &tool_names("emcc", "emcc.bat"),
            "Emscripten compiler",
        )?;
        let node = resolve_tool(
            env::var_os("EMSDK_NODE").filter(|value| !value.is_empty()),
            None,
            &tool_names("node", "node.exe"),
            "Node.js",
        )?;
        let npm = resolve_tool(None, None, &tool_names("npm", "npm.cmd"), "npm")?;
        let wasm_opt = resolve_tool(
            env::var_os("WASM_OPT").filter(|value| !value.is_empty()),
            emsdk.as_deref().map(emsdk_wasm_opt),
            &tool_names("wasm-opt", "wasm-opt.exe"),
            "wasm-opt",
        )?;

        let tools = Self {
            emcc,
            node,
            npm,
            wasm_opt,
        };
        tools.validate()?;
        Ok(tools)
    }

    pub(crate) fn emcc_command(&self) -> Result<Command, String> {
        let mut command = tool_command(&self.emcc, "Emscripten compiler")?;
        remove_matching_environment(&mut command, is_emcc_injection_environment);
        remove_node_injection_environment(&mut command);
        Ok(command)
    }

    pub(crate) fn node_command(&self) -> Result<Command, String> {
        let mut command = tool_command(&self.node, "Node.js")?;
        remove_node_injection_environment(&mut command);
        Ok(command)
    }

    pub(crate) fn npm_command(&self) -> Result<Command, String> {
        let mut command = tool_command(&self.npm, "npm")?;
        remove_node_injection_environment(&mut command);
        Ok(command)
    }

    pub(crate) fn wasm_opt_command(&self) -> Result<Command, String> {
        let mut command = tool_command(&self.wasm_opt, "wasm-opt")?;
        remove_matching_environment(&mut command, |key| {
            key.to_string_lossy()
                .to_ascii_uppercase()
                .starts_with("BINARYEN_")
        });
        Ok(command)
    }

    pub(crate) fn version(&self) -> &'static str {
        EMSCRIPTEN_VERSION
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        validate_emcc(&self.emcc)?;
        validate_exact_version(&self.node, "Node.js", &format!("v{NODE_VERSION}"))?;
        validate_available(&self.npm, "npm")?;
        validate_available(&self.wasm_opt, "wasm-opt")
    }

    pub(crate) fn revalidate(&self) -> Result<(), String> {
        for (path, label) in [
            (&self.emcc, "Emscripten compiler"),
            (&self.node, "Node.js"),
            (&self.npm, "npm"),
            (&self.wasm_opt, "wasm-opt"),
        ] {
            require_executable_file(path, label)?;
        }
        Ok(())
    }
}

fn emsdk_emcc(root: &Path) -> PathBuf {
    root.join("upstream")
        .join("emscripten")
        .join(if cfg!(windows) { "emcc.bat" } else { "emcc" })
}

fn emsdk_wasm_opt(root: &Path) -> PathBuf {
    root.join("upstream").join("bin").join(if cfg!(windows) {
        "wasm-opt.exe"
    } else {
        "wasm-opt"
    })
}

const fn tool_names(unix: &'static str, windows: &'static str) -> [&'static str; 1] {
    if cfg!(windows) { [windows] } else { [unix] }
}

fn resolve_tool(
    explicit: Option<OsString>,
    sdk_candidate: Option<PathBuf>,
    names: &[&str],
    label: &str,
) -> Result<PathBuf, String> {
    if let Some(path) = explicit {
        return resolve_explicit_tool(Path::new(&path), label);
    }
    if let Some(path) = sdk_candidate.filter(|path| path.is_file()) {
        return canonical_tool(&path, label);
    }
    find_on_path(names).ok_or_else(|| {
        format!(
            "{label} was not found; install and activate Emscripten {EMSCRIPTEN_VERSION}, then ensure {} is on PATH",
            names.join(" or ")
        )
    })
}

fn resolve_explicit_tool(path: &Path, label: &str) -> Result<PathBuf, String> {
    if path.components().count() == 1 {
        let name = path.to_string_lossy();
        return find_on_path(&[name.as_ref()])
            .ok_or_else(|| format!("{label} override was not found on PATH: {}", path.display()));
    }
    canonical_tool(path, label)
}

fn find_on_path(names: &[&str]) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .flat_map(|directory| names.iter().map(move |name| directory.join(name)))
        .find_map(|candidate| canonical_tool(&candidate, "tool").ok())
}

fn canonical_tool(path: &Path, label: &str) -> Result<PathBuf, String> {
    require_executable_file(path, label)?;
    fs::canonicalize(path)
        .map_err(|error| format!("failed to resolve {label} {}: {error}", path.display()))
}

fn require_executable_file(path: &Path, label: &str) -> Result<(), String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("failed to inspect {label} {}: {error}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!("{label} is not a file: {}", path.display()));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;

        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(format!("{label} is not executable: {}", path.display()));
        }
    }
    Ok(())
}

fn tool_command(path: &Path, label: &str) -> Result<Command, String> {
    require_executable_file(path, label)?;
    let mut command = Command::new(path);
    remove_process_injection_environment(&mut command);
    Ok(command)
}

fn remove_node_injection_environment(command: &mut Command) {
    for key in ["NODE_OPTIONS", "NODE_PATH"] {
        command.env_remove(key);
    }
}

fn is_emcc_injection_environment(key: &OsStr) -> bool {
    let key = key.to_string_lossy().to_ascii_uppercase();
    key.starts_with("EMCC_")
        || key.starts_with("EMMAKEN_")
        || matches!(
            key.as_str(),
            "EM_COMPILER_WRAPPER"
                | "CFLAGS"
                | "CPPFLAGS"
                | "CXXFLAGS"
                | "LDFLAGS"
                | "CC"
                | "CXX"
                | "AR"
                | "LD"
                | "RANLIB"
        )
}

fn validate_emcc(emcc: &Path) -> Result<(), String> {
    let output = version_output(emcc, "Emscripten compiler")?;
    let first_line = output.lines().next().unwrap_or_default();
    if emcc_version_is_exact(first_line, EMSCRIPTEN_VERSION) {
        Ok(())
    } else {
        Err(format!(
            "repository WASM builds require Emscripten {EMSCRIPTEN_VERSION}; found {first_line:?}"
        ))
    }
}

fn emcc_version_is_exact(first_line: &str, expected: &str) -> bool {
    first_line
        .split_ascii_whitespace()
        .any(|token| token == expected)
}

fn validate_exact_version(tool: &Path, label: &str, expected: &str) -> Result<(), String> {
    let actual = version_output(tool, label)?;
    if actual.trim() == expected {
        Ok(())
    } else {
        Err(format!(
            "{label} must be {expected}; found {:?}",
            actual.trim()
        ))
    }
}

fn validate_available(tool: &Path, label: &str) -> Result<(), String> {
    version_output(tool, label).map(|_| ())
}

fn version_output(tool: &Path, label: &str) -> Result<String, String> {
    let mut command = tool_command(tool, label)?;
    remove_matching_environment(&mut command, is_emcc_injection_environment);
    remove_node_injection_environment(&mut command);
    remove_matching_environment(&mut command, |key| {
        key.to_string_lossy()
            .to_ascii_uppercase()
            .starts_with("BINARYEN_")
    });
    command.arg("--version");
    let output = run_output(&mut command, &format!("inspect {label} {}", tool.display()))?;
    if !output.status.success() {
        return Err(format!(
            "{label} --version failed with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("{label} --version output is not UTF-8: {error}"))?;
    if !stdout.trim().is_empty() {
        return Ok(stdout);
    }
    String::from_utf8(output.stderr)
        .map_err(|error| format!("{label} --version output is not UTF-8: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emsdk_candidates_follow_the_standard_layout() {
        let root = Path::new("emsdk");
        assert_eq!(
            emsdk_emcc(root),
            root.join("upstream")
                .join("emscripten")
                .join(if cfg!(windows) { "emcc.bat" } else { "emcc" })
        );
        assert_eq!(
            emsdk_wasm_opt(root),
            root.join("upstream").join("bin").join(if cfg!(windows) {
                "wasm-opt.exe"
            } else {
                "wasm-opt"
            })
        );
    }

    #[test]
    fn missing_explicit_tool_has_a_direct_error() {
        let missing = Path::new("definitely-missing-boxdd-tool/path");
        let error = resolve_explicit_tool(missing, "fixture").unwrap_err();
        assert!(error.contains("fixture"));
        assert!(error.contains(&missing.display().to_string()));
    }

    #[test]
    fn configured_versions_are_canonical() {
        assert_eq!(EMSCRIPTEN_VERSION.split('.').count(), 3);
        assert_eq!(NODE_VERSION.split('.').count(), 3);
        assert!(
            EMSCRIPTEN_VERSION
                .split('.')
                .all(|component| component.parse::<u64>().is_ok())
        );
        assert!(
            NODE_VERSION
                .split('.')
                .all(|component| component.parse::<u64>().is_ok())
        );
    }

    #[test]
    fn emcc_version_requires_an_exact_output_token() {
        let canonical = format!(
            "emcc (Emscripten gcc/clang-like replacement + linker emulating GNU ld) {EMSCRIPTEN_VERSION} (test)"
        );
        assert!(emcc_version_is_exact(&canonical, EMSCRIPTEN_VERSION));

        for rejected in ["6.0.40", "6.0.4-beta", "6.0.4+local"] {
            let output = format!("emcc (Emscripten) {rejected} (test)");
            assert!(!emcc_version_is_exact(&output, EMSCRIPTEN_VERSION));
        }
    }

    #[test]
    fn tool_commands_remove_output_and_process_injection_environment() {
        let mut emcc = Command::new("ignored");
        for key in [
            "EMCC_CFLAGS",
            "EM_COMPILER_WRAPPER",
            "EMMAKEN_CFLAGS",
            "LDFLAGS",
            "NODE_OPTIONS",
            "NODE_PATH",
        ] {
            emcc.env(key, "payload");
        }
        remove_matching_environment(&mut emcc, is_emcc_injection_environment);
        remove_node_injection_environment(&mut emcc);
        for key in [
            "EMCC_CFLAGS",
            "EM_COMPILER_WRAPPER",
            "EMMAKEN_CFLAGS",
            "LDFLAGS",
            "NODE_OPTIONS",
            "NODE_PATH",
        ] {
            assert!(
                emcc.get_envs()
                    .any(|(actual, value)| actual == OsStr::new(key) && value.is_none()),
                "{key} was not removed"
            );
        }

        let mut node = Command::new("ignored");
        node.env("NODE_OPTIONS", "--require=payload");
        remove_node_injection_environment(&mut node);
        assert!(
            node.get_envs()
                .any(|(actual, value)| { actual == OsStr::new("NODE_OPTIONS") && value.is_none() })
        );
    }
}
