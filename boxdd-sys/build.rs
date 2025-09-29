use std::env;
use std::fs;
use std::path::{Path, PathBuf};

// Local build-support helpers (inlined)
fn parse_bool_env(key: &str) -> bool {
    match env::var(key) {
        Ok(v) => matches!(
            v.as_str(),
            "1" | "true" | "yes" | "on" | "TRUE" | "YES" | "ON"
        ),
        Err(_) => false,
    }
}

fn msvc_crt_suffix_from_env(target_env: Option<&str>) -> Option<&'static str> {
    let is_msvc = match target_env {
        Some(s) => s == "msvc",
        None => matches!(
            env::var("CARGO_CFG_TARGET_ENV").ok().as_deref(),
            Some("msvc")
        ),
    };
    if !is_msvc {
        return None;
    }
    let tf = env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default();
    if tf.split(',').any(|f| f == "crt-static") {
        Some("mt")
    } else {
        Some("md")
    }
}

fn expected_lib_name(target_env: &str, base: &str) -> String {
    if target_env == "msvc" {
        format!("{}.lib", base)
    } else {
        format!("lib{}.a", base)
    }
}

fn compose_archive_name(
    crate_short: &str,
    version: &str,
    target: &str,
    link_type: &str,
    extra: Option<&str>,
    crt: &str,
) -> String {
    let extra = extra.unwrap_or("");
    if crt.is_empty() {
        if extra.is_empty() {
            format!(
                "{}-prebuilt-{}-{}-{}.tar.gz",
                crate_short, version, target, link_type
            )
        } else {
            format!(
                "{}-prebuilt-{}-{}-{}{}.tar.gz",
                crate_short, version, target, link_type, extra
            )
        }
    } else if extra.is_empty() {
        format!(
            "{}-prebuilt-{}-{}-{}-{}.tar.gz",
            crate_short, version, target, link_type, crt
        )
    } else {
        format!(
            "{}-prebuilt-{}-{}-{}{}-{}.tar.gz",
            crate_short, version, target, link_type, extra, crt
        )
    }
}

fn release_tags(crate_sys_name: &str, version: &str) -> [String; 2] {
    [
        format!("{}-v{}", crate_sys_name, version),
        format!("v{}", version),
    ]
}

fn release_owner_repo() -> (String, String) {
    let owner = env::var("BUILD_SUPPORT_GH_OWNER").unwrap_or_else(|_| "Latias94".to_string());
    let repo = env::var("BUILD_SUPPORT_GH_REPO").unwrap_or_else(|_| "boxdd".to_string());
    (owner, repo)
}

fn release_candidate_urls(
    owner: &str,
    repo: &str,
    tags: &[String],
    names: &[String],
) -> Vec<String> {
    let mut out = Vec::with_capacity(tags.len() * names.len());
    for tag in tags {
        for name in names {
            out.push(format!(
                "https://github.com/{}/{}/releases/download/{}/{}",
                owner, repo, tag, name
            ));
        }
    }
    out
}

fn release_candidate_urls_env(tags: &[String], names: &[String]) -> Vec<String> {
    let (owner, repo) = release_owner_repo();
    release_candidate_urls(&owner, &repo, tags, names)
}

fn is_offline() -> bool {
    match env::var("CARGO_NET_OFFLINE") {
        Ok(v) => matches!(
            v.as_str(),
            "1" | "true" | "yes" | "on" | "TRUE" | "YES" | "ON"
        ),
        Err(_) => false,
    }
}

fn prebuilt_cache_root_from_env_or_target(
    manifest_dir: &Path,
    cache_env_var: &str,
    folder: &str,
) -> PathBuf {
    if let Ok(dir) = env::var(cache_env_var) {
        return PathBuf::from(dir);
    }
    let target_dir = env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest_dir.parent().unwrap().join("target"));
    target_dir.join(folder)
}

