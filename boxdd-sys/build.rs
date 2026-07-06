use std::env;
use std::fs;
use std::path::{Path, PathBuf};

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
    profile: String,
    is_docsrs: bool,
    skip_cc: bool,
    force_bindgen: bool,
    wasm_mode: Option<WasmMode>,
}

impl BuildConfig {
    fn from_env() -> Self {
        let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
        let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
        let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
        let is_docsrs = env::var("DOCS_RS").is_ok() || env::var("CARGO_CFG_DOCSRS").is_ok();
        let skip_cc = parse_bool_env("BOXDD_SYS_SKIP_CC");
        let force_bindgen = parse_bool_env("BOXDD_SYS_FORCE_BINDGEN");
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
            profile: env::var("PROFILE").unwrap_or_else(|_| "release".into()),
            is_docsrs,
            skip_cc,
            force_bindgen,
            wasm_mode,
        }
    }

    fn is_debug(&self) -> bool {
        self.profile == "debug"
    }

    fn pregenerated_bindings(&self) -> PathBuf {
        self.manifest_dir
            .join("src")
            .join("bindings_pregenerated.rs")
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
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=third-party/box2d/include/box2d/box2d.h");
    println!("cargo:rerun-if-changed=third-party/box2d");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_SKIP_CC");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_WASM_CC");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_WASM_MODE");
    println!("cargo:rerun-if-env-changed=BOX2D_LIB_DIR");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_LINK_KIND");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_FORCE_BINDGEN");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_STRICT_FEATURES");
    println!("cargo:rerun-if-env-changed=EMSDK");
    println!("cargo:rerun-if-env-changed=WASI_SDK_PATH");
    println!("cargo:rerun-if-env-changed=WASI_SYSROOT");
    println!("cargo:rerun-if-env-changed=DOCS_RS");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_DOCSRS");

    let config = BuildConfig::from_env();
    let pregenerated = config.pregenerated_bindings();
    let has_pregenerated = pregenerated.exists();

    validate_build_config(&config);

    if config.force_bindgen {
        println!("cargo:rustc-cfg=force_bindgen");
    } else if has_pregenerated {
        println!("cargo:rustc-cfg=has_pregenerated");
    }

    if config.wasm_mode == Some(WasmMode::Provider) {
        println!("cargo:rustc-cfg=boxdd_sys_wasm_provider");
        if config.force_bindgen {
            panic!(
                "BOXDD_SYS_WASM_MODE=provider cannot be combined with BOXDD_SYS_FORCE_BINDGEN=1 yet"
            );
        }
        if !has_pregenerated {
            panic!("BOXDD_SYS_WASM_MODE=provider requires checked-in pregenerated bindings");
        }
        generate_wasm_provider_bindings(&pregenerated, &config.out_dir);
    }

    if config.force_bindgen || (!has_pregenerated && !config.is_docsrs) {
        #[cfg(feature = "bindgen")]
        generate_bindings(&config.manifest_dir, &config.out_dir);
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

    if handle_wasm_build(&config) {
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

    build_box2d_from_source(&config);
}

fn validate_build_config(config: &BuildConfig) {
    if config.wasm_mode == Some(WasmMode::Provider) && config.target_arch != "wasm32" {
        panic!("BOXDD_SYS_WASM_MODE=provider is only valid for wasm32 targets");
    }
}

fn handle_wasm_build(config: &BuildConfig) -> bool {
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
            build_box2d_from_source(config);
            true
        }
    }
}

fn generate_wasm_provider_bindings(pregenerated: &Path, out_dir: &Path) {
    const IMPORT_MODULE: &str = "box2d-sys-v0";
    let source = fs::read_to_string(pregenerated).unwrap_or_else(|err| {
        panic!(
            "failed to read pregenerated bindings at {}: {err}",
            pregenerated.display()
        )
    });
    let rewritten = source.replace(
        "unsafe extern \"C\" {",
        &format!("#[link(wasm_import_module = \"{IMPORT_MODULE}\")]\nunsafe extern \"C\" {{"),
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
fn generate_bindings(manifest_dir: &Path, out_dir: &Path) {
    let include_root = manifest_dir
        .join("third-party")
        .join("box2d")
        .join("include");
    let header = include_root.join("box2d").join("box2d.h");
    let bindings = bindgen::Builder::default()
        .header(header.to_string_lossy())
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .clang_args(["-x", "c", "-std=c17"])
        .clang_arg(format!("-I{}", include_root.display()))
        .allowlist_function("b2.*")
        .allowlist_type("b2.*")
        .allowlist_var("B2_.*")
        .layout_tests(false)
        .generate()
        .expect("failed to generate Box2D bindings");

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("failed to write Box2D bindings");
}

#[cfg(not(feature = "bindgen"))]
#[allow(dead_code)]
fn generate_bindings(_manifest_dir: &Path, _out_dir: &Path) {
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

fn add_msvc_c_standard_flag(build: &mut cc::Build) {
    match build.is_flag_supported("/std:c17") {
        Ok(true) => {
            build.flag("/std:c17");
        }
        Ok(false) | Err(_) => {
            build.flag_if_supported("/std:c11");
        }
    }
}

fn build_box2d_from_source(config: &BuildConfig) {
    let box2d_root = config.manifest_dir.join("third-party").join("box2d");
    let box2d_include = box2d_root.join("include");
    let box2d_src = box2d_root.join("src");
    if !box2d_include.exists() || !box2d_src.exists() {
        println!(
            "cargo:warning=Box2D submodule not found at {}; run: git submodule update --init --recursive",
            box2d_root.display()
        );
    }

    let mut build = cc::Build::new();
    build.include(&box2d_include);
    build.include(&box2d_src);

    let mut files = Vec::new();
    collect_c_files(&box2d_src, &mut files);
    for file in files {
        build.file(file);
    }

    if config.target_env == "msvc" {
        let use_static_crt = env::var("CARGO_CFG_TARGET_FEATURE")
            .unwrap_or_default()
            .split(',')
            .any(|feature| feature == "crt-static");
        build.static_crt(use_static_crt);
        build.debug(config.is_debug());
        build.opt_level(if config.is_debug() { 0 } else { 2 });
        add_msvc_c_standard_flag(&mut build);
        if cfg!(feature = "disable-simd") {
            build.define("BOX2D_DISABLE_SIMD", None);
        } else if cfg!(feature = "simd-avx2") && config.target_arch == "x86_64" {
            build.define("BOX2D_AVX2", None);
            build.flag_if_supported("/arch:AVX2");
        }
    } else {
        build.flag_if_supported("-std=c17");
        build.flag_if_supported("-ffp-contract=off");
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

fn collect_c_files(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_c_files(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "c") {
                out.push(path);
            }
        }
    }
}
