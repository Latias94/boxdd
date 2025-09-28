use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    // Allow using a custom cfg(has_pregenerated) without warnings
    println!("cargo:rustc-check-cfg=cfg(has_pregenerated)");
    println!("cargo:rerun-if-changed=build.rs");
    // Upstream header/source changes
    println!("cargo:rerun-if-changed=third-party/box2d/include/box2d/box2d.h");
    println!("cargo:rerun-if-changed=third-party/box2d");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let profile = env::var("PROFILE").unwrap_or_else(|_| "release".into());
    let is_debug = profile == "debug";

    // Detect features (via cfg) and environment toggles
    let feat_prebuilt: bool = cfg!(feature = "prebuilt");

    // Detect pregenerated bindings and docs.rs
    let pregenerated = manifest_dir.join("src").join("bindings_pregenerated.rs");
    let has_pregenerated = pregenerated.exists();
    if has_pregenerated {
        println!("cargo:rustc-cfg=has_pregenerated");
    }
    let is_docsrs = env::var("DOCS_RS").is_ok() || env::var("CARGO_CFG_DOCSRS").is_ok();
    // For docs.rs, prefer pregenerated if present; otherwise generate

    // Generate bindings for the upstream C API unless we're explicitly preferring pregenerated and it exists
    if !(is_docsrs && has_pregenerated) {
        generate_bindings(&manifest_dir, &out_dir);
    }

    // If building on docs.rs, skip compiling/linking C code. Bindings are enough for rustdoc.
    if is_docsrs {
        println!("cargo:rustc-cfg=docsrs");
        return;
    }

    // Build Box2D + our C wrapper into a single static lib
    build_box2d_and_wrapper(
        &manifest_dir,
        &target_env,
        &target_os,
        is_debug,
        feat_prebuilt,
    );
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
        .clang_args(["-x", "c", "-std=c11"]) // upstream exposes a C API in v3
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

fn build_box2d_and_wrapper(
    manifest_dir: &Path,
    target_env: &str,
    target_os: &str,
    is_debug: bool,
    feat_prebuilt: bool,
) {
    let third_party = manifest_dir.join("third-party");
    let box2d_root = third_party.join("box2d");
    let box2d_include = box2d_root.join("include");
    let box2d_src = box2d_root.join("src");

    if !box2d_include.exists() || !box2d_src.exists() {
        println!(
            "cargo:warning=Box2D submodule not found at {} — run: git submodule update --init --recursive",
            box2d_root.display()
        );
    }

    // If prebuilt feature is enabled, link against a precompiled library and return
    if feat_prebuilt {
        if let Ok(libdir) = env::var("BOX2D_LIB_DIR") {
            println!("cargo:rustc-link-search=native={}", libdir);
            println!("cargo:rustc-link-lib=static=box2d");
            return;
        } else {
            println!(
                "cargo:warning=prebuilt feature enabled but BOX2D_LIB_DIR is not set; attempting system search for 'box2d'"
            );
            println!("cargo:rustc-link-lib=static=box2d");
            return;
        }
    }

    let mut build = cc::Build::new();
    build.include(&box2d_include);

    // Gather all Box2D .cpp sources recursively
    let mut files: Vec<PathBuf> = Vec::new();
    collect_cpp_files(&box2d_src, &mut files);
    for f in files {
        build.file(f);
    }
    // No wrapper needed: upstream exposes a stable C API

    // MSVC tuning
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
        // Opt-in to modern C standard when available on MSVC
        build.flag_if_supported("/std:c17");
        build.flag_if_supported("/std:c11");
    } else {
        // GCC/Clang: prefer C11 where available
        build.flag_if_supported("-std=c11");

        // On Linux, expose POSIX clock_gettime and ensure pthread is available
        if target_os == "linux" {
            build.define("_POSIX_C_SOURCE", Some("199309L"));
            build.flag_if_supported("-pthread");
            println!("cargo:rustc-link-lib=pthread");
        }
    }

    // Compile into static lib named `box2d`
    build.compile("box2d");

    // Link hints (usually handled automatically by cc, but be explicit if needed)
    // C API: no need to link C++ stdlib
}

fn collect_cpp_files(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(rd) = fs::read_dir(dir) {
        for entry in rd.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_cpp_files(&path, out);
            } else if let Some(ext) = path.extension()
                && ext == "c"
            {
                out.push(path);
            }
        }
    }
}
