use std::env;
use std::fs;
use std::path::{Path, PathBuf};

// From-source-only build for Box2D C API.
// - Prefers pregenerated bindings to avoid libclang on CI (e.g. macOS).
// - Always compiles vendored C sources with `cc` for native targets.
// - WASM builds are opt-in via EMSDK/WASI or `BOXDD_SYS_WASM_CC=1`.

fn parse_bool_env(key: &str) -> bool {
    match env::var(key) {
        Ok(v) => matches!(
            v.as_str(),
            "1" | "true" | "yes" | "on" | "TRUE" | "YES" | "ON"
        ),
        Err(_) => false,
    }
}

fn main() {
    // Re-run triggers
    println!("cargo:rustc-check-cfg=cfg(has_pregenerated)");
    println!("cargo:rustc-check-cfg=cfg(has_wasm_pregenerated)");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=third-party/box2d/include/box2d/box2d.h");
    println!("cargo:rerun-if-changed=third-party/box2d");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_SKIP_CC");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_WASM_CC");
    println!("cargo:rerun-if-env-changed=BOX2D_LIB_DIR");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_LINK_KIND");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_FORCE_BINDGEN");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_STRICT_FEATURES");
    println!("cargo:rerun-if-env-changed=EMSDK");
    println!("cargo:rerun-if-env-changed=WASI_SDK_PATH");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let profile = env::var("PROFILE").unwrap_or_else(|_| "release".into());
    let is_debug = profile == "debug";

    // Detect pregenerated bindings and docs.rs
    let pregenerated = manifest_dir.join("src").join("bindings_pregenerated.rs");
    let wasm_pregenerated = manifest_dir
        .join("src")
        .join("wasm_bindings_pregenerated.rs");
    let has_pregenerated = pregenerated.exists();
    let has_wasm_pregenerated = wasm_pregenerated.exists();
    if has_pregenerated {
        println!("cargo:rustc-cfg=has_pregenerated");
    }
    if target_arch == "wasm32" && has_wasm_pregenerated {
        println!("cargo:rustc-cfg=has_wasm_pregenerated");
    }
    let is_docsrs = env::var("DOCS_RS").is_ok() || env::var("CARGO_CFG_DOCSRS").is_ok();

    // If wasm and have wasm-specific pregenerated, copy to OUT_DIR
    let mut used_wasm_pregenerated = false;
    if target_arch == "wasm32"
        && has_wasm_pregenerated
        && let Ok(s) = std::fs::read_to_string(&wasm_pregenerated)
    {
        let _ = std::fs::write(out_dir.join("bindings.rs"), s);
        used_wasm_pregenerated = true;
        println!(
            "cargo:warning=Using wasm pregenerated bindings: {}",
            wasm_pregenerated.display()
        );
    }

    // Run bindgen only if needed or explicitly requested
    let force_bindgen = parse_bool_env("BOXDD_SYS_FORCE_BINDGEN");
    if force_bindgen || (!has_pregenerated && !is_docsrs && !used_wasm_pregenerated) {
        generate_bindings(&manifest_dir, &out_dir);
    }

    // Build C sources unless explicitly skipped
    if parse_bool_env("BOXDD_SYS_SKIP_CC") {
        println!("cargo:warning=Skipping native C build due to BOXDD_SYS_SKIP_CC");
        return;
    }

    if target_arch == "wasm32" {
        if target_env == "emscripten" {
            build_with_cc_wasm(&manifest_dir, &target_env, is_debug);
        } else if target_os == "wasi" {
            if env::var("WASI_SDK_PATH").is_ok() || parse_bool_env("BOXDD_SYS_WASM_CC") {
                build_with_cc_wasi(&manifest_dir, is_debug);
            } else {
                println!(
                    "cargo:warning=WASI target detected but no WASI_SDK_PATH; skipping native C build"
                );
            }
        } else if parse_bool_env("BOXDD_SYS_WASM_CC") {
            build_with_cc_wasm(&manifest_dir, &target_env, is_debug);
        } else {
            println!(
                "cargo:warning=WASM (unknown) skeleton: skipping native C build (set BOXDD_SYS_WASM_CC=1 to enable)"
            );
        }
    } else {
        // Try system link first (env or pkg-config), then fallback to source build
        if try_link_system(&target_arch) {
            return;
        }
        build_box2d_from_source(&manifest_dir, &target_env, &target_os, is_debug);
    }
}

fn generate_bindings(manifest_dir: &Path, out_dir: &Path) {
    let header = manifest_dir
        .join("third-party")
        .join("box2d")
        .join("include")
        .join("box2d")
        .join("box2d.h");
    let include_dir = header.parent().unwrap().to_path_buf();
    let bindings = bindgen::Builder::default()
        .header(header.to_string_lossy())
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .clang_args(["-x", "c", "-std=c17"]) // upstream uses C17
        .clang_arg(format!("-I{}", include_dir.display()))
        .allowlist_function("b2.*")
        .allowlist_type("b2.*")
        .allowlist_var("B2_.*")
        .layout_tests(false)
        .generate()
        .expect("Failed to generate box2d bindings");
    let out = out_dir.join("bindings.rs");
    bindings
        .write_to_file(&out)
        .expect("Couldn't write bindings!");
}

