use std::{
    collections::BTreeSet,
    env,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn main() {
    println!("cargo:rerun-if-env-changed=RUSTC");
    println!("cargo:rerun-if-env-changed=RUSTUP_HOME");
    println!("cargo:rerun-if-env-changed=RUSTUP_TOOLCHAIN");

    let rustc = PathBuf::from(
        env::var_os("RUSTC").expect("Cargo must provide RUSTC to the xtask build script"),
    );
    assert!(
        rustc.is_absolute(),
        "Cargo must provide an absolute RUSTC path to the xtask build script: {}",
        rustc.display()
    );
    let output = Command::new(&rustc)
        .args(["--print", "sysroot"])
        .output()
        .expect("the compiler building xtask must report its sysroot");
    assert!(
        output.status.success(),
        "the compiler building xtask failed to report its sysroot: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    let sysroot = String::from_utf8(output.stdout)
        .expect("the compiler building xtask must report a UTF-8 sysroot");
    let sysroot = PathBuf::from(sysroot.trim());
    assert!(sysroot.is_absolute(), "the Rust sysroot must be absolute");
    let sysroot = fs::canonicalize(&sysroot).expect("the Rust sysroot must be canonicalizable");

    for (environment, program) in [
        ("BOXDD_XTASK_CARGO", executable_name("cargo")),
        ("BOXDD_XTASK_RUSTC", executable_name("rustc")),
    ] {
        let path = sysroot.join("bin").join(program);
        let path = fs::canonicalize(&path).unwrap_or_else(|error| {
            panic!(
                "the compiler sysroot tool {} must be canonicalizable: {error}",
                path.display()
            )
        });
        let metadata = fs::symlink_metadata(&path).unwrap_or_else(|error| {
            panic!(
                "the compiler sysroot tool {} must be inspectable: {error}",
                path.display()
            )
        });
        assert!(
            metadata.file_type().is_file() && !metadata.file_type().is_symlink(),
            "the compiler sysroot tool must be a regular non-symlink file: {}",
            path.display()
        );
        let path = path
            .to_str()
            .expect("the compiler sysroot tool path must be UTF-8");
        println!("cargo:rustc-env={environment}={path}");
    }

    generate_provider_policy_sources();
}

fn executable_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    }
}

fn generate_provider_policy_sources() {
    let manifest_directory = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("Cargo must provide CARGO_MANIFEST_DIR"),
    );
    let manifest_directory = fs::canonicalize(&manifest_directory)
        .expect("the xtask manifest directory must be canonicalizable");
    let workspace = manifest_directory
        .parent()
        .expect("the xtask manifest directory must have a workspace parent");
    let mut sources = BTreeSet::new();
    for relative in [
        "Cargo.lock",
        "Cargo.toml",
        "rust-toolchain.toml",
        "boxdd-sys/Cargo.toml",
        "boxdd-sys/build.rs",
        "boxdd-sys/src/prebuilt_provenance.rs",
        "boxdd-sys/src/provider_archive.rs",
        "boxdd-sys/src/provider_manifest.rs",
        "boxdd-sys/src/source_overlay.rs",
        "boxdd-sys/src/wasm_provider_contract.rs",
        "examples-wasm/provider-smoke/provider-runtime-contract.mjs",
        "xtask/Cargo.toml",
        "xtask/build.rs",
    ] {
        insert_policy_source(workspace, Path::new(relative), &mut sources);
    }
    collect_policy_tree(workspace, Path::new("xtask/src"), &["rs"], &mut sources);
    collect_policy_tree(
        workspace,
        Path::new("xtask/toolchains"),
        &["toml"],
        &mut sources,
    );
    collect_policy_tree(
        workspace,
        Path::new("boxdd-sys/native"),
        &["c", "h", "inl"],
        &mut sources,
    );

    let mut generated =
        String::from("const COMPILED_GENERATOR_POLICY_SOURCES: &[(&str, &[u8])] = &[\n");
    for relative in sources {
        let absolute = workspace.join(&relative);
        let relative = relative
            .to_str()
            .expect("provider policy paths must be UTF-8")
            .replace('\\', "/");
        let absolute = absolute
            .to_str()
            .expect("provider policy source paths must be UTF-8");
        writeln!(
            generated,
            "    ({relative:?}, include_bytes!({absolute:?})),"
        )
        .expect("writing to a String cannot fail");
        println!("cargo:rerun-if-changed={absolute}");
    }
    generated.push_str("];\n");

    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo must provide OUT_DIR"))
        .join("provider_policy_sources.rs");
    fs::write(&output, generated).unwrap_or_else(|error| {
        panic!(
            "failed to write generated provider policy inventory {}: {error}",
            output.display()
        )
    });
}

fn collect_policy_tree(
    workspace: &Path,
    relative: &Path,
    extensions: &[&str],
    sources: &mut BTreeSet<PathBuf>,
) {
    let directory = workspace.join(relative);
    println!("cargo:rerun-if-changed={}", directory.display());
    let mut entries = fs::read_dir(&directory)
        .unwrap_or_else(|error| {
            panic!(
                "failed to read provider policy directory {}: {error}",
                directory.display()
            )
        })
        .map(|entry| entry.expect("provider policy directory entries must be readable"))
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).unwrap_or_else(|error| {
            panic!(
                "failed to inspect provider policy source {}: {error}",
                path.display()
            )
        });
        assert!(
            !metadata.file_type().is_symlink(),
            "provider policy source trees must not contain symlinks: {}",
            path.display()
        );
        let relative = path
            .strip_prefix(workspace)
            .expect("provider policy source must remain inside the workspace");
        if metadata.is_dir() {
            collect_policy_tree(workspace, relative, extensions, sources);
        } else if metadata.is_file()
            && path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extensions.contains(&extension))
        {
            insert_policy_source(workspace, relative, sources);
        }
    }
}

fn insert_policy_source(workspace: &Path, relative: &Path, sources: &mut BTreeSet<PathBuf>) {
    assert!(
        relative.is_relative()
            && relative
                .components()
                .all(|component| matches!(component, std::path::Component::Normal(_))),
        "provider policy source must be a normalized relative path: {}",
        relative.display()
    );
    let path = workspace.join(relative);
    let metadata = fs::symlink_metadata(&path).unwrap_or_else(|error| {
        panic!(
            "failed to inspect provider policy source {}: {error}",
            path.display()
        )
    });
    assert!(
        metadata.file_type().is_file() && !metadata.file_type().is_symlink(),
        "provider policy source must be a regular non-symlink file: {}",
        path.display()
    );
    assert!(
        sources.insert(relative.to_owned()),
        "provider policy source is duplicated: {}",
        relative.display()
    );
}
