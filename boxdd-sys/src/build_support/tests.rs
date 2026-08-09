use super::{
    COSIGN_VERSION, PrebuiltProvenance, VerifiedFileSnapshot,
    atomic_publish::{generate_file_create_new, publish_verified_file, snapshot_file_create_new},
    cosign_verify_blob_args, cosign_version_is_qualified,
    provider_selection::{
        ProviderInputs, parse_optional_bool, parse_optional_unicode, select_provider,
        validate_force_bindgen_policy, validate_skip_cc_policy,
    },
    release_tag_matches_version,
    target::{BindingTargetFamily, classify_binding_target, simd_identity},
    workspace_release_tag,
};
use crate::provider_catalog::ProviderCapability as ProviderAdapter;
use sha2::{Digest as _, Sha256};
use std::{ffi::OsStr, fs, io::Write as _, path::Path};

fn native_inputs() -> ProviderInputs<'static> {
    ProviderInputs {
        target_arch: "x86_64",
        target_os: "linux",
        explicit_provider: None,
        has_system_dir: false,
        has_system_manifest: false,
        has_prebuilt_manifest: false,
        has_prebuilt_provenance: false,
        has_prebuilt_bundle: false,
        has_prebuilt_trusted_root: false,
        build_from_source_enabled: true,
        link_kind: None,
    }
}

#[test]
fn release_tags_match_the_exact_workspace_or_sys_forms() {
    assert_eq!(workspace_release_tag("0.6.0"), "v0.6.0");
    assert!(release_tag_matches_version("0.6.0", "v0.6.0"));
    assert!(release_tag_matches_version("0.6.0", "boxdd-sys-v0.6.0"));
    assert!(!release_tag_matches_version("", "v"));
    for tag in ["0.6.0", "v0.6.1", "boxdd-v0.6.0", "v0.6.0-extra", "main"] {
        assert!(!release_tag_matches_version("0.6.0", tag), "{tag}");
    }
}

#[test]
fn vendored_is_the_only_implicit_native_default() {
    assert_eq!(
        select_provider(native_inputs()).unwrap(),
        ProviderAdapter::Vendored
    );
}

#[test]
fn external_native_adapters_require_an_explicit_selector() {
    let mut system = native_inputs();
    system.has_system_dir = true;
    system.has_system_manifest = true;
    system.link_kind = Some("static");
    assert!(select_provider(system).is_err());

    let mut prebuilt = native_inputs();
    prebuilt.has_prebuilt_manifest = true;
    prebuilt.has_prebuilt_provenance = true;
    prebuilt.has_prebuilt_bundle = true;
    prebuilt.link_kind = Some("static");
    assert!(select_provider(prebuilt).is_err());
}

#[test]
fn external_native_adapters_require_complete_static_inputs() {
    let mut system = native_inputs();
    system.explicit_provider = Some("system");
    assert!(select_provider(system).is_err());
    system.has_system_dir = true;
    system.has_system_manifest = true;
    system.link_kind = Some("static");
    assert_eq!(select_provider(system).unwrap(), ProviderAdapter::System);
    system.link_kind = Some("dylib");
    assert!(select_provider(system).is_err());

    let mut prebuilt = native_inputs();
    prebuilt.explicit_provider = Some("prebuilt");
    prebuilt.has_prebuilt_manifest = true;
    assert!(select_provider(prebuilt).is_err());
    prebuilt.has_prebuilt_provenance = true;
    assert!(select_provider(prebuilt).is_err());
    prebuilt.has_prebuilt_bundle = true;
    assert_eq!(
        select_provider(prebuilt).unwrap(),
        ProviderAdapter::Prebuilt
    );

    let mut root_override_only = native_inputs();
    root_override_only.has_prebuilt_trusted_root = true;
    assert!(select_provider(root_override_only).is_err());
}

#[test]
fn multiple_provider_signals_fail_closed() {
    let mut inputs = native_inputs();
    inputs.has_system_dir = true;
    inputs.has_system_manifest = true;
    inputs.has_prebuilt_manifest = true;
    inputs.has_prebuilt_provenance = true;
    inputs.has_prebuilt_bundle = true;
    inputs.has_prebuilt_trusted_root = true;
    assert!(select_provider(inputs).is_err());
}