fn prebuilt_extract_dir_env(cache_root: &Path, target_env: &str) -> PathBuf {
    let target = env::var("TARGET").unwrap_or_default();
    let crt_suffix = if target_env == "msvc" {
        let tf = env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default();
        if tf.split(',').any(|f| f == "crt-static") {
            "-mt"
        } else {
            "-md"
        }
    } else {
        ""
    };
    cache_root
        .join(target)
        .join(format!("static{}", crt_suffix))
}

fn extract_archive_to_cache(
    archive_path: &Path,
    cache_root: &Path,
    lib_name: &str,
) -> Result<PathBuf, String> {
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let extract_dir = prebuilt_extract_dir_env(cache_root, &target_env);
    if extract_dir.exists() {
        let lib_dir = extract_dir.join("lib");
        if lib_dir.join(lib_name).exists() || extract_dir.join(lib_name).exists() {
            return Ok(lib_dir);
        }
        let _ = std::fs::remove_dir_all(&extract_dir);
    }
    std::fs::create_dir_all(&extract_dir)
        .map_err(|e| format!("create dir {}: {}", extract_dir.display(), e))?;
    let file = std::fs::File::open(archive_path)
        .map_err(|e| format!("open {}: {}", archive_path.display(), e))?;
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(file));
    archive
        .unpack(&extract_dir)
        .map_err(|e| format!("unpack {}: {}", archive_path.display(), e))?;
    let lib_dir = extract_dir.join("lib");
    if lib_dir.join(lib_name).exists() {
        return Ok(lib_dir);
    }
    if extract_dir.join(lib_name).exists() {
        return Ok(extract_dir);
    }
    Err("extracted archive did not contain expected library".into())
}

