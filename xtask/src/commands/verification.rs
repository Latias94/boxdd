use std::{
    env,
    path::Path,
    process::{Command, Output},
};

use crate::{Error, Result, qualified_git::qualified_git_command};

use super::{
    provider::{self, ProviderPrecision},
    support::{cargo_target_dir, run_command},
};

const VERIFICATION_NIGHTLY: &str = "nightly-2026-05-27";
const SEMVER_CHECKS_VERSION: &str = "0.48.0";
const SEMVER_BASELINE_REFERENCE: &str = "v0.5.0^{commit}";
const SEMVER_BASELINE_COMMIT: &str = "a3d1e2a660abb2c930ecaad4afb46b22d062fa67";
const MIRI_FLAGS: &str = "-Zmiri-strict-provenance -Zmiri-symbolic-alignment-check";
const MIRI_FLAGS_ALLOW_INTENTIONAL_LEAKS: &str =
    "-Zmiri-strict-provenance -Zmiri-symbolic-alignment-check -Zmiri-ignore-leaks";
const TSAN_FOUNDATION_UNIT_TEST: &str = "core::foundation::tests::safe_worldless_native_calls_block_replay_until_transient_leases_drain";
const WASM_CALLBACK_FREE_QUERY_PROBE: &str = "wasm-callback-free-query";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct WasmCallbackBoundaryProbe {
    binary: &'static str,
    unavailable_apis: &'static [WasmUnavailableApi],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WasmUnavailableApi {
    Method(&'static str),
    Trait(&'static str),
}

impl WasmUnavailableApi {
    fn name(self) -> &'static str {
        match self {
            Self::Method(name) | Self::Trait(name) => name,
        }
    }

    fn diagnostic(self) -> String {
        match self {
            Self::Method(name) => format!("no method named `{name}`"),
            Self::Trait(name) => format!("cannot find trait `{name}`"),
        }
    }
}

const WASM_CALLBACK_BOUNDARY_PROBES: &[WasmCallbackBoundaryProbe] = &[
    WasmCallbackBoundaryProbe {
        binary: "wasm-foundation-callback-boundary",
        unavailable_apis: &[
            WasmUnavailableApi::Method("with_assert_hook"),
            WasmUnavailableApi::Method("with_log_hook"),
        ],
    },
    WasmCallbackBoundaryProbe {
        binary: "wasm-world-callback-boundary",
        unavailable_apis: &[
            WasmUnavailableApi::Method("set_custom_filter"),
            WasmUnavailableApi::Method("try_set_custom_filter"),
            WasmUnavailableApi::Method("clear_custom_filter"),
            WasmUnavailableApi::Method("try_clear_custom_filter"),
            WasmUnavailableApi::Method("set_custom_filter_callback"),
            WasmUnavailableApi::Method("try_set_custom_filter_callback"),
            WasmUnavailableApi::Method("set_pre_solve"),
            WasmUnavailableApi::Method("try_set_pre_solve"),
            WasmUnavailableApi::Method("clear_pre_solve"),
            WasmUnavailableApi::Method("try_clear_pre_solve"),
            WasmUnavailableApi::Method("set_pre_solve_callback"),
            WasmUnavailableApi::Method("try_set_pre_solve_callback"),
            WasmUnavailableApi::Method("set_friction_callback"),
            WasmUnavailableApi::Method("try_set_friction_callback"),
            WasmUnavailableApi::Method("clear_friction_callback"),
            WasmUnavailableApi::Method("try_clear_friction_callback"),
            WasmUnavailableApi::Method("set_restitution_callback"),
            WasmUnavailableApi::Method("try_set_restitution_callback"),
            WasmUnavailableApi::Method("clear_restitution_callback"),
            WasmUnavailableApi::Method("try_clear_restitution_callback"),
            WasmUnavailableApi::Method("set_task_system_raw"),
        ],
    },
    WasmCallbackBoundaryProbe {
        binary: "wasm-world-builder-callback-boundary",
        unavailable_apis: &[WasmUnavailableApi::Method("task_system_raw")],
    },
    WasmCallbackBoundaryProbe {
        binary: "wasm-debug-draw-boundary",
        unavailable_apis: &[WasmUnavailableApi::Trait("DebugDraw")],
    },
    WasmCallbackBoundaryProbe {
        binary: "wasm-debug-draw-method-boundary",
        unavailable_apis: &[
            WasmUnavailableApi::Method("debug_draw_collect"),
            WasmUnavailableApi::Method("try_debug_draw_collect"),
            WasmUnavailableApi::Method("debug_draw_collect_into"),
            WasmUnavailableApi::Method("try_debug_draw_collect_into"),
            WasmUnavailableApi::Method("debug_draw"),
            WasmUnavailableApi::Method("try_debug_draw"),
        ],
    },
    WasmCallbackBoundaryProbe {
        binary: "wasm-query-callback-boundary",
        unavailable_apis: &[
            WasmUnavailableApi::Method("overlap_aabb"),
            WasmUnavailableApi::Method("overlap_aabb_into"),
            WasmUnavailableApi::Method("visit_overlap_aabb"),
            WasmUnavailableApi::Method("try_overlap_aabb"),
            WasmUnavailableApi::Method("try_overlap_aabb_into"),
            WasmUnavailableApi::Method("try_visit_overlap_aabb"),
            WasmUnavailableApi::Method("overlap_polygon_points"),
            WasmUnavailableApi::Method("overlap_polygon_points_into"),
            WasmUnavailableApi::Method("visit_overlap_polygon_points"),
            WasmUnavailableApi::Method("try_overlap_polygon_points"),
            WasmUnavailableApi::Method("try_overlap_polygon_points_into"),
            WasmUnavailableApi::Method("try_visit_overlap_polygon_points"),
            WasmUnavailableApi::Method("overlap_polygon_points_with_offset"),
            WasmUnavailableApi::Method("overlap_polygon_points_with_offset_into"),
            WasmUnavailableApi::Method("visit_overlap_polygon_points_with_offset"),
            WasmUnavailableApi::Method("try_overlap_polygon_points_with_offset"),
            WasmUnavailableApi::Method("try_overlap_polygon_points_with_offset_into"),
            WasmUnavailableApi::Method("try_visit_overlap_polygon_points_with_offset"),
            WasmUnavailableApi::Method("cast_ray_all"),
            WasmUnavailableApi::Method("cast_ray_all_into"),
            WasmUnavailableApi::Method("try_cast_ray_all"),
            WasmUnavailableApi::Method("try_cast_ray_all_into"),
            WasmUnavailableApi::Method("cast_shape_points"),
            WasmUnavailableApi::Method("cast_shape_points_into"),
            WasmUnavailableApi::Method("try_cast_shape_points"),
            WasmUnavailableApi::Method("try_cast_shape_points_into"),
            WasmUnavailableApi::Method("cast_shape_points_with_offset"),
            WasmUnavailableApi::Method("cast_shape_points_with_offset_into"),
            WasmUnavailableApi::Method("try_cast_shape_points_with_offset"),
            WasmUnavailableApi::Method("try_cast_shape_points_with_offset_into"),
            WasmUnavailableApi::Method("collide_mover"),
            WasmUnavailableApi::Method("collide_mover_into"),
            WasmUnavailableApi::Method("try_collide_mover"),
            WasmUnavailableApi::Method("try_collide_mover_into"),
        ],
    },
    WasmCallbackBoundaryProbe {
        binary: "wasm-handle-query-callback-boundary",
        unavailable_apis: &[
            WasmUnavailableApi::Method("overlap_aabb"),
            WasmUnavailableApi::Method("overlap_aabb_into"),
            WasmUnavailableApi::Method("visit_overlap_aabb"),
            WasmUnavailableApi::Method("try_overlap_aabb"),
            WasmUnavailableApi::Method("try_overlap_aabb_into"),
            WasmUnavailableApi::Method("try_visit_overlap_aabb"),
            WasmUnavailableApi::Method("overlap_polygon_points"),
            WasmUnavailableApi::Method("overlap_polygon_points_into"),
            WasmUnavailableApi::Method("visit_overlap_polygon_points"),
            WasmUnavailableApi::Method("try_overlap_polygon_points"),
            WasmUnavailableApi::Method("try_overlap_polygon_points_into"),
            WasmUnavailableApi::Method("try_visit_overlap_polygon_points"),
            WasmUnavailableApi::Method("overlap_polygon_points_with_offset"),
            WasmUnavailableApi::Method("overlap_polygon_points_with_offset_into"),
            WasmUnavailableApi::Method("visit_overlap_polygon_points_with_offset"),
            WasmUnavailableApi::Method("try_overlap_polygon_points_with_offset"),
            WasmUnavailableApi::Method("try_overlap_polygon_points_with_offset_into"),
            WasmUnavailableApi::Method("try_visit_overlap_polygon_points_with_offset"),
            WasmUnavailableApi::Method("cast_ray_all"),
            WasmUnavailableApi::Method("cast_ray_all_into"),
            WasmUnavailableApi::Method("try_cast_ray_all"),
            WasmUnavailableApi::Method("try_cast_ray_all_into"),
            WasmUnavailableApi::Method("cast_shape_points"),
            WasmUnavailableApi::Method("cast_shape_points_into"),
            WasmUnavailableApi::Method("try_cast_shape_points"),
            WasmUnavailableApi::Method("try_cast_shape_points_into"),
            WasmUnavailableApi::Method("cast_shape_points_with_offset"),
            WasmUnavailableApi::Method("cast_shape_points_with_offset_into"),
            WasmUnavailableApi::Method("try_cast_shape_points_with_offset"),
            WasmUnavailableApi::Method("try_cast_shape_points_with_offset_into"),
            WasmUnavailableApi::Method("collide_mover"),
            WasmUnavailableApi::Method("collide_mover_into"),
            WasmUnavailableApi::Method("try_collide_mover"),
            WasmUnavailableApi::Method("try_collide_mover_into"),
        ],
    },
    WasmCallbackBoundaryProbe {
        binary: "wasm-recording-query-callback-boundary",
        unavailable_apis: &[
            WasmUnavailableApi::Method("overlap_aabb"),
            WasmUnavailableApi::Method("try_overlap_aabb"),
            WasmUnavailableApi::Method("overlap_polygon_points"),
            WasmUnavailableApi::Method("try_overlap_polygon_points"),
            WasmUnavailableApi::Method("cast_ray_all"),
            WasmUnavailableApi::Method("try_cast_ray_all"),
            WasmUnavailableApi::Method("cast_shape_points"),
            WasmUnavailableApi::Method("try_cast_shape_points"),
            WasmUnavailableApi::Method("collide_mover"),
            WasmUnavailableApi::Method("try_collide_mover"),
        ],
    },
    WasmCallbackBoundaryProbe {
        binary: "wasm-dynamic-tree-callback-boundary",
        unavailable_apis: &[
            WasmUnavailableApi::Method("query"),
            WasmUnavailableApi::Method("try_query"),
            WasmUnavailableApi::Method("query_all"),
            WasmUnavailableApi::Method("try_query_all"),
            WasmUnavailableApi::Method("ray_cast"),
            WasmUnavailableApi::Method("try_ray_cast"),
            WasmUnavailableApi::Method("box_cast"),
            WasmUnavailableApi::Method("try_box_cast"),
        ],
    },
    WasmCallbackBoundaryProbe {
        binary: "wasm-replay-callback-boundary",
        unavailable_apis: &[
            WasmUnavailableApi::Method("with_friction_mixer"),
            WasmUnavailableApi::Method("with_restitution_mixer"),
            WasmUnavailableApi::Method("draw"),
        ],
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct IntentionalLeakTest {
    target: &'static str,
    filter: &'static str,
}

const ASAN_INTENTIONAL_LEAK_TESTS: &[IntentionalLeakTest] = &[
    IntentionalLeakTest {
        target: "owned_destruction",
        filter: "query_preserves_primary_panic_while_flushing_another_world",
    },
    IntentionalLeakTest {
        target: "owned_destruction",
        filter: "query_preserves_visitor_panic_when_native_guard_triggers_panicking_world_teardown",
    },
    IntentionalLeakTest {
        target: "replay",
        filter: "replay_mixer_drop_panics_run_all_cleanup_before_resuming",
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MiriSuite {
    filter: &'static str,
    allows_intentional_leaks: bool,
}

const MIRI_SUITES: &[MiriSuite] = &[
    MiriSuite {
        filter: "core::ffi_vec::tests",
        allows_intentional_leaks: false,
    },
    MiriSuite {
        filter: "core::callback_state::tests::owner_cleanup_runs_only_after_the_outer_callback_returns",
        allows_intentional_leaks: false,
    },
    MiriSuite {
        filter: "core::callback_state::tests::callback_without_owner_frame_retains_cleanup_without_running_or_dropping_it",
        allows_intentional_leaks: true,
    },
    MiriSuite {
        filter: "core::callback_state::tests::nested_owner_scope_without_outer_frame_retains_cleanup_at_unsafe_boundary",
        allows_intentional_leaks: true,
    },
    MiriSuite {
        filter: "core::callback_state::tests::concurrent_worker_panics_keep_one_payload_and_leak_losers",
        allows_intentional_leaks: true,
    },
    MiriSuite {
        filter: "core::foundation::tests::activity_counter_exhaustion_does_not_wrap",
        allows_intentional_leaks: true,
    },
    MiriSuite {
        filter: "events::tests::ffi_slice_accepts_empty_null_and_rejects_broken_pairs",
        allows_intentional_leaks: false,
    },
    MiriSuite {
        filter: "recording::tests::recording_capacity_checks_native_signed_boundary",
        allows_intentional_leaks: false,
    },
    MiriSuite {
        filter: "snapshot::tests::native_payload_length_is_bounded_before_ffi_conversion",
        allows_intentional_leaks: false,
    },
    MiriSuite {
        filter: "core::identity_registry::tests::dropped_restore_plan_leaves_active_state_but_consumes_nonces",
        allows_intentional_leaks: false,
    },
    MiriSuite {
        filter: "core::identity_registry::tests::restore_preserves_only_the_exact_registration_intersection",
        allows_intentional_leaks: false,
    },
    MiriSuite {
        filter: "replay::preflight::lifecycle::tests::pool_reuse_invalidates_the_old_generation_and_preserves_lifo_order",
        allows_intentional_leaks: false,
    },
    MiriSuite {
        filter: "replay::preflight::tests::explosion_preflight_rejects_double_positions_outside_native_query_bounds",
        allows_intentional_leaks: false,
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FeatureCoordinate {
    package: &'static str,
    features: &'static str,
}

const FEATURE_COORDINATES: &[FeatureCoordinate] = &[
    FeatureCoordinate {
        package: "boxdd-sys",
        features: "",
    },
    FeatureCoordinate {
        package: "boxdd-sys",
        features: "double-precision",
    },
    FeatureCoordinate {
        package: "boxdd-sys",
        features: "validate",
    },
    FeatureCoordinate {
        package: "boxdd-sys",
        features: "double-precision validate",
    },
    FeatureCoordinate {
        package: "boxdd-sys",
        features: "disable-simd",
    },
    FeatureCoordinate {
        package: "boxdd-sys",
        features: "double-precision disable-simd",
    },
    FeatureCoordinate {
        package: "boxdd",
        features: "",
    },
    FeatureCoordinate {
        package: "boxdd",
        features: "serde",
    },
    FeatureCoordinate {
        package: "boxdd",
        features: "mint",
    },
    FeatureCoordinate {
        package: "boxdd",
        features: "nalgebra",
    },
    FeatureCoordinate {
        package: "boxdd",
        features: "glam",
    },
    FeatureCoordinate {
        package: "boxdd",
        features: "bytemuck",
    },
    FeatureCoordinate {
        package: "boxdd",
        features: "bytemuck glam",
    },
    FeatureCoordinate {
        package: "boxdd",
        features: "unchecked",
    },
    FeatureCoordinate {
        package: "boxdd",
        features: "validate",
    },
    FeatureCoordinate {
        package: "boxdd",
        features: "double-precision",
    },
    FeatureCoordinate {
        package: "boxdd",
        features: "double-precision serde mint nalgebra glam bytemuck",
    },
    FeatureCoordinate {
        package: "boxdd",
        features: "double-precision unchecked validate disable-simd",
    },
    FeatureCoordinate {
        package: "bevy_boxdd",
        features: "",
    },
    FeatureCoordinate {
        package: "bevy_boxdd",
        features: "double-precision",
    },
];

pub fn verify_feature_matrix(root: &Path, args: &[String]) -> Result<()> {
    require_check_args("verify-feature-matrix", args)?;
    let toolchain = env::var("BOXDD_VERIFY_TOOLCHAIN").unwrap_or_else(|_| "1.97.1".to_owned());
    verify_toolchain_exists(&toolchain)?;

    for coordinate in feature_coordinates_for_host() {
        let mut command = cargo_command(&toolchain);
        command.current_dir(root).args([
            "check",
            "--locked",
            "--all-targets",
            "-p",
            coordinate.package,
        ]);
        if !coordinate.features.is_empty() {
            command.args(["--features", coordinate.features]);
        }
        run_command(
            &mut command,
            &format!(
                "feature matrix coordinate {} [{}] on Rust {toolchain}",
                coordinate.package,
                if coordinate.features.is_empty() {
                    "default"
                } else {
                    coordinate.features
                }
            ),
        )?;
    }
    Ok(())
}

pub fn verify_compile_fail(root: &Path, args: &[String]) -> Result<()> {
    require_check_args("verify-compile-fail", args)?;
    for features in [None, Some("double-precision"), Some("serde")] {
        let mut command = Command::new("cargo");
        command.current_dir(root).args([
            "nextest",
            "run",
            "--locked",
            "-p",
            "boxdd",
            "--test",
            "compile_fail",
        ]);
        if let Some(features) = features {
            command.args(["--features", features]);
        }
        run_command(&mut command, "compile-fail ownership contract")?;
    }
    Ok(())
}

pub fn verify_wasm(root: &Path, args: &[String]) -> Result<()> {
    match args {
        [mode] if mode == "--compile-only" => verify_wasm_compile_only(root),
        [mode] if mode == "--runtime" => verify_wasm_runtime(root),
        _ => Err(Error::message(
            "verify-wasm expects exactly --compile-only or --runtime",
        )),
    }
}

fn verify_wasm_compile_only(root: &Path) -> Result<()> {
    let toolchain = verification_toolchain();
    verify_toolchain_exists(&toolchain)?;
    for target in ["wasm32-unknown-unknown", "wasm32-wasip1"] {
        require_rust_target(&toolchain, target)?;
        for features in [
            None,
            Some("double-precision"),
            Some("disable-simd"),
            Some("double-precision,disable-simd"),
            Some("simd-avx2"),
            Some("double-precision,simd-avx2"),
        ] {
            let mut command = cargo_command(&toolchain);
            command
                .current_dir(root)
                .env("BOXDD_SYS_PROVIDER", "wasm-compile-only")
                .args(["check", "--locked", "-p", "boxdd-sys", "--target", target]);
            if let Some(features) = features {
                command.args(["--features", features]);
            }
            run_command(&mut command, &format!("WASM compile contract for {target}"))?;
        }

        for features in [None, Some("double-precision")] {
            let mut command = cargo_command(&toolchain);
            command
                .current_dir(root)
                .env("BOXDD_SYS_PROVIDER", "wasm-compile-only")
                .args([
                    "check", "--locked", "-p", "boxdd", "--lib", "--target", target,
                ]);
            if let Some(features) = features {
                command.args(["--features", features]);
            }
            run_command(
                &mut command,
                &format!("Safe Rust WASM compile contract for {target}"),
            )?;

            let mut callback_free = wasm_probe_command(
                root,
                &toolchain,
                target,
                features,
                WASM_CALLBACK_FREE_QUERY_PROBE,
            );
            run_command(
                &mut callback_free,
                &format!("callback-free Safe Rust WASM query contract for {target}"),
            )?;

            for probe in WASM_CALLBACK_BOUNDARY_PROBES {
                verify_wasm_callback_boundary(root, &toolchain, target, features, *probe)?;
            }
        }
    }
    Ok(())
}

fn verify_wasm_callback_boundary(
    root: &Path,
    toolchain: &str,
    target: &str,
    features: Option<&str>,
    probe: WasmCallbackBoundaryProbe,
) -> Result<()> {
    let mut command = wasm_probe_command(root, toolchain, target, features, probe.binary);
    let output = command
        .output()
        .map_err(|source| Error::io(format!("cargo check {}", probe.binary), source))?;
    if output.status.success() {
        return Err(Error::message(format!(
            "WASM callback boundary probe `{}` unexpectedly compiled for {target}",
            probe.binary
        )));
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    for api in probe.unavailable_apis {
        let diagnostic = api.diagnostic();
        if !stderr.contains(&diagnostic) {
            return Err(Error::message(format!(
                "WASM callback boundary probe `{}` failed without proving `{}` unavailable for {target}: {}",
                probe.binary,
                api.name(),
                stderr.trim()
            )));
        }
    }
    Ok(())
}

fn wasm_probe_command(
    root: &Path,
    toolchain: &str,
    target: &str,
    features: Option<&str>,
    binary: &str,
) -> Command {
    let mut command = cargo_command(toolchain);
    command
        .current_dir(root)
        .env("BOXDD_SYS_PROVIDER", "wasm-compile-only")
        .args([
            "check",
            "--locked",
            "-p",
            "boxdd-provider-smoke",
            "--bin",
            binary,
            "--target",
            target,
        ]);
    if let Some(features) = features {
        command.args(["--features", features]);
    }
    command
}

fn verify_wasm_runtime(root: &Path) -> Result<()> {
    let target_dir = cargo_target_dir(root)?;
    for precision in [ProviderPrecision::Single, ProviderPrecision::Double] {
        // The provider command validates the pinned Emscripten and wasm-bindgen identities before
        // compiling, so a missing or stale SDK is an explicit qualification failure.
        let sdk = provider::provider_smoke_for_precision(root, precision)?;
        let mut browser = browser_provider_smoke_command(root, &target_dir, precision, &sdk)?;
        run_command(
            &mut browser,
            &format!(
                "Chromium provider shared-memory smoke ({})",
                precision.as_str()
            ),
        )?;
    }
    Ok(())
}

fn browser_provider_smoke_command(
    root: &Path,
    target_dir: &Path,
    precision: ProviderPrecision,
    sdk: &crate::emscripten_sdk::QualifiedEmscriptenSdk,
) -> Result<Command> {
    let command = sdk.npm_command().map_err(Error::message)?;
    Ok(configure_browser_provider_smoke_command(
        command, root, target_dir, precision,
    ))
}

fn configure_browser_provider_smoke_command(
    mut command: Command,
    root: &Path,
    target_dir: &Path,
    precision: ProviderPrecision,
) -> Command {
    command
        .current_dir(root)
        .args(["run", "test:browser"])
        .env("BOXDD_WASM_PRECISION", precision.as_str())
        .env("CARGO_TARGET_DIR", target_dir);
    command
}

pub fn verify_miri(root: &Path, args: &[String]) -> Result<()> {
    require_check_args("verify-miri", args)?;
    verify_toolchain_exists(VERIFICATION_NIGHTLY)?;
    require_rust_component(VERIFICATION_NIGHTLY, "miri")?;
    require_rust_component(VERIFICATION_NIGHTLY, "rust-src")?;

    for suite in MIRI_SUITES {
        let mut command = cargo_command(VERIFICATION_NIGHTLY);
        command
            .current_dir(root)
            .env(
                "MIRIFLAGS",
                if suite.allows_intentional_leaks {
                    MIRI_FLAGS_ALLOW_INTENTIONAL_LEAKS
                } else {
                    MIRI_FLAGS
                },
            )
            .args([
                "miri",
                "test",
                "--locked",
                "-p",
                "boxdd",
                "--lib",
                suite.filter,
            ]);
        run_command(
            &mut command,
            &format!("Miri pure-Rust suite {}", suite.filter),
        )?;
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Sanitizer {
    Address,
    Undefined,
    Thread,
}

impl Sanitizer {
    fn parse(args: &[String]) -> Result<Self> {
        match args {
            [value] if value == "--address" => Ok(Self::Address),
            [value] if value == "--undefined" => Ok(Self::Undefined),
            [value] if value == "--thread" => Ok(Self::Thread),
            _ => Err(Error::message(
                "verify-sanitizers expects exactly --address, --undefined, or --thread",
            )),
        }
    }

    const fn rust_flag(self) -> Option<&'static str> {
        match self {
            Self::Address => Some("address"),
            // Rust has no UBSan instrumentation mode. C is instrumented and Rust retains overflow,
            // alignment, and debug assertions while driving the mixed FFI paths.
            Self::Undefined => None,
            Self::Thread => Some("thread"),
        }
    }

    const fn c_flag(self) -> &'static str {
        match self {
            Self::Address => "-fsanitize=address -fno-omit-frame-pointer",
            Self::Undefined => {
                "-fsanitize=undefined -fno-sanitize-recover=undefined -fno-omit-frame-pointer"
            }
            Self::Thread => "-fsanitize=thread -fno-omit-frame-pointer",
        }
    }

    const fn tests(self) -> &'static [&'static str] {
        match self {
            Self::Address | Self::Undefined => &[
                "--test",
                "owned_destruction",
                "--test",
                "world_and_queries",
                "--test",
                "user_data",
                "--test",
                "snapshot",
                "--test",
                "recording",
                "--test",
                "replay",
                "--test",
                "buffer_reuse",
                "--test",
                "events_and_sensors",
            ],
            Self::Thread => &[
                "--test",
                "foundation_world_activity",
                "--test",
                "worker_callbacks_multithread",
                "--test",
                "world_operational",
            ],
        }
    }
}

pub fn verify_sanitizers(root: &Path, args: &[String]) -> Result<()> {
    let sanitizer = Sanitizer::parse(args)?;
    verify_toolchain_exists(VERIFICATION_NIGHTLY)?;
    if sanitizer == Sanitizer::Thread {
        require_rust_component(VERIFICATION_NIGHTLY, "rust-src")?;
    }
    let host = rustc_host(VERIFICATION_NIGHTLY)?;
    verify_c_sanitizer(sanitizer)?;
    let target_dir = root
        .join("target")
        .join(format!("sanitizer-{}", sanitizer_label(sanitizer)));

    let mut command = sanitizer_command(root, sanitizer, &host, &target_dir, false);
    command.args(sanitizer.tests());
    if sanitizer == Sanitizer::Address {
        command.arg("--");
        for test in ASAN_INTENTIONAL_LEAK_TESTS {
            command.args(["--skip", test.filter]);
        }
    }
    run_command(
        &mut command,
        &format!("{} sanitizer suite", sanitizer_label(sanitizer)),
    )?;

    if sanitizer == Sanitizer::Thread {
        let mut command = thread_foundation_unit_test_command(root, &host, &target_dir);
        run_command(
            &mut command,
            "thread sanitizer safe worldless foundation exclusion proof",
        )?;
    }

    if sanitizer == Sanitizer::Address {
        for test in ASAN_INTENTIONAL_LEAK_TESTS {
            let mut command = sanitizer_command(root, sanitizer, &host, &target_dir, true);
            command.args(["--test", test.target, "--", test.filter, "--exact"]);
            run_command(
                &mut command,
                &format!("address sanitizer intentional-leak test {}", test.filter),
            )?;
        }
    }

    Ok(())
}

fn thread_foundation_unit_test_command(root: &Path, host: &str, target_dir: &Path) -> Command {
    let mut command = sanitizer_command(root, Sanitizer::Thread, host, target_dir, false);
    command.args(["--lib", "--", TSAN_FOUNDATION_UNIT_TEST, "--exact"]);
    command
}

fn sanitizer_command(
    root: &Path,
    sanitizer: Sanitizer,
    host: &str,
    target_dir: &Path,
    allow_intentional_leaks: bool,
) -> Command {
    let mut command = cargo_command(VERIFICATION_NIGHTLY);
    command
        .current_dir(root)
        .env("CFLAGS", sanitizer.c_flag())
        .env("CXXFLAGS", sanitizer.c_flag())
        .env("CARGO_TARGET_DIR", target_dir)
        .args(["nextest", "run"]);
    if sanitizer == Sanitizer::Thread {
        command.args(["-Z", "build-std"]);
    }
    command.args([
        "--locked",
        "--no-fail-fast",
        "-p",
        "boxdd",
        "--target",
        host,
    ]);
    if let Some(kind) = sanitizer.rust_flag() {
        command.env(
            "RUSTFLAGS",
            format!("-Zsanitizer={kind} -Cforce-frame-pointers=yes"),
        );
    }
    match sanitizer {
        Sanitizer::Address => {
            command.env(
                "ASAN_OPTIONS",
                if allow_intentional_leaks {
                    "detect_leaks=0:halt_on_error=1:strict_string_checks=1"
                } else {
                    "detect_leaks=1:halt_on_error=1:strict_string_checks=1"
                },
            );
        }
        Sanitizer::Undefined => {
            command.env("UBSAN_OPTIONS", "halt_on_error=1:print_stacktrace=1");
            // Rust's linker uses -nodefaultlibs, so the C UBSan runtime must be explicit.
            command.env(
                "RUSTFLAGS",
                "-Clink-arg=-fsanitize=undefined -Clink-arg=-lubsan",
            );
        }
        Sanitizer::Thread => {
            command.env("TSAN_OPTIONS", "halt_on_error=1:second_deadlock_stack=1");
        }
    }
    command
}

pub fn verify_semver(root: &Path, args: &[String]) -> Result<()> {
    require_check_args("verify-semver", args)?;
    verify_semver_baseline(root)?;

    let output = Command::new("cargo")
        .args(["semver-checks", "--version"])
        .output()
        .map_err(|source| Error::io("cargo semver-checks --version", source))?;
    require_success(&output, "cargo semver-checks --version")?;
    let version = String::from_utf8_lossy(&output.stdout);
    if !version
        .split_whitespace()
        .any(|token| token == SEMVER_CHECKS_VERSION)
    {
        return Err(Error::message(format!(
            "SemVer verification requires cargo-semver-checks {SEMVER_CHECKS_VERSION}; found {}",
            version.trim()
        )));
    }

    for manifest in [
        "boxdd-sys/Cargo.toml",
        "boxdd/Cargo.toml",
        "bevy_boxdd/Cargo.toml",
    ] {
        let mut command = semver_check_command(root, manifest, None);
        run_command(&mut command, &format!("SemVer contract for {manifest}"))?;

        let output = semver_check_command(root, manifest, Some("patch"))
            .output()
            .map_err(|source| {
                Error::io(format!("SemVer patch rejection for {manifest}"), source)
            })?;
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if output.status.success() || !diagnostic.contains("semver requires new major version:") {
            return Err(Error::message(format!(
                "SemVer patch-negative contract for {manifest} did not report the expected breaking-API rejection (status {})",
                output.status
            )));
        }
    }
    Ok(())
}

fn verify_semver_baseline(root: &Path) -> Result<()> {
    let mut command = semver_baseline_resolution_command(root)?;
    let output = command
        .output()
        .map_err(|source| Error::io("resolve pinned SemVer baseline", source))?;
    require_success(&output, "resolve pinned SemVer baseline")?;
    require_semver_baseline_identity(&output.stdout)
}

fn semver_baseline_resolution_command(root: &Path) -> Result<Command> {
    let mut command = qualified_git_command().map_err(Error::message)?;
    command.current_dir(root).args([
        "rev-parse",
        "--verify",
        "--end-of-options",
        SEMVER_BASELINE_REFERENCE,
    ]);
    Ok(command)
}

fn require_semver_baseline_identity(stdout: &[u8]) -> Result<()> {
    let resolved = std::str::from_utf8(stdout).map_err(|error| {
        Error::message(format!(
            "qualified Git returned non-UTF-8 output for SemVer baseline {SEMVER_BASELINE_REFERENCE}: {error}"
        ))
    })?;
    let mut lines = resolved.lines();
    let actual = lines.next().unwrap_or_default();
    if actual == SEMVER_BASELINE_COMMIT && lines.next().is_none() {
        return Ok(());
    }
    Err(Error::message(format!(
        "SemVer baseline {SEMVER_BASELINE_REFERENCE} must resolve exactly to {SEMVER_BASELINE_COMMIT}; found {resolved:?}"
    )))
}

fn semver_check_command(root: &Path, manifest: &str, release_type: Option<&str>) -> Command {
    let mut command = Command::new("cargo");
    command.current_dir(root).args([
        "semver-checks",
        "check-release",
        "--manifest-path",
        manifest,
        "--baseline-rev",
        SEMVER_BASELINE_COMMIT,
        "--color",
        "never",
    ]);
    if let Some(release_type) = release_type {
        command.args(["--release-type", release_type]);
    }
    command
}

fn feature_coordinates_for_host() -> Vec<FeatureCoordinate> {
    let mut coordinates = FEATURE_COORDINATES.to_vec();
    if env::consts::ARCH == "x86_64" {
        coordinates.extend([
            FeatureCoordinate {
                package: "boxdd-sys",
                features: "simd-avx2",
            },
            FeatureCoordinate {
                package: "boxdd-sys",
                features: "double-precision simd-avx2",
            },
        ]);
    }
    coordinates
}

fn cargo_command(toolchain: &str) -> Command {
    let mut command = Command::new("cargo");
    command.arg(format!("+{toolchain}"));
    command
}

fn verification_toolchain() -> String {
    env::var("BOXDD_VERIFY_TOOLCHAIN")
        .or_else(|_| env::var("RUSTUP_TOOLCHAIN"))
        .unwrap_or_else(|_| "1.97.1".to_owned())
}

fn verify_toolchain_exists(toolchain: &str) -> Result<()> {
    let output = Command::new("rustup")
        .args(["run", toolchain, "rustc", "--version"])
        .output()
        .map_err(|source| Error::io("rustup toolchain probe", source))?;
    require_success(&output, &format!("required Rust toolchain {toolchain}"))
}

fn require_rust_target(toolchain: &str, target: &str) -> Result<()> {
    let output = Command::new("rustup")
        .args(["target", "list", "--installed", "--toolchain", toolchain])
        .output()
        .map_err(|source| Error::io("rustup target list", source))?;
    require_success(&output, "rustup target list")?;
    let installed = String::from_utf8_lossy(&output.stdout);
    if installed.lines().any(|line| line.trim() == target) {
        Ok(())
    } else {
        Err(Error::message(format!(
            "WASM verification requires Rust target {target} for toolchain {toolchain}; CI must install it and local runs must not silently skip"
        )))
    }
}

fn require_rust_component(toolchain: &str, component: &str) -> Result<()> {
    let output = Command::new("rustup")
        .args(["component", "list", "--toolchain", toolchain])
        .output()
        .map_err(|source| Error::io("rustup component list", source))?;
    require_success(&output, "rustup component list")?;
    let components = String::from_utf8_lossy(&output.stdout);
    if components
        .lines()
        .any(|line| line.starts_with(component) && line.ends_with("(installed)"))
    {
        Ok(())
    } else {
        Err(Error::message(format!(
            "verification requires {component} installed for {toolchain}; CI must install it and local runs must not silently skip"
        )))
    }
}

fn rustc_host(toolchain: &str) -> Result<String> {
    let output = Command::new("rustup")
        .args(["run", toolchain, "rustc", "-vV"])
        .output()
        .map_err(|source| Error::io("rustc -vV", source))?;
    require_success(&output, "rustc -vV")?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .map(ToOwned::to_owned)
        .ok_or_else(|| Error::message("rustc -vV did not report a host target"))
}

fn verify_c_sanitizer(sanitizer: Sanitizer) -> Result<()> {
    let compiler = env::var("CC").unwrap_or_else(|_| "cc".to_owned());
    let mut command = Command::new(&compiler);
    command
        .args(sanitizer.c_flag().split_whitespace())
        .args(["-x", "c", "-fsyntax-only", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|source| Error::io(format!("{compiler} sanitizer probe"), source))?;
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin
            .write_all(b"int main(void) { return 0; }\n")
            .map_err(|source| Error::io(format!("write {compiler} sanitizer probe"), source))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|source| Error::io(format!("wait for {compiler} sanitizer probe"), source))?;
    require_success(
        &output,
        &format!(
            "C compiler support for {} sanitizer",
            sanitizer_label(sanitizer)
        ),
    )
}

const fn sanitizer_label(sanitizer: Sanitizer) -> &'static str {
    match sanitizer {
        Sanitizer::Address => "address",
        Sanitizer::Undefined => "undefined",
        Sanitizer::Thread => "thread",
    }
}

fn require_success(output: &Output, label: &str) -> Result<()> {
    if output.status.success() {
        Ok(())
    } else {
        Err(Error::message(format!(
            "{label} failed with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

fn require_check_args(command: &str, args: &[String]) -> Result<()> {
    match args {
        [] => Ok(()),
        [value] if value == "--check" => Ok(()),
        _ => Err(Error::message(format!(
            "{command} accepts no arguments other than optional --check"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::ffi::OsStr;

    use super::*;

    #[test]
    fn feature_matrix_is_explicit_and_never_uses_removed_or_ambiguous_features() {
        let coordinates = FEATURE_COORDINATES
            .iter()
            .map(|coordinate| (coordinate.package, coordinate.features))
            .collect::<BTreeSet<_>>();
        assert_eq!(coordinates.len(), FEATURE_COORDINATES.len());

        for coordinate in FEATURE_COORDINATES {
            assert!(
                !coordinate
                    .features
                    .split_whitespace()
                    .any(|feature| feature == "serialize")
            );
            assert!(!coordinate.features.contains("all-features"));
        }

        for features in [
            "serde",
            "mint",
            "nalgebra",
            "glam",
            "bytemuck",
            "double-precision",
            "double-precision serde mint nalgebra glam bytemuck",
            "double-precision unchecked validate disable-simd",
        ] {
            assert!(coordinates.contains(&("boxdd", features)));
        }
        for features in [
            "",
            "double-precision",
            "validate",
            "double-precision validate",
            "disable-simd",
            "double-precision disable-simd",
        ] {
            assert!(coordinates.contains(&("boxdd-sys", features)));
        }
        for features in ["", "double-precision"] {
            assert!(coordinates.contains(&("bevy_boxdd", features)));
        }
    }

    #[test]
    fn sanitizers_instrument_c_and_only_use_supported_rust_modes() {
        assert_eq!(Sanitizer::Address.rust_flag(), Some("address"));
        assert_eq!(Sanitizer::Undefined.rust_flag(), None);
        assert_eq!(Sanitizer::Thread.rust_flag(), Some("thread"));
        for sanitizer in [Sanitizer::Address, Sanitizer::Undefined, Sanitizer::Thread] {
            assert!(sanitizer.c_flag().contains("-fsanitize="));
            assert!(!sanitizer.tests().is_empty());
        }
        for required in ["buffer_reuse", "events_and_sensors"] {
            assert!(Sanitizer::Address.tests().contains(&required));
            assert!(Sanitizer::Undefined.tests().contains(&required));
        }
    }

    #[test]
    fn address_sanitizer_leak_allowlist_is_exact_and_narrow() {
        assert_eq!(
            ASAN_INTENTIONAL_LEAK_TESTS,
            [
                IntentionalLeakTest {
                    target: "owned_destruction",
                    filter: "query_preserves_primary_panic_while_flushing_another_world",
                },
                IntentionalLeakTest {
                    target: "owned_destruction",
                    filter: "query_preserves_visitor_panic_when_native_guard_triggers_panicking_world_teardown",
                },
                IntentionalLeakTest {
                    target: "replay",
                    filter: "replay_mixer_drop_panics_run_all_cleanup_before_resuming",
                },
            ]
        );
    }

    #[test]
    fn sanitizer_commands_bind_required_runtimes_and_standard_library_mode() {
        let root = Path::new("/workspace/boxdd");
        let target = Path::new("/tmp/boxdd-sanitizer");

        let address = sanitizer_command(
            root,
            Sanitizer::Address,
            "aarch64-unknown-linux-gnu",
            target,
            false,
        );
        assert!(address.get_envs().any(|(key, value)| {
            key == OsStr::new("RUSTFLAGS")
                && value
                    .and_then(OsStr::to_str)
                    .is_some_and(|flags| flags.contains("-Zsanitizer=address"))
        }));

        let undefined = sanitizer_command(
            root,
            Sanitizer::Undefined,
            "aarch64-unknown-linux-gnu",
            target,
            false,
        );
        assert!(undefined.get_envs().any(|(key, value)| {
            key == OsStr::new("RUSTFLAGS")
                && value
                    .and_then(OsStr::to_str)
                    .is_some_and(|flags| flags.contains("-Clink-arg=-lubsan"))
        }));

        let thread = sanitizer_command(
            root,
            Sanitizer::Thread,
            "aarch64-unknown-linux-gnu",
            target,
            false,
        );
        let args = thread.get_args().collect::<Vec<_>>();
        assert!(
            args.windows(2)
                .any(|window| window == [OsStr::new("-Z"), OsStr::new("build-std")])
        );
        assert!(thread.get_envs().any(|(key, value)| {
            key == OsStr::new("RUSTFLAGS")
                && value
                    .and_then(OsStr::to_str)
                    .is_some_and(|flags| flags.contains("-Zsanitizer=thread"))
        }));

        let foundation_unit =
            thread_foundation_unit_test_command(root, "aarch64-unknown-linux-gnu", target);
        let args = foundation_unit.get_args().collect::<Vec<_>>();
        assert!(args.contains(&OsStr::new("--lib")));
        assert!(args.contains(&OsStr::new(TSAN_FOUNDATION_UNIT_TEST)));
        assert!(args.ends_with(&[
            OsStr::new("--"),
            OsStr::new(TSAN_FOUNDATION_UNIT_TEST),
            OsStr::new("--exact"),
        ]));
    }

    #[test]
    fn miri_ignores_leaks_only_for_explicit_retention_contracts() {
        assert!(!MIRI_FLAGS.contains("ignore-leaks"));
        assert!(MIRI_FLAGS_ALLOW_INTENTIONAL_LEAKS.contains("ignore-leaks"));

        let leak_tolerant = MIRI_SUITES
            .iter()
            .filter(|suite| suite.allows_intentional_leaks)
            .map(|suite| suite.filter)
            .collect::<Vec<_>>();
        assert_eq!(
            leak_tolerant,
            [
                "core::callback_state::tests::callback_without_owner_frame_retains_cleanup_without_running_or_dropping_it",
                "core::callback_state::tests::nested_owner_scope_without_outer_frame_retains_cleanup_at_unsafe_boundary",
                "core::callback_state::tests::concurrent_worker_panics_keep_one_payload_and_leak_losers",
                "core::foundation::tests::activity_counter_exhaustion_does_not_wrap",
            ]
        );
        assert!(MIRI_SUITES.iter().any(|suite| {
            suite.filter
                == "core::callback_state::tests::owner_cleanup_runs_only_after_the_outer_callback_returns"
                && !suite.allows_intentional_leaks
        }));
        for required in ["recording::tests", "snapshot::tests", "replay::preflight"] {
            assert!(MIRI_SUITES.iter().any(|suite| {
                suite.filter.contains(required) && !suite.allows_intentional_leaks
            }));
        }
    }

    #[test]
    fn verification_commands_fail_closed_on_unknown_arguments() {
        assert!(require_check_args("gate", &["--skip".to_owned()]).is_err());
        assert!(Sanitizer::parse(&["--memory".to_owned()]).is_err());
    }

    #[test]
    fn semver_commands_bind_the_qualified_immutable_baseline_and_optional_release_type() {
        let root = Path::new("/workspace/boxdd");
        let baseline = semver_baseline_resolution_command(root).expect("qualified Git command");
        assert_eq!(baseline.get_current_dir(), Some(root));
        assert!(baseline.get_args().collect::<Vec<_>>().ends_with(&[
            OsStr::new("rev-parse"),
            OsStr::new("--verify"),
            OsStr::new("--end-of-options"),
            OsStr::new(SEMVER_BASELINE_REFERENCE),
        ]));

        let inferred = semver_check_command(root, "boxdd/Cargo.toml", None);
        assert_eq!(inferred.get_current_dir(), Some(root));
        assert_eq!(
            inferred
                .get_args()
                .map(|argument| argument.to_str().expect("literal semver argument"))
                .collect::<Vec<_>>(),
            [
                "semver-checks",
                "check-release",
                "--manifest-path",
                "boxdd/Cargo.toml",
                "--baseline-rev",
                SEMVER_BASELINE_COMMIT,
                "--color",
                "never",
            ]
        );

        let patch = semver_check_command(root, "boxdd/Cargo.toml", Some("patch"));
        assert!(
            patch
                .get_args()
                .collect::<Vec<_>>()
                .ends_with(&[OsStr::new("--release-type"), OsStr::new("patch")])
        );
    }

    #[test]
    fn semver_baseline_identity_rejects_tag_drift_and_ambiguous_output() {
        assert!(
            require_semver_baseline_identity(format!("{SEMVER_BASELINE_COMMIT}\n").as_bytes())
                .is_ok()
        );

        let drifted = "0000000000000000000000000000000000000000\n";
        let error = require_semver_baseline_identity(drifted.as_bytes())
            .expect_err("a drifted baseline tag must fail closed");
        let diagnostic = error.to_string();
        assert!(diagnostic.contains(SEMVER_BASELINE_REFERENCE));
        assert!(diagnostic.contains(SEMVER_BASELINE_COMMIT));
        assert!(diagnostic.contains(drifted.trim()));

        let ambiguous = format!("{SEMVER_BASELINE_COMMIT}\n{SEMVER_BASELINE_COMMIT}\n");
        assert!(require_semver_baseline_identity(ambiguous.as_bytes()).is_err());
    }

    #[test]
    fn wasm_runtime_browser_command_preserves_qualified_npm_and_binds_coordinates() {
        let root = Path::new("/workspace/boxdd");
        let target_dir = Path::new("/tmp/boxdd-target");

        for precision in [ProviderPrecision::Single, ProviderPrecision::Double] {
            let mut npm = Command::new("/qualified/node");
            npm.arg("/qualified/npm-cli.js");
            let command =
                configure_browser_provider_smoke_command(npm, root, target_dir, precision);
            assert_eq!(command.get_program(), "/qualified/node");
            assert_eq!(
                command
                    .get_args()
                    .map(|argument| argument.to_str().expect("literal npm argument"))
                    .collect::<Vec<_>>(),
                ["/qualified/npm-cli.js", "run", "test:browser"]
            );
            assert_eq!(command.get_current_dir(), Some(root));
            assert!(command.get_envs().any(|(key, value)| {
                key == OsStr::new("BOXDD_WASM_PRECISION")
                    && value == Some(OsStr::new(precision.as_str()))
            }));
            assert!(command.get_envs().any(|(key, value)| {
                key == OsStr::new("CARGO_TARGET_DIR") && value == Some(target_dir.as_os_str())
            }));
        }
    }

    #[test]
    fn wasm_callback_boundaries_are_explicit_compile_fail_coordinates() {
        use std::collections::HashSet;

        let root = Path::new("/workspace/boxdd");
        let expected = [
            "wasm-foundation-callback-boundary",
            "wasm-world-callback-boundary",
            "wasm-world-builder-callback-boundary",
            "wasm-debug-draw-boundary",
            "wasm-debug-draw-method-boundary",
            "wasm-query-callback-boundary",
            "wasm-handle-query-callback-boundary",
            "wasm-recording-query-callback-boundary",
            "wasm-dynamic-tree-callback-boundary",
            "wasm-replay-callback-boundary",
        ];
        assert_eq!(
            WASM_CALLBACK_BOUNDARY_PROBES
                .iter()
                .map(|probe| probe.binary)
                .collect::<Vec<_>>(),
            expected
        );
        assert!(WASM_CALLBACK_BOUNDARY_PROBES.iter().all(|probe| {
            let names = probe
                .unavailable_apis
                .iter()
                .map(|api| api.name())
                .collect::<Vec<_>>();
            !names.is_empty()
                && names.iter().all(|name| !name.is_empty())
                && names.iter().copied().collect::<HashSet<_>>().len() == names.len()
        }));
        assert_eq!(
            WasmUnavailableApi::Method("draw").diagnostic(),
            "no method named `draw`"
        );
        assert_eq!(
            WasmUnavailableApi::Trait("DebugDraw").diagnostic(),
            "cannot find trait `DebugDraw`"
        );

        let command = wasm_probe_command(
            root,
            "1.97.1",
            "wasm32-unknown-unknown",
            Some("double-precision"),
            expected[1],
        );
        assert_eq!(command.get_current_dir(), Some(root));
        let arguments = command
            .get_args()
            .map(|argument| argument.to_str().expect("literal cargo argument"))
            .collect::<Vec<_>>();
        assert_eq!(arguments.first(), Some(&"+1.97.1"));
        assert!(
            arguments
                .windows(2)
                .any(|pair| pair == ["--bin", expected[1]])
        );
        assert!(
            arguments
                .windows(2)
                .any(|pair| pair == ["--target", "wasm32-unknown-unknown"])
        );
        assert!(
            arguments
                .windows(2)
                .any(|pair| pair == ["--features", "double-precision"])
        );
        assert!(command.get_envs().any(|(key, value)| {
            key == OsStr::new("BOXDD_SYS_PROVIDER")
                && value == Some(OsStr::new("wasm-compile-only"))
        }));

        let positive = wasm_probe_command(
            root,
            "1.95.0",
            "wasm32-wasip1",
            None,
            WASM_CALLBACK_FREE_QUERY_PROBE,
        );
        let positive_arguments = positive
            .get_args()
            .map(|argument| argument.to_str().expect("literal cargo argument"))
            .collect::<Vec<_>>();
        assert_eq!(positive_arguments.first(), Some(&"+1.95.0"));
        assert!(
            positive_arguments
                .windows(2)
                .any(|pair| pair == ["--bin", WASM_CALLBACK_FREE_QUERY_PROBE])
        );
    }
}