#[test]
fn wasm_selection_is_explicit_and_native_inputs_are_rejected() {
    let mut inputs = native_inputs();
    inputs.target_arch = "wasm32";
    inputs.target_os = "unknown";
    assert_eq!(
        select_provider(inputs).unwrap(),
        ProviderAdapter::WasmCompileOnly
    );

    inputs.explicit_provider = Some("wasm-provider");
    assert_eq!(
        select_provider(inputs).unwrap(),
        ProviderAdapter::WasmProvider
    );
    for target_os in ["wasi", "emscripten"] {
        inputs.target_os = target_os;
        assert!(select_provider(inputs).is_err());
    }
    inputs.explicit_provider = None;
    assert_eq!(
        select_provider(inputs).unwrap(),
        ProviderAdapter::WasmCompileOnly
    );
    inputs.explicit_provider = Some("wasm-source");
    assert!(select_provider(inputs).is_err());
    inputs.explicit_provider = None;
    inputs.has_system_dir = true;
    assert!(select_provider(inputs).is_err());
}

#[test]
fn checked_in_bindings_are_selected_by_exact_target_family_and_precision() {
    let native =
        classify_binding_target("x86_64-unknown-linux-gnu", "unix", "x86_64", "linux", "gnu")
            .unwrap();
    assert_eq!(native, BindingTargetFamily::Native);
    assert_eq!(
        native.pregenerated_bindings_file(false),
        "bindings_pregenerated.rs"
    );
    assert_eq!(
        native.pregenerated_bindings_file(true),
        "bindings_double.rs"
    );

    let unknown =
        classify_binding_target("wasm32-unknown-unknown", "wasm", "wasm32", "unknown", "").unwrap();
    assert_eq!(unknown, BindingTargetFamily::WasmUnknownUnknown);
    assert_eq!(
        unknown.pregenerated_bindings_file(false),
        "bindings_wasm32_unknown_unknown.rs"
    );
    assert_eq!(
        unknown.pregenerated_bindings_file(true),
        "bindings_wasm32_unknown_unknown_double.rs"
    );

    let wasip1 = classify_binding_target("wasm32-wasip1", "wasm", "wasm32", "wasi", "p1").unwrap();
    assert_eq!(wasip1, BindingTargetFamily::WasmWasiP1);
    assert_eq!(
        wasip1.pregenerated_bindings_file(false),
        "bindings_wasm32_wasip1.rs"
    );
    assert_eq!(
        wasip1.pregenerated_bindings_file(true),
        "bindings_wasm32_wasip1_double.rs"
    );
}

#[test]
fn unsupported_or_inconsistent_wasm_targets_fail_closed() {
    for (target, target_family, target_arch, target_os, target_env) in [
        ("wasm32-wasip2", "wasm", "wasm32", "wasi", "p2"),
        (
            "wasm32-unknown-emscripten",
            "wasm",
            "wasm32",
            "emscripten",
            "",
        ),
        ("custom-wasm32", "wasm", "wasm32", "unknown", ""),
        ("wasm32-wasip1", "wasm", "wasm32", "wasi", "p2"),
        ("wasm32-unknown-unknown", "wasm", "wasm32", "wasi", "p1"),
        ("wasm64-unknown-unknown", "wasm", "wasm64", "unknown", ""),
        ("wasm32-wasip1", "unix,wasm", "wasm32", "wasi", "p1"),
        ("wasm32-wasip1", "", "wasm32", "wasi", "p1"),
    ] {
        assert!(
            classify_binding_target(target, target_family, target_arch, target_os, target_env)
                .is_err(),
            "unexpectedly accepted target={target:?}, target_family={target_family:?}, target_arch={target_arch:?}, target_os={target_os:?}, target_env={target_env:?}"
        );
    }
}