fn download_prebuilt(
    cache_root: &Path,
    url: &str,
    lib_name: &str,
    _target_env: &str,
) -> Result<PathBuf, String> {
    let dl_dir = cache_root.join("download");
    let _ = std::fs::create_dir_all(&dl_dir);
    if url.ends_with(".tar.gz") || url.ends_with(".tgz") {
        let fname = url.split('/').next_back().unwrap_or("prebuilt.tar.gz");
        let archive_path = dl_dir.join(fname);
        if !archive_path.exists() {
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .map_err(|e| format!("create http client: {}", e))?;
            let resp = client
                .get(url)
                .send()
                .map_err(|e| format!("http get: {}", e))?;
            if !resp.status().is_success() {
                return Err(format!("http status {}", resp.status()));
            }
            let bytes = resp.bytes().map_err(|e| format!("read body: {}", e))?;
            std::fs::write(&archive_path, &bytes)
                .map_err(|e| format!("write {}: {}", archive_path.display(), e))?;
        }
        return extract_archive_to_cache(&archive_path, cache_root, lib_name);
    }
    let dst = dl_dir.join(lib_name);
    if dst.exists() {
        return Ok(dl_dir);
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("http client: {}", e))?;
    let resp = client
        .get(url)
        .send()
        .map_err(|e| format!("http get: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("http status {}", resp.status()));
    }
    let bytes = resp.bytes().map_err(|e| format!("read body: {}", e))?;
    std::fs::write(&dst, &bytes).map_err(|e| format!("write {}: {}", dst.display(), e))?;
    Ok(dl_dir)
}

fn main() {
    // Allow using a custom cfg(has_pregenerated) without warnings
    println!("cargo:rustc-check-cfg=cfg(has_pregenerated)");
    println!("cargo:rustc-check-cfg=cfg(has_wasm_pregenerated)");
    println!("cargo:rerun-if-changed=build.rs");
    // Upstream header/source changes
    println!("cargo:rerun-if-changed=third-party/box2d/include/box2d/box2d.h");
    println!("cargo:rerun-if-changed=third-party/box2d");
    // Env toggles for prebuilt/native build
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_LIB_DIR");
    println!("cargo:rerun-if-env-changed=BOX2D_LIB_DIR");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_PREBUILT_URL");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_USE_PREBUILT");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_FORCE_BUILD");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_SKIP_CC");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_CACHE_DIR");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_WASM_CC");
    println!("cargo:rerun-if-env-changed=CARGO_NET_OFFLINE");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_STRICT_WASM_BINDINGS");
    println!("cargo:rerun-if-env-changed=EMSDK");
    println!("cargo:rerun-if-env-changed=WASI_SDK_PATH");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let profile = env::var("PROFILE").unwrap_or_else(|_| "release".into());
    let is_debug = profile == "debug";

    // Detect features (via cfg) and environment toggles
    let feat_prebuilt: bool = cfg!(feature = "prebuilt");

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
    // For docs.rs, prefer pregenerated if present; otherwise generate

    // If wasm and have wasm-specific pregenerated, use it by copying to OUT_DIR/bindings.rs
    let mut used_wasm_pregenerated = false;
    if target_arch == "wasm32" && has_wasm_pregenerated {
        if let Ok(s) = std::fs::read_to_string(&wasm_pregenerated) {
            let _ = std::fs::write(out_dir.join("bindings.rs"), s);
            used_wasm_pregenerated = true;
            println!(
                "cargo:warning=Using wasm pregenerated bindings: {}",
                wasm_pregenerated.display()
            );
        }
    }

    // Strict mode: require wasm pregenerated or proper sysroot setup
    if target_arch == "wasm32" && parse_bool_env("BOXDD_SYS_STRICT_WASM_BINDINGS") {
        let allow_sysroot = (target_env == "emscripten" && env::var("EMSDK").is_ok())
            || (target_os == "wasi" && env::var("WASI_SDK_PATH").is_ok());
        if !has_wasm_pregenerated && !allow_sysroot {
            panic!(
                "Strict wasm bindings enabled: provide src/wasm_bindings_pregenerated.rs or set EMSDK/WASI_SDK_PATH for bindgen"
            );
        }
    }

    // Generate bindings unless docs.rs/wasm prefer pregenerated and it exists
    if !(is_docsrs || used_wasm_pregenerated || (target_arch == "wasm32" && has_pregenerated)) {
        generate_bindings(&manifest_dir, &out_dir);
    }

    // If building on docs.rs, skip compiling/linking C code. Bindings are enough for rustdoc.
    if is_docsrs {
        println!("cargo:rustc-cfg=docsrs");
        return;
    }

    // Build strategy selection
    let force_build = parse_bool_env("BOXDD_SYS_FORCE_BUILD");
    let skip_cc = parse_bool_env("BOXDD_SYS_SKIP_CC");

    // Try prebuilt first unless forced to build (skip for wasm targets)
    let mut linked_prebuilt = false;
    if !force_build && target_arch != "wasm32" {
        linked_prebuilt = try_link_prebuilt_all(&manifest_dir, &target_env);
    }

    // Build from sources when needed
    if !linked_prebuilt {
        if skip_cc {
            println!("cargo:warning=Skipping native C build due to BOXDD_SYS_SKIP_CC");
        } else {
            // WASM targets: build with wasm-friendly flags/toolchain
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
                // Build Box2D into a single static lib
                build_box2d_and_wrapper(
                    &manifest_dir,
                    &target_env,
                    &target_os,
                    is_debug,
                    feat_prebuilt,
                );
            }
        }
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

    // If prebuilt feature is enabled, prefer linking against a precompiled library and return
    if feat_prebuilt {
        if let Some(dir) = env::var_os("BOXDD_SYS_LIB_DIR").or_else(|| env::var_os("BOX2D_LIB_DIR"))
        {
            let libdir = PathBuf::from(dir);
            if try_link_prebuilt(&libdir, target_env) {
                println!(
                    "cargo:warning=Using prebuilt box2d from {}",
                    libdir.display()
                );
                return;
            }
        } else {
            println!(
                "cargo:warning=feature 'prebuilt' enabled but BOXDD_SYS_LIB_DIR/BOX2D_LIB_DIR not set; attempting system search for 'box2d' (pkg-config if enabled)"
            );
            if try_pkg_config() {
                return;
            }
            println!("cargo:rustc-link-lib={}=box2d", link_kind());
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

        // Features: validate / SIMD
        if cfg!(feature = "validate") {
            build.define("BOX2D_VALIDATE", None);
        }
        let is_x86_64 = env::var("CARGO_CFG_TARGET_ARCH").ok().as_deref() == Some("x86_64");
        let disable_simd = cfg!(feature = "disable-simd");
        if disable_simd {
            build.define("BOX2D_DISABLE_SIMD", None);
        } else if cfg!(feature = "simd-avx2") && is_x86_64 {
            build.define("BOX2D_AVX2", None);
            build.flag_if_supported("/arch:AVX2");
        }
    } else {
        // GCC/Clang: prefer C17 to match upstream
        build.flag_if_supported("-std=c17");

        // Deterministic math like upstream: disable FP contraction
        if target_os == "linux"
            || target_os == "macos"
            || env::var("CARGO_CFG_TARGET_ENV").ok().as_deref() == Some("gnu")
        {
            build.flag_if_supported("-ffp-contract=off");
        }

        // Features: validate / SIMD
        if cfg!(feature = "validate") {
            build.define("BOX2D_VALIDATE", None);
        }
        let is_x86_64 = env::var("CARGO_CFG_TARGET_ARCH").ok().as_deref() == Some("x86_64");
        let disable_simd = cfg!(feature = "disable-simd");
        if disable_simd {
            build.define("BOX2D_DISABLE_SIMD", None);
        } else if cfg!(feature = "simd-avx2") && is_x86_64 {
            build.define("BOX2D_AVX2", None);
            build.flag_if_supported("-mavx2");
        }

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

fn collect_c_files(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(rd) = fs::read_dir(dir) {
        for entry in rd.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_c_files(&path, out);
            } else if let Some(ext) = path.extension() {
                if ext == "c" {
                    out.push(path);
                }
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

    // Gather C files
    let mut files: Vec<PathBuf> = Vec::new();
    collect_c_files(&box2d_src, &mut files);
    for f in files {
        build.file(f);
    }

    // Try EMSDK clang when available
    if target_env == "emscripten" {
        if let Ok(emsdk) = env::var("EMSDK") {
            let mut clang = PathBuf::from(&emsdk);
            clang.push("upstream");
            clang.push("bin");
            clang.push(if cfg!(windows) { "clang.exe" } else { "clang" });
            if clang.exists() {
                build.compiler(clang);
            }
            // Use emscripten sysroot headers
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
        // Unknown-unknown target
        build.flag("-target");
        build.flag("wasm32-unknown-unknown");
    }

    // Deterministic math
    build.flag_if_supported("-ffp-contract=off");

    // Tuning
    if is_debug {
        build.debug(true);
        build.opt_level(0);
    } else {
        build.debug(false);
        build.opt_level(2);
    }

    // C standard
    build.flag_if_supported("-std=c17");

    // Avoid linking any host stdlib
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

    // Gather C files
    let mut files: Vec<PathBuf> = Vec::new();
    collect_c_files(&box2d_src, &mut files);
    for f in files {
        build.file(f);
    }

    // Prefer wasi-sdk's clang & sysroot if available
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
    // Prefer the modern wasip1 triple to match Rust target naming.
    build.flag("wasm32-wasip1");

    // Deterministic math
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

fn try_link_prebuilt_all(manifest_dir: &Path, target_env: &str) -> bool {
    let mut linked = false;
    // 1) Explicit directory via env
    if let Some(dir) = env::var_os("BOXDD_SYS_LIB_DIR").or_else(|| env::var_os("BOX2D_LIB_DIR")) {
        let libdir = PathBuf::from(dir);
        if try_link_prebuilt(&libdir, target_env) {
            println!(
                "cargo:warning=Using prebuilt box2d from {}",
                libdir.display()
            );
            return true;
        }
    }

    // 2) Direct URL to library or archive
    if let Some(url) = env::var_os("BOXDD_SYS_PREBUILT_URL") {
        let cache_root = prebuilt_cache_root(manifest_dir);
        let lib_name = expected_lib_name(target_env, "box2d");
        if let Ok(dir) =
            download_prebuilt(&cache_root, &url.to_string_lossy(), &lib_name, target_env)
        {
            if try_link_prebuilt(&dir, target_env) {
                println!(
                    "cargo:warning=Downloaded and using prebuilt box2d from {}",
                    dir.display()
                );
                return true;
            }
        }
    }

    // 3) Auto download from release when enabled
    let allow_auto_prebuilt =
        cfg!(feature = "prebuilt") || parse_bool_env("BOXDD_SYS_USE_PREBUILT");
    if allow_auto_prebuilt {
        if let Some(dir) = try_download_prebuilt_from_release(manifest_dir, target_env) {
            if try_link_prebuilt(&dir, target_env) {
                println!(
                    "cargo:warning=Downloaded and using prebuilt box2d from release at {}",
                    dir.display()
                );
                return true;
            }
        }
    }

    // 4) Repo-prebuilt fallback
    let repo_prebuilt = manifest_dir
        .join("third-party")
        .join("prebuilt")
        .join(env::var("TARGET").unwrap_or_default());
    if try_link_prebuilt(&repo_prebuilt, target_env) {
        println!(
            "cargo:warning=Using repo prebuilt box2d from {}",
            repo_prebuilt.display()
        );
        linked = true;
    }
    if linked {
        return true;
    }
    // 5) pkg-config fallback when enabled
    if try_pkg_config() {
        println!("cargo:warning=Using system box2d via pkg-config");
        return true;
    }
    false
}

fn try_link_prebuilt(dir: &Path, target_env: &str) -> bool {
    if !dir.exists() {
        return false;
    }
    let lib_name = expected_lib_name(target_env, "box2d");
    let lib_file = dir.join(&lib_name);
    let lib_in_lib_dir = dir.join("lib").join(&lib_name);
    if !lib_file.exists() && !lib_in_lib_dir.exists() {
        return false;
    }
    // Accept prebuilt only if matches CRT variant for MSVC when applicable (we separate mt/md by folder when using build_support)
    println!("cargo:rustc-link-search=native={}", dir.display());
    println!("cargo:rustc-link-lib={}=box2d", link_kind());
    true
}

fn prebuilt_cache_root(manifest_dir: &Path) -> PathBuf {
    prebuilt_cache_root_from_env_or_target(manifest_dir, "BOXDD_SYS_CACHE_DIR", "boxdd-prebuilt")
}

fn try_download_prebuilt_from_release(manifest_dir: &Path, target_env: &str) -> Option<PathBuf> {
    if is_offline() {
        return None;
    }
    let version = env::var("CARGO_PKG_VERSION").unwrap_or_default();
    let link_type = "static";
    let crt = msvc_crt_suffix_from_env(Some(target_env)).unwrap_or("");
    let target = env::var("TARGET").unwrap_or_default();

    // Candidate archive names: with and without CRT suffix for Windows
    let mut names: Vec<String> = Vec::new();
    if !crt.is_empty() {
        names.push(compose_archive_name(
            "boxdd", &version, &target, link_type, None, crt,
        ));
    }
    names.push(compose_archive_name(
        "boxdd", &version, &target, link_type, None, "",
    ));

    let tags = release_tags("boxdd-sys", &version);
    let urls = release_candidate_urls_env(&tags, &names);
    let cache_root = prebuilt_cache_root(manifest_dir);
    let lib_name = expected_lib_name(target_env, "box2d");
    for url in urls {
        if let Ok(dir) = download_prebuilt(&cache_root, &url, &lib_name, target_env) {
            return Some(dir);
        }
    }
    None
}

fn link_kind() -> &'static str {
    if cfg!(feature = "dynamic-link") {
        "dylib"
    } else {
        "static"
    }
}

#[cfg(feature = "pkg-config")]
fn try_pkg_config() -> bool {
    match pkg_config::Config::new()
        .cargo_metadata(true)
        .probe("box2d")
    {
        Ok(_lib) => true,
        Err(e) => {
            println!("cargo:warning=pkg-config probe failed: {}", e);
            false
        }
    }
}

#[cfg(not(feature = "pkg-config"))]
fn try_pkg_config() -> bool {
    false
}
