use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[path = "src/build_support.rs"]
mod build_support;

#[allow(dead_code)]
#[path = "src/precision.rs"]
mod precision;

use build_support::validate_c_source_paths;
use precision::Precision;

const LEGACY_WASM_IMPORT_MODULE: &str = "box2d-sys-v0";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WasmMode {
    CompileOnly,
    Source,
    Provider,
}

#[derive(Debug)]
struct BuildConfig {
    manifest_dir: PathBuf,
    #[cfg_attr(not(feature = "bindgen"), allow(dead_code))]
    out_dir: PathBuf,
    target_arch: String,
    target_env: String,
    target_os: String,
    target: String,
    profile: String,
    is_docsrs: bool,
    skip_cc: bool,
    force_bindgen: bool,
    #[cfg_attr(not(feature = "bindgen"), allow(dead_code))]
    bindgen_target: String,
    wasm_mode: Option<WasmMode>,
    precision: Precision,
}

#[derive(Debug)]
struct UpstreamBuildManifest {
    active_revision: String,
    c_sources: Vec<PathBuf>,
}

impl BuildConfig {
    fn from_env() -> Self {
        let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
        let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
        let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
        let target = env::var("TARGET").expect("Cargo must provide TARGET");
        let is_docsrs = env::var("DOCS_RS").is_ok() || env::var("CARGO_CFG_DOCSRS").is_ok();
        let skip_cc = parse_bool_env("BOXDD_SYS_SKIP_CC");
        let force_bindgen = parse_bool_env("BOXDD_SYS_FORCE_BINDGEN");
        let bindgen_target =
            env::var("BOXDD_SYS_BINDGEN_TARGET").unwrap_or_else(|_| target.clone());
        let wasm_mode = (target_arch == "wasm32").then(|| {
            env::var("BOXDD_SYS_WASM_MODE")
                .ok()
                .map(|value| parse_wasm_mode(&value))
                .unwrap_or_else(|| default_wasm_mode(&target_env, &target_os))
        });

        Self {
            manifest_dir: PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()),
            out_dir: PathBuf::from(env::var("OUT_DIR").unwrap()),
            target_arch,
            target_env,
            target_os,
            target,
            profile: env::var("PROFILE").unwrap_or_else(|_| "release".into()),
            is_docsrs,
            skip_cc,
            force_bindgen,
            bindgen_target,
            wasm_mode,
            precision: Precision::ACTIVE,
        }
    }

    fn is_debug(&self) -> bool {
        self.profile == "debug"
    }

    fn pregenerated_bindings(&self) -> PathBuf {
        self.manifest_dir
            .join("src")
            .join(self.precision.pregenerated_bindings_file())
    }
}

fn parse_bool_env(key: &str) -> bool {
    match env::var(key) {
        Ok(v) => matches!(
            v.as_str(),
            "1" | "true" | "yes" | "on" | "TRUE" | "YES" | "ON"
        ),
        Err(_) => false,
    }
}

fn parse_wasm_mode(value: &str) -> WasmMode {
    match value {
        "compile-only" | "compile_only" | "check" => WasmMode::CompileOnly,
        "source" | "c-backed" | "c_backed" | "wasi" => WasmMode::Source,
        "provider" | "import-provider" | "import_provider" => WasmMode::Provider,
        other => panic!(
            "unsupported BOXDD_SYS_WASM_MODE={other:?}; expected compile-only, source, or provider"
        ),
    }
}

fn default_wasm_mode(target_env: &str, target_os: &str) -> WasmMode {
    if parse_bool_env("BOXDD_SYS_WASM_CC")
        || (target_env == "emscripten" && env::var_os("EMSDK").is_some())
        || (target_os == "wasi" && env::var_os("WASI_SDK_PATH").is_some())
    {
        WasmMode::Source
    } else {
        WasmMode::CompileOnly
    }
}