#[test]
fn prebuilt_provenance_binds_repository_workflow_tag_and_commit() {
    assert_eq!(COSIGN_VERSION, "v3.0.6");
    assert!(cosign_version_is_qualified("GitVersion: v3.0.6"));
    assert!(!cosign_version_is_qualified("GitVersion: v3.0.60"));
    let commit = "1234567890abcdef1234567890abcdef12345678";
    let args = cosign_verify_blob_args(PrebuiltProvenance {
        crate_version: "0.6.0",
        source_commit: commit,
        release_tag: "v0.6.0",
        payload: Path::new("artifact.toml"),
        bundle: Path::new("artifact.sigstore.json"),
        trusted_root: Path::new("trusted-root.json"),
    })
    .unwrap();
    let args = args
        .iter()
        .map(|arg| arg.to_string_lossy())
        .collect::<Vec<_>>();
    assert!(args.iter().any(|arg| {
        arg.as_ref()
            == "https://github.com/Latias94/boxdd/.github/workflows/prebuilt-binaries.yml@refs/tags/v0.6.0"
    }));
    assert!(args.iter().any(|arg| arg.as_ref() == commit));
    assert!(args.windows(2).any(|pair| {
        pair[0].as_ref() == "--certificate-github-workflow-trigger" && pair[1].as_ref() == "push"
    }));
    assert!(args.windows(2).any(|pair| {
        pair[0].as_ref() == "--certificate-github-workflow-name"
            && pair[1].as_ref() == "Build Prebuilt Binaries (boxdd-sys)"
    }));
    assert!(
        cosign_verify_blob_args(PrebuiltProvenance {
            crate_version: "0.6.0",
            source_commit: commit,
            release_tag: "main",
            payload: Path::new("artifact.toml"),
            bundle: Path::new("artifact.sigstore.json"),
            trusted_root: Path::new("trusted-root.json"),
        })
        .is_err()
    );
}

#[test]
fn skip_cc_is_fail_closed_to_bindgen_fixture_only() {
    assert!(validate_skip_cc_policy(false, false, false, ProviderAdapter::Vendored).is_ok());
    assert!(validate_skip_cc_policy(true, true, false, ProviderAdapter::Vendored).is_ok());
    assert!(validate_skip_cc_policy(false, true, true, ProviderAdapter::Vendored).is_ok());
    assert!(validate_skip_cc_policy(false, true, true, ProviderAdapter::WasmCompileOnly).is_ok());
    for provider in [
        ProviderAdapter::System,
        ProviderAdapter::Prebuilt,
        ProviderAdapter::WasmProvider,
    ] {
        assert!(validate_skip_cc_policy(false, true, true, provider).is_err());
    }
    assert!(validate_skip_cc_policy(false, true, false, ProviderAdapter::Vendored).is_err());
    assert!(validate_skip_cc_policy(false, true, false, ProviderAdapter::WasmCompileOnly).is_err());
}

#[test]
fn force_bindgen_cannot_split_an_authenticated_provider_identity() {
    for provider in [
        ProviderAdapter::System,
        ProviderAdapter::Prebuilt,
        ProviderAdapter::WasmProvider,
    ] {
        let error = validate_force_bindgen_policy(true, provider)
            .expect_err("authenticated providers must consume checked bindings");
        assert!(error.contains(provider.as_str()), "{error}");
    }
    for provider in [ProviderAdapter::Vendored, ProviderAdapter::WasmCompileOnly] {
        assert!(validate_force_bindgen_policy(true, provider).is_ok());
    }
    assert!(validate_force_bindgen_policy(false, ProviderAdapter::Prebuilt).is_ok());
}

#[test]
fn optional_build_booleans_are_strict_and_name_invalid_inputs() {
    for value in ["1", "true", "yes", "on"] {
        assert!(parse_optional_bool("BOXDD_TEST_BOOL", Some(OsStr::new(value))).unwrap());
    }
    for value in ["0", "false", "no", "off"] {
        assert!(!parse_optional_bool("BOXDD_TEST_BOOL", Some(OsStr::new(value))).unwrap());
    }
    assert!(!parse_optional_bool("BOXDD_TEST_BOOL", None).unwrap());

    for value in ["", "TRUE", "False", "2", "enabled"] {
        let error = parse_optional_bool("BOXDD_TEST_BOOL", Some(OsStr::new(value)))
            .expect_err("unsupported boolean spelling must fail");
        assert!(error.contains("BOXDD_TEST_BOOL"), "{error}");
        assert!(error.contains(&format!("{value:?}")), "{error}");
    }

    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt as _;

        let error = parse_optional_bool("BOXDD_TEST_BOOL", Some(OsStr::from_bytes(&[0xff])))
            .expect_err("non-Unicode boolean must fail");
        assert!(error.contains("BOXDD_TEST_BOOL"), "{error}");
        assert!(error.contains("Unicode"), "{error}");
    }
}