fn build_box2d_from_source(manifest_dir: &Path, target_env: &str, target_os: &str, is_debug: bool) {
    let third_party = manifest_dir.join("third-party");
    let box2d_root = third_party.join("box2d");
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

    let mut files: Vec<PathBuf> = Vec::new();
    collect_c_files(&box2d_src, &mut files);
    for f in files {
        build.file(f);
    }

    if target_env == "msvc" {
        let use_static_crt = env::var("CARGO_CFG_TARGET_FEATURE")
            .unwrap_or_default()
            .split(',')
            .any(|f| f == "crt-static");
        build.static_crt(use_static_crt);
        if use_static_crt {
            build.flag("/MT");
        } else {
            build.flag("/MD");
        }
        if is_debug {
            build.debug(true);
            build.opt_level(0);
        } else {
            build.debug(false);
            build.opt_level(2);
        }
        build.flag_if_supported("/std:c17");
        build.flag_if_supported("/std:c11");
        if cfg!(feature = "validate") {
            build.define("BOX2D_VALIDATE", None);
        }
        let is_x86_64 = env::var("CARGO_CFG_TARGET_ARCH").ok().as_deref() == Some("x86_64");
        if cfg!(feature = "disable-simd") {
            build.define("BOX2D_DISABLE_SIMD", None);
        } else if cfg!(feature = "simd-avx2") && is_x86_64 {
            build.define("BOX2D_AVX2", None);
            build.flag_if_supported("/arch:AVX2");
        }
    } else {
        build.flag_if_supported("-std=c17");
        if target_os == "linux"
            || target_os == "macos"
            || env::var("CARGO_CFG_TARGET_ENV").ok().as_deref() == Some("gnu")
        {
            build.flag_if_supported("-ffp-contract=off");
        }
        if cfg!(feature = "validate") {
            build.define("BOX2D_VALIDATE", None);
        }
        let is_x86_64 = env::var("CARGO_CFG_TARGET_ARCH").ok().as_deref() == Some("x86_64");
        if cfg!(feature = "disable-simd") {
            build.define("BOX2D_DISABLE_SIMD", None);
        } else if cfg!(feature = "simd-avx2") && is_x86_64 {
            build.define("BOX2D_AVX2", None);
            build.flag_if_supported("-mavx2");
        }
        if target_os == "linux" {
            build.define("_POSIX_C_SOURCE", Some("199309L"));
            build.flag_if_supported("-pthread");
            println!("cargo:rustc-link-lib=pthread");
        }
    }

    build.compile("box2d");
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
    // Env-provided lib dir
    if let Ok(dir) = env::var("BOX2D_LIB_DIR") {
        println!("cargo:rustc-link-search=native={}", dir);
        if let Some(kind) = link_kind_from_env() {
            println!("cargo:rustc-link-lib={}=box2d", kind);
        } else {
            println!("cargo:rustc-link-lib=box2d");
        }
        warn_or_error_system_ignores_features();
        return true;
    }
    // pkg-config path when enabled
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

fn collect_c_files(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(rd) = fs::read_dir(dir) {
        for entry in rd.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_c_files(&path, out);
            } else if let Some(ext) = path.extension()
                && ext == "c"
            {
                out.push(path);
            }
        }
    }
}

fn build_with_cc_wasm(manifest_dir: &Path, target_env: &str, is_debug: bool) {
    let third_party = manifest_dir.join("third-party");
    let box2d_root = third_party.join("box2d");
    let box2d_include = box2d_root.join("include");
    let box2d_src = box2d_root.join("src");

    let mut build = cc::Build::new();
    build.include(&box2d_include);

    let mut files: Vec<PathBuf> = Vec::new();
    collect_c_files(&box2d_src, &mut files);
    for f in files {
        build.file(f);
    }

    if target_env == "emscripten" {
        if let Ok(emsdk) = env::var("EMSDK") {
            let mut clang = PathBuf::from(&emsdk);
            clang.push("upstream");
            clang.push("bin");
            clang.push(if cfg!(windows) { "clang.exe" } else { "clang" });
            if clang.exists() {
                build.compiler(clang);
            }
            let mut sysroot = PathBuf::from(&emsdk);
            sysroot.push("upstream");
            sysroot.push("emscripten");
            sysroot.push("cache");
            sysroot.push("sysroot");
            if sysroot.exists() {
                build.flag(format!("--sysroot={}", sysroot.display()));
            }
        }
        build.flag("-target");
        build.flag("wasm32-unknown-emscripten");
    } else {
        build.flag("-target");
        build.flag("wasm32-unknown-unknown");
    }

    build.flag_if_supported("-ffp-contract=off");
    if is_debug {
        build.debug(true);
        build.opt_level(0);
    } else {
        build.debug(false);
        build.opt_level(2);
    }
    build.flag_if_supported("-std=c17");
    build.static_flag(true);
    build.compile("box2d");
}

fn build_with_cc_wasi(manifest_dir: &Path, is_debug: bool) {
    let third_party = manifest_dir.join("third-party");
    let box2d_root = third_party.join("box2d");
    let box2d_include = box2d_root.join("include");
    let box2d_src = box2d_root.join("src");

    let mut build = cc::Build::new();
    build.include(&box2d_include);

    let mut files: Vec<PathBuf> = Vec::new();
    collect_c_files(&box2d_src, &mut files);
    for f in files {
        build.file(f);
    }

    if let Ok(wasi_sdk) = env::var("WASI_SDK_PATH") {
        let mut clang = PathBuf::from(&wasi_sdk);
        clang.push("bin");
        clang.push(if cfg!(windows) { "clang.exe" } else { "clang" });
        if clang.exists() {
            build.compiler(clang);
        }
        let mut sysroot = PathBuf::from(&wasi_sdk);
        sysroot.push("share");
        sysroot.push("wasi-sysroot");
        if sysroot.exists() {
            build.flag(format!("--sysroot={}", sysroot.display()));
        }
    }

    build.flag("-target");
    build.flag("wasm32-wasip1");
    build.flag_if_supported("-ffp-contract=off");
    if is_debug {
        build.debug(true);
        build.opt_level(0);
    } else {
        build.debug(false);
        build.opt_level(2);
    }
    build.flag_if_supported("-std=c17");
    build.static_flag(true);
    build.compile("box2d");
}