fn main() {
    println!("cargo:rustc-check-cfg=cfg(has_pregenerated)");
    println!("cargo:rustc-check-cfg=cfg(force_bindgen)");
    println!("cargo:rustc-check-cfg=cfg(boxdd_sys_wasm_provider)");
    println!("cargo:rustc-check-cfg=cfg(boxdd_sys_legacy_wasm_provider_bindings)");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/precision.rs");
    println!("cargo:rerun-if-changed=src/bindings_pregenerated.rs");
    println!("cargo:rerun-if-changed=src/bindings_double.rs");
    println!("cargo:rerun-if-changed=upstream.toml");
    println!("cargo:rerun-if-changed=third-party/box2d/include/box2d/box2d.h");
    println!("cargo:rerun-if-changed=third-party/box2d");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_SKIP_CC");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_WASM_CC");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_WASM_MODE");
    println!("cargo:rerun-if-env-changed=BOX2D_LIB_DIR");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_LINK_KIND");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_FORCE_BINDGEN");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_BINDGEN_TARGET");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_STRICT_FEATURES");
    println!("cargo:rerun-if-env-changed=EMSDK");
    println!("cargo:rerun-if-env-changed=WASI_SDK_PATH");
    println!("cargo:rerun-if-env-changed=WASI_SYSROOT");
    println!("cargo:rerun-if-env-changed=DOCS_RS");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_DOCSRS");

    let config = BuildConfig::from_env();
    reject_external_precision_overrides(&config.target);
    let upstream = load_upstream_manifest(&config.manifest_dir);
    let pregenerated = config.pregenerated_bindings();
    let has_pregenerated = pregenerated.exists();

    validate_build_config(&config);

    let legacy_wasm_provider = config.wasm_mode == Some(WasmMode::Provider)
        && !config.force_bindgen
        && has_pregenerated
        && needs_legacy_wasm_provider_bindings(&pregenerated, config.precision);
    let wasm_import_module = if legacy_wasm_provider {
        LEGACY_WASM_IMPORT_MODULE
    } else {
        config.precision.wasm_import_module()
    };
    emit_build_identity(&config, &upstream.active_revision, wasm_import_module);

    if config.force_bindgen {
        println!("cargo:rustc-cfg=force_bindgen");
    } else if has_pregenerated {
        println!("cargo:rustc-cfg=has_pregenerated");
    }

    if config.wasm_mode == Some(WasmMode::Provider) {
        println!("cargo:rustc-cfg=boxdd_sys_wasm_provider");
        if !has_pregenerated && !config.force_bindgen {
            panic!("BOXDD_SYS_WASM_MODE=provider requires checked-in pregenerated bindings");
        }
        if legacy_wasm_provider {
            println!("cargo:rustc-cfg=boxdd_sys_legacy_wasm_provider_bindings");
            generate_legacy_wasm_provider_bindings(&pregenerated, &config.out_dir);
        }
    }

    if config.force_bindgen || (!has_pregenerated && !config.is_docsrs) {
        #[cfg(feature = "bindgen")]
        generate_bindings(
            &config.manifest_dir,
            &config.out_dir,
            &config.bindgen_target,
            config.precision,
        );
        #[cfg(not(feature = "bindgen"))]
        {
            if config.force_bindgen {
                panic!("BOXDD_SYS_FORCE_BINDGEN=1 requires the `bindgen` feature");
            }
            panic!(
                "pregenerated Box2D bindings are missing; enable `bindgen` or refresh checked-in bindings"
            );
        }
    }

    if config.is_docsrs {
        println!("cargo:warning=DOCS_RS detected: skipping native Box2D C build");
        return;
    }

    if config.skip_cc {
        if config.wasm_mode == Some(WasmMode::Source) {
            panic!(
                "BOXDD_SYS_SKIP_CC=1 cannot be combined with BOXDD_SYS_WASM_MODE=source; source mode must compile Box2D C sources"
            );
        }
        println!("cargo:warning=Skipping native Box2D C build due to BOXDD_SYS_SKIP_CC");
        return;
    }

    if handle_wasm_build(&config, &upstream.c_sources) {
        return;
    }

    if try_link_system(&config.target_arch) {
        return;
    }

    if !cfg!(feature = "build-from-source") {
        println!(
            "cargo:warning=build-from-source disabled: not compiling vendored Box2D C sources"
        );
        return;
    }

    build_box2d_from_source(&config, &upstream.c_sources);
}