#[test]
fn optional_provider_settings_reject_non_unicode_without_changing_absence_semantics() {
    assert_eq!(
        parse_optional_unicode("BOXDD_SYS_PROVIDER", None).unwrap(),
        None
    );
    assert_eq!(
        parse_optional_unicode("BOXDD_SYS_PROVIDER", Some(OsStr::new("prebuilt"))).unwrap(),
        Some("prebuilt")
    );
    assert_eq!(
        parse_optional_unicode("BOXDD_SYS_LINK_KIND", Some(OsStr::new("static"))).unwrap(),
        Some("static")
    );

    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt as _;

        for key in ["BOXDD_SYS_PROVIDER", "BOXDD_SYS_LINK_KIND"] {
            let error = parse_optional_unicode(key, Some(OsStr::from_bytes(&[0xff])))
                .expect_err("non-Unicode provider settings must fail closed");
            assert!(error.contains(key), "{error}");
            assert!(error.contains("Unicode"), "{error}");
        }
    }
}

#[test]
fn verified_file_snapshot_binds_one_bounded_byte_sequence() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("input.bin");
    let bytes = b"authenticated input";
    fs::write(&path, bytes).unwrap();

    let error = VerifiedFileSnapshot::read(&path, bytes.len() as u64 - 1, "test input")
        .expect_err("an oversized input must fail before it is accepted");
    assert!(error.contains("byte limit"), "{error}");

    let snapshot = VerifiedFileSnapshot::read(&path, bytes.len() as u64, "test input").unwrap();
    assert_eq!(snapshot.path(), path);
    assert_eq!(snapshot.bytes(), bytes);
    assert_eq!(snapshot.len(), bytes.len());
    assert_eq!(snapshot.sha256(), format!("{:x}", Sha256::digest(bytes)));
    snapshot
        .verify_exact(bytes, snapshot.sha256(), "test input")
        .unwrap();
    snapshot.revalidate("test input").unwrap();

    let error = snapshot
        .verify_exact(b"different input", snapshot.sha256(), "test input")
        .expect_err("a digest match cannot substitute for exact-byte equality");
    assert!(error.contains("exact bytes"), "{error}");

    fs::write(&path, b"tampered file data!").unwrap();
    let error = snapshot
        .revalidate("test input")
        .expect_err("a retained snapshot must reject later path drift");
    assert!(error.contains("SHA-256"), "{error}");
}

#[test]
fn create_new_snapshot_retains_exact_bytes_and_refuses_reuse() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source.bin");
    let destination = directory.path().join("private.bin");
    fs::write(&source, b"authenticated input").unwrap();

    let snapshot = snapshot_file_create_new(&source, &destination, 64, "test input").unwrap();
    assert_eq!(snapshot.bytes(), b"authenticated input");
    snapshot.revalidate("test input").unwrap();
    assert!(snapshot_file_create_new(&source, &destination, 64, "test input").is_err());
    assert!(
        snapshot_file_create_new(
            &source,
            &directory.path().join("too-small.bin"),
            4,
            "test input",
        )
        .is_err()
    );
}