fn validate_build_config(config: &BuildConfig) {
    if config.wasm_mode == Some(WasmMode::Provider) && config.target_arch != "wasm32" {
        panic!("BOXDD_SYS_WASM_MODE=provider is only valid for wasm32 targets");
    }
}

fn load_upstream_manifest(manifest_dir: &Path) -> UpstreamBuildManifest {
    let path = manifest_dir.join("upstream.toml");
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    let manifest: toml::Value = toml::from_str(&source)
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()));
    let revision = manifest
        .get("active_revision")
        .and_then(toml::Value::as_str)
        .unwrap_or_else(|| panic!("{} has no string active_revision", path.display()));
    assert!(
        revision.len() == 40
            && revision
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "{} active_revision must be a lowercase 40-character Git SHA",
        path.display()
    );

    let raw_sources = manifest
        .get("source_inventory")
        .and_then(|inventory| inventory.get("c_sources"))
        .and_then(toml::Value::as_array)
        .unwrap_or_else(|| panic!("{} has no source_inventory.c_sources array", path.display()));
    assert!(
        !raw_sources.is_empty(),
        "{} source_inventory.c_sources must not be empty",
        path.display()
    );

    let raw_sources = raw_sources
        .iter()
        .map(|source| {
            source.as_str().unwrap_or_else(|| {
                panic!(
                    "{} source_inventory.c_sources entries must be strings",
                    path.display()
                )
            })
        })
        .collect::<Vec<_>>();
    let c_sources = validate_c_source_paths(raw_sources).unwrap_or_else(|error| {
        panic!(
            "{} has invalid source_inventory.c_sources: {error}",
            path.display()
        )
    });

    UpstreamBuildManifest {
        active_revision: revision.to_owned(),
        c_sources,
    }
}

fn emit_build_identity(config: &BuildConfig, upstream_sha: &str, wasm_import_module: &str) {
    println!("cargo:rustc-env=BOXDD_SYS_UPSTREAM_SHA={upstream_sha}");
    println!("cargo:rustc-env=BOXDD_SYS_WASM_IMPORT_MODULE={wasm_import_module}");
    println!("cargo:precision={}", config.precision.as_str());
    println!("cargo:upstream_sha={upstream_sha}");
    println!(
        "cargo:include={}",
        config
            .manifest_dir
            .join("third-party/box2d/include")
            .display()
    );
    println!("cargo:wasm_import_module={wasm_import_module}");
}

fn reject_external_precision_overrides(target: &str) {
    let normalized_target = target.replace('-', "_");
    let target_keys = [
        format!("CFLAGS_{target}"),
        format!("CFLAGS_{normalized_target}"),
        format!("{target}_CFLAGS"),
        format!("{normalized_target}_CFLAGS"),
        format!("BINDGEN_EXTRA_CLANG_ARGS_{target}"),
        format!("BINDGEN_EXTRA_CLANG_ARGS_{normalized_target}"),
    ];
    for key in ["CFLAGS", "CPPFLAGS", "CL", "BINDGEN_EXTRA_CLANG_ARGS"]
        .into_iter()
        .map(str::to_owned)
        .chain(target_keys)
    {
        println!("cargo:rerun-if-env-changed={key}");
        let Some(value) = env::var_os(&key) else {
            continue;
        };
        if value.to_string_lossy().contains("BOX2D_DOUBLE_PRECISION") {
            panic!(
                "{key} must not define BOX2D_DOUBLE_PRECISION; use the `double-precision` Cargo feature so C and Rust select one ABI"
            );
        }
    }
}

fn handle_wasm_build(config: &BuildConfig, c_sources: &[PathBuf]) -> bool {
    let Some(mode) = config.wasm_mode else {
        return false;
    };

    match mode {
        WasmMode::CompileOnly => {
            println!(
                "cargo:warning=boxdd-sys is using compile-only WASM mode; Box2D C sources are not linked"
            );
            true
        }
        WasmMode::Provider => {
            println!(
                "cargo:warning=boxdd-sys WASM provider mode is active; Box2D symbols are imported from the browser provider module"
            );
            true
        }
        WasmMode::Source => {
            if !cfg!(feature = "build-from-source") {
                panic!(
                    "BOXDD_SYS_WASM_MODE=source requires the default `build-from-source` feature"
                );
            }
            build_box2d_from_source(config, c_sources);
            true
        }
    }
}

fn needs_legacy_wasm_provider_bindings(pregenerated: &Path, precision: Precision) -> bool {
    let source = fs::read_to_string(pregenerated).unwrap_or_else(|error| {
        panic!(
            "failed to read pregenerated bindings at {}: {error}",
            pregenerated.display()
        )
    });
    let expected = format!(
        "#[link(wasm_import_module = \"{}\")]",
        precision.wasm_import_module()
    );
    if source.contains(&expected) {
        return false;
    }
    if source.contains("wasm_import_module") {
        panic!(
            "pregenerated bindings at {} use a stale WASM import module; refresh them with xtask",
            pregenerated.display()
        );
    }
    if precision == Precision::Double {
        panic!(
            "double-precision pregenerated bindings at {} lack their precision-specific WASM import module; refresh them with xtask",
            pregenerated.display()
        );
    }
    true
}

fn generate_legacy_wasm_provider_bindings(pregenerated: &Path, out_dir: &Path) {
    let source = fs::read_to_string(pregenerated).unwrap_or_else(|err| {
        panic!(
            "failed to read pregenerated bindings at {}: {err}",
            pregenerated.display()
        )
    });
    let rewritten = source.replace(
        "unsafe extern \"C\" {",
        &format!(
            "#[link(wasm_import_module = \"{LEGACY_WASM_IMPORT_MODULE}\")]\nunsafe extern \"C\" {{"
        ),
    );
    if rewritten == source {
        panic!(
            "failed to generate WASM provider bindings from {}; no extern blocks were found",
            pregenerated.display()
        );
    }
    fs::write(out_dir.join("wasm_provider_bindings.rs"), rewritten)
        .expect("failed to write WASM provider bindings");
}

#[cfg(feature = "bindgen")]
fn generate_bindings(manifest_dir: &Path, out_dir: &Path, target: &str, precision: Precision) {
    let include_root = manifest_dir
        .join("third-party")
        .join("box2d")
        .join("include");
    let header = include_root.join("box2d").join("box2d.h");
    let builder = bindgen::Builder::default()
        .header(header.to_string_lossy())
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .clang_args(["-x", "c", "-std=c17"])
        .clang_arg(format!("--target={target}"))
        .clang_arg(format!("-I{}", include_root.display()))
        .wasm_import_module_name(precision.wasm_import_module())
        .allowlist_function("b2.*")
        .allowlist_type("b2.*")
        .allowlist_var("B2_.*")
        .layout_tests(false);
    let builder = match precision {
        Precision::Single => builder.clang_arg("-UBOX2D_DOUBLE_PRECISION"),
        Precision::Double => builder.clang_arg("-DBOX2D_DOUBLE_PRECISION=1"),
    };
    let bindings = configure_bindgen_host_headers(builder, target)
        .generate()
        .expect("failed to generate Box2D bindings");

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("failed to write Box2D bindings");
}