#[test]
fn generated_file_publication_refuses_replacement() {
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("package.tar.gz");
    let snapshot = generate_file_create_new(&destination, 64, "test package", |output| {
        output
            .write_all(b"generated package")
            .map_err(|error| error.to_string())
    })
    .unwrap();
    assert_eq!(snapshot.path(), destination);
    assert_eq!(snapshot.bytes(), b"generated package");
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);

    let error = generate_file_create_new(&destination, 64, "test package", |output| {
        output
            .write_all(b"replacement")
            .map_err(|error| error.to_string())
    })
    .expect_err("an existing package must never be replaced");
    assert!(error.contains("refusing to replace"), "{error}");
    assert_eq!(fs::read(&destination).unwrap(), b"generated package");
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn generated_file_publication_does_not_follow_destination_symlinks() {
    use std::os::unix::fs::symlink;

    let directory = tempfile::tempdir().unwrap();
    let victim = directory.path().join("victim");
    let destination = directory.path().join("package.tar.gz");
    fs::write(&victim, b"retained").unwrap();
    symlink(&victim, &destination).unwrap();

    assert!(
        generate_file_create_new(&destination, 64, "test package", |output| {
            output
                .write_all(b"replacement")
                .map_err(|error| error.to_string())
        })
        .is_err()
    );
    assert_eq!(fs::read(&victim).unwrap(), b"retained");
    assert!(
        fs::symlink_metadata(&destination)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
}

#[cfg(unix)]
#[test]
fn verified_file_snapshot_rejects_symlinks() {
    use std::os::unix::fs::symlink;

    let directory = tempfile::tempdir().unwrap();
    let original = directory.path().join("original.bin");
    let link = directory.path().join("link.bin");
    fs::write(&original, b"original").unwrap();
    symlink(&original, &link).unwrap();

    let error = VerifiedFileSnapshot::read(&link, 64, "test input")
        .expect_err("a symlink input must fail closed");
    assert!(error.contains("non-symlink"), "{error}");
}

#[test]
fn verified_file_publication_is_idempotent_and_rejects_other_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("snapshot");
    let bytes = b"complete authenticated bytes";
    let digest = format!("{:x}", Sha256::digest(bytes));

    publish_verified_file(&destination, &digest, bytes, "test snapshot").unwrap();
    assert_eq!(fs::read(&destination).unwrap(), bytes);
    publish_verified_file(&destination, &digest, bytes, "test snapshot").unwrap();
    let error = publish_verified_file(&destination, &digest, b"different bytes", "test snapshot")
        .expect_err("bytes cannot be published under another digest");
    assert!(error.contains("SHA-256"), "{error}");
    assert_eq!(fs::read(&destination).unwrap(), bytes);
}

#[test]
fn verified_file_publication_recovers_from_an_incomplete_destination() {
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("snapshot");
    let bytes = b"complete authenticated bytes";
    let digest = format!("{:x}", Sha256::digest(bytes));

    fs::write(&destination, b"interrupted partial write").unwrap();
    publish_verified_file(&destination, &digest, bytes, "test snapshot").unwrap();

    assert_eq!(fs::read(&destination).unwrap(), bytes);
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[cfg(windows)]
#[test]
fn verified_file_publication_keeps_windows_temporary_paths_short() {
    use std::os::windows::ffi::OsStrExt as _;

    fn wide_len(path: &Path) -> usize {
        path.as_os_str().encode_wide().count()
    }

    let directory = tempfile::tempdir().unwrap();
    let mut parent = directory.path().to_path_buf();
    let target_parent_len = 165;
    let component_len = target_parent_len
        .checked_sub(wide_len(&parent) + 1)
        .expect("Windows temporary root is too long for the MAX_PATH regression fixture");
    parent.push("x".repeat(component_len));
    fs::create_dir(&parent).unwrap();

    let bytes = b"complete authenticated bytes";
    let digest = format!("{:x}", Sha256::digest(bytes));
    let destination = parent.join(format!("boxdd-bindings-{digest}.rs"));
    let legacy_temporary = parent.join(format!(
        ".{}.boxdd-tmp-XXXXXX",
        destination.file_name().unwrap().to_string_lossy()
    ));
    assert!(wide_len(&destination) < 260);
    assert!(wide_len(&legacy_temporary) >= 260);

    publish_verified_file(&destination, &digest, bytes, "test bindings").unwrap();
    assert_eq!(fs::read(destination).unwrap(), bytes);
}

#[test]
fn concurrent_verified_file_publishers_converge_on_complete_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("snapshot");
    let bytes = b"complete authenticated bytes";
    let digest = format!("{:x}", Sha256::digest(bytes));
    let barrier = std::sync::Barrier::new(8);
    fs::write(&destination, b"interrupted partial write").unwrap();

    std::thread::scope(|scope| {
        let mut workers = Vec::new();
        for _ in 0..8 {
            workers.push(scope.spawn(|| {
                barrier.wait();
                publish_verified_file(&destination, &digest, bytes, "test snapshot")
            }));
        }
        for worker in workers {
            worker.join().unwrap().unwrap();
        }
    });

    assert_eq!(fs::read(&destination).unwrap(), bytes);
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn simd_identity_matches_the_actual_target_compiler_policy() {
    assert_eq!(simd_identity("wasm32", false, true), "disabled");
    assert_eq!(simd_identity("x86_64", false, true), "avx2");
    assert_eq!(simd_identity("aarch64", false, true), "default");
    assert_eq!(simd_identity("x86_64", true, true), "disabled");
}