#[cfg(feature = "bindgen")]
fn configure_bindgen_host_headers(builder: bindgen::Builder, target: &str) -> bindgen::Builder {
    if !cfg!(target_os = "macos") || !target.contains("-linux-") {
        return builder;
    }

    // Apple Clang has no Linux libc sysroot. The Box2D public API only needs ISO C headers here;
    // Xcode supplies those headers while `--target` above remains the manifest's Linux ABI.
    let output = std::process::Command::new("xcrun")
        .args(["--sdk", "macosx", "--show-sdk-path"])
        .output()
        .expect("xcrun is required to locate ISO C headers for Linux-target bindgen on macOS");
    assert!(
        output.status.success(),
        "xcrun could not locate the macOS SDK for Linux-target bindgen: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    let sdk = String::from_utf8(output.stdout)
        .expect("xcrun SDK path must be UTF-8")
        .trim()
        .to_owned();
    assert!(!sdk.is_empty(), "xcrun returned an empty macOS SDK path");
    builder.clang_arg(format!("--sysroot={sdk}"))
}

#[cfg(not(feature = "bindgen"))]
#[allow(dead_code)]
fn generate_bindings(_manifest_dir: &Path, _out_dir: &Path, _target: &str, _precision: Precision) {
    unreachable!("generate_bindings is only available with the `bindgen` feature enabled");
}

fn link_kind_from_env() -> Option<&'static str> {
    match env::var("BOXDD_SYS_LINK_KIND").ok().as_deref() {
        Some("static") | Some("STATIC") => Some("static"),
        Some("dylib") | Some("DYLIB") | Some("shared") | Some("SHARED") => Some("dylib"),
        _ => None,
    }
}

fn warn_or_error_system_ignores_features() {
    let simd = cfg!(feature = "simd-avx2");
    let nosimd = cfg!(feature = "disable-simd");
    let validate = cfg!(feature = "validate");
    if Precision::ACTIVE == Precision::Double {
        panic!(
            "double-precision system libraries are not accepted without ABI attestation; use the vendored source provider"
        );
    }
    if simd || nosimd || validate {
        if parse_bool_env("BOXDD_SYS_STRICT_FEATURES") {
            panic!(
                "System library mode ignores crate features (simd-avx2/disable-simd/validate). Use source build instead or unset BOXDD_SYS_STRICT_FEATURES."
            );
        } else {
            println!(
                "cargo:warning=System library mode ignores crate features: {}{}{}",
                if simd { "simd-avx2 " } else { "" },
                if nosimd { "disable-simd " } else { "" },
                if validate { "validate" } else { "" },
            );
        }
    }
}

fn try_link_system(_target_arch: &str) -> bool {
    if let Ok(dir) = env::var("BOX2D_LIB_DIR") {
        println!("cargo:rustc-link-search=native={dir}");
        if let Some(kind) = link_kind_from_env() {
            println!("cargo:rustc-link-lib={kind}=box2d");
        } else {
            println!("cargo:rustc-link-lib=box2d");
        }
        warn_or_error_system_ignores_features();
        return true;
    }

    #[cfg(feature = "pkg-config")]
    {
        if pkg_config::Config::new()
            .cargo_metadata(true)
            .probe("box2d")
            .is_ok()
        {
            warn_or_error_system_ignores_features();
            return true;
        }
    }

    false
}

fn configure_msvc_language(build: &mut cc::Build) {
    match build.is_flag_supported("/std:c17") {
        Ok(true) => {
            build.flag("/std:c17");
        }
        Ok(false) => {
            panic!("the selected MSVC C compiler does not support the C17 mode required by Box2D");
        }
        Err(error) => {
            panic!("failed to verify MSVC C17 support required by Box2D: {error}");
        }
    }
    if build.get_compiler().is_like_clang_cl() {
        build.flag("/clang:-ffp-contract=off");
    }
}

fn build_box2d_from_source(config: &BuildConfig, c_sources: &[PathBuf]) {
    let box2d_root = config.manifest_dir.join("third-party").join("box2d");
    let box2d_include = box2d_root.join("include");
    let box2d_src = box2d_root.join("src");
    if !box2d_include.exists() || !box2d_src.exists() {
        panic!(
            "Box2D submodule not found at {}; run: git submodule update --init --recursive",
            box2d_root.display()
        );
    }

    let mut build = cc::Build::new();
    build.include(&box2d_include);
    build.include(&box2d_src);

    for relative_path in c_sources {
        let source = box2d_root.join(relative_path);
        assert!(
            source.is_file(),
            "Box2D source declared by upstream.toml is missing: {}",
            source.display()
        );
        build.file(source);
    }

    if config.precision == Precision::Double {
        build.define("BOX2D_DOUBLE_PRECISION", None);
    }

    if config.target_env == "msvc" {
        let use_static_crt = env::var("CARGO_CFG_TARGET_FEATURE")
            .unwrap_or_default()
            .split(',')
            .any(|feature| feature == "crt-static");
        build.static_crt(use_static_crt);
        build.debug(config.is_debug());
        build.opt_level(if config.is_debug() { 0 } else { 2 });
        configure_msvc_language(&mut build);
        if cfg!(feature = "disable-simd") {
            build.define("BOX2D_DISABLE_SIMD", None);
        } else if cfg!(feature = "simd-avx2") && config.target_arch == "x86_64" {
            build.define("BOX2D_AVX2", None);
            build.flag_if_supported("/arch:AVX2");
        }
    } else {
        build.flag("-std=c17");
        build.flag("-ffp-contract=off");
        build.debug(config.is_debug());
        build.opt_level(if config.is_debug() { 0 } else { 2 });

        if config.target_arch == "wasm32" {
            configure_wasm_source_build(config, &mut build);
        } else if config.target_os == "linux"
            || config.target_os == "macos"
            || config.target_env == "gnu"
        {
            if config.target_os == "linux" {
                build.define("_POSIX_C_SOURCE", Some("199309L"));
                println!("cargo:rustc-link-lib=m");
                println!("cargo:rustc-link-lib=pthread");
            }
            build.flag_if_supported("-pthread");
        }

        if cfg!(feature = "disable-simd") || config.target_arch == "wasm32" {
            build.define("BOX2D_DISABLE_SIMD", None);
        } else if cfg!(feature = "simd-avx2") && config.target_arch == "x86_64" {
            build.define("BOX2D_AVX2", None);
            build.flag_if_supported("-mavx2");
        }
    }

    if cfg!(feature = "validate") {
        build.define("BOX2D_VALIDATE", None);
    }

    build.compile("box2d");
}

fn configure_wasm_source_build(config: &BuildConfig, build: &mut cc::Build) {
    if config.target_env == "emscripten" {
        build.define("_POSIX_C_SOURCE", Some("199309L"));
        if let Ok(emsdk) = env::var("EMSDK") {
            let emscripten = PathBuf::from(&emsdk).join("upstream").join("emscripten");
            let clang = emscripten.join(if cfg!(windows) { "emcc.bat" } else { "emcc" });
            if clang.exists() {
                build.compiler(clang);
            }
        }
        build.flag("-target");
        build.flag("wasm32-unknown-emscripten");
    } else if config.target_os == "wasi" {
        configure_wasi_sysroot(build);
        build.flag("-target");
        build.flag("wasm32-wasip1");
    } else {
        build.flag("-target");
        build.flag("wasm32-unknown-unknown");
    }
}

fn configure_wasi_sysroot(build: &mut cc::Build) {
    let sysroot = env::var_os("WASI_SYSROOT")
        .map(PathBuf::from)
        .or_else(|| env::var_os("WASI_SDK_PATH").map(|path| PathBuf::from(path).join("share/wasi-sysroot")))
        .unwrap_or_else(|| {
            panic!(
                "wasm32-wasip1 source builds require WASI_SYSROOT or WASI_SDK_PATH so clang can find WASI libc headers"
            )
        });

    let has_libc_headers = sysroot.join("include").join("math.h").exists()
        || sysroot
            .join("include")
            .join("wasm32-wasi")
            .join("math.h")
            .exists();
    if !has_libc_headers {
        panic!(
            "WASI sysroot at {} does not contain WASI libc headers",
            sysroot.display()
        );
    }

    build.flag(format!("--sysroot={}", sysroot.display()));
}
