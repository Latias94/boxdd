use std::{
    env,
    path::Path,
    process::{Command, Output},
};

use crate::provider_catalog::ProviderCapability;
use crate::subprocess_policy::{run_output, run_output_with_input};
use crate::{Error, Result};

use super::{
    pages::BEVY_WEB_EXAMPLE,
    provider::{self, ProviderPrecision},
    support::{WASM_TARGET, add_wasm_app_link_args, run_command},
};

const VERIFICATION_NIGHTLY: &str = "nightly-2026-05-27";
const MIRI_FLAGS: &str = "-Zmiri-strict-provenance -Zmiri-symbolic-alignment-check";
const MIRI_FLAGS_ALLOW_INTENTIONAL_LEAKS: &str =
    "-Zmiri-strict-provenance -Zmiri-symbolic-alignment-check -Zmiri-ignore-leaks";
const TSAN_FOUNDATION_UNIT_TEST: &str = "core::foundation::tests::safe_worldless_native_calls_block_replay_until_transient_leases_drain";
const WASM_CALLBACK_FREE_QUERY_PROBE: &str = "wasm-callback-free-query";
const WASM_RECORDING_CALLBACK_FREE_QUERY_PROBE: &str = "wasm-recording-callback-free-query";
const WASM_QUERY_CALLBACK_APIS: &[WasmUnavailableApi] = &[
    WasmUnavailableApi::Method("overlap_aabb"),
    WasmUnavailableApi::Method("overlap_aabb_into"),
    WasmUnavailableApi::Method("visit_overlap_aabb"),
    WasmUnavailableApi::Method("visit_overlap_aabb_with_buffer"),
    WasmUnavailableApi::Method("overlap_shape"),
    WasmUnavailableApi::Method("overlap_shape_into"),
    WasmUnavailableApi::Method("visit_overlap_shape"),
    WasmUnavailableApi::Method("visit_overlap_shape_with_buffer"),
    WasmUnavailableApi::Method("cast_ray_all"),
    WasmUnavailableApi::Method("cast_ray_all_into"),
    WasmUnavailableApi::Method("cast_shape"),
    WasmUnavailableApi::Method("cast_shape_into"),
    WasmUnavailableApi::Method("collide_mover"),
    WasmUnavailableApi::Method("collide_mover_into"),
];

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
            WasmUnavailableApi::Method("clear_custom_filter"),
            WasmUnavailableApi::Method("set_pre_solve"),
            WasmUnavailableApi::Method("clear_pre_solve"),
            WasmUnavailableApi::Method("set_friction_callback"),
            WasmUnavailableApi::Method("clear_friction_callback"),
            WasmUnavailableApi::Method("set_restitution_callback"),
            WasmUnavailableApi::Method("clear_restitution_callback"),
        ],
    },
    WasmCallbackBoundaryProbe {
        binary: "wasm-debug-draw-boundary",
        unavailable_apis: &[WasmUnavailableApi::Trait("DebugDraw")],
    },
    WasmCallbackBoundaryProbe {
        binary: "wasm-debug-draw-method-boundary",
        unavailable_apis: &[
            WasmUnavailableApi::Method("debug_draw_collect"),
            WasmUnavailableApi::Method("debug_draw_collect_into"),
            WasmUnavailableApi::Method("debug_draw"),
        ],
    },
    WasmCallbackBoundaryProbe {
        binary: "wasm-query-callback-boundary",
        unavailable_apis: WASM_QUERY_CALLBACK_APIS,
    },
    WasmCallbackBoundaryProbe {
        binary: "wasm-recording-query-callback-boundary",
        unavailable_apis: WASM_QUERY_CALLBACK_APIS,
    },
    WasmCallbackBoundaryProbe {
        binary: "wasm-dynamic-tree-callback-boundary",
        unavailable_apis: &[
            WasmUnavailableApi::Method("query"),
            WasmUnavailableApi::Method("query_all"),
            WasmUnavailableApi::Method("ray_cast"),
            WasmUnavailableApi::Method("box_cast"),
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
        target: "material_mix_callbacks",
        filter: "outer_unwind_subprocess::material_mix_cleanup_during_outer_unwind_does_not_abort",
    },
    IntentionalLeakTest {
        target: "panic_across_ffi_is_caught",
        filter: "outer_unwind_subprocess::callback_panics_during_outer_unwind_do_not_abort",
    },
    IntentionalLeakTest {
        target: "replay",
        filter: "replay_mixer_drop_panics_run_all_cleanup_before_resuming",
    },
    IntentionalLeakTest {
        target: "user_data",
        filter: "outer_unwind_subprocess::user_data_destructors_during_outer_unwind_do_not_abort",
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MiriTest {
    name: &'static str,
    allows_intentional_leaks: bool,
}

const MIRI_TESTS: &[MiriTest] = &[
    MiriTest {
        name: "core::ffi_vec::tests::exact_and_excess_capacity_expose_only_initialized_values",
        allows_intentional_leaks: false,
    },
    MiriTest {
        name: "core::ffi_vec::tests::excessive_native_count_is_rejected_without_committing_length",
        allows_intentional_leaks: false,
    },
    MiriTest {
        name: "core::ffi_vec::tests::fill_panic_leaves_the_visible_length_at_zero",
        allows_intentional_leaks: false,
    },
    MiriTest {
        name: "core::ffi_vec::tests::grows_from_existing_capacity_to_the_full_request",
        allows_intentional_leaks: false,
    },
    MiriTest {
        name: "core::ffi_vec::tests::mapped_read_discards_partial_output_on_error",
        allows_intentional_leaks: false,
    },
    MiriTest {
        name: "core::ffi_vec::tests::mapped_read_supports_a_different_safe_layout",
        allows_intentional_leaks: false,
    },
    MiriTest {
        name: "core::ffi_vec::tests::negative_native_count_is_rejected_without_committing_length",
        allows_intentional_leaks: false,
    },
    MiriTest {
        name: "core::ffi_vec::tests::negative_request_is_rejected_after_clearing",
        allows_intentional_leaks: false,
    },
    MiriTest {
        name: "core::ffi_vec::tests::repeated_fill_reuses_the_allocation",
        allows_intentional_leaks: false,
    },
    MiriTest {
        name: "core::ffi_vec::tests::zero_request_clears_without_calling_native_fill",
        allows_intentional_leaks: false,
    },
    MiriTest {
        name: "core::callback_state::tests::owner_cleanup_runs_only_after_the_callback_returns",
        allows_intentional_leaks: false,
    },
    MiriTest {
        name: "core::callback_state::tests::callback_without_owner_frame_retains_cleanup_without_running_or_dropping_it",
        allows_intentional_leaks: true,
    },
    MiriTest {
        name: "core::callback_state::tests::concurrent_worker_panics_drop_every_payload_only_on_the_owner_boundary",
        allows_intentional_leaks: true,
    },
    MiriTest {
        name: "core::foundation::tests::activity_counter_exhaustion_does_not_wrap",
        allows_intentional_leaks: true,
    },
    MiriTest {
        name: "events::tests::empty_null_slice_is_valid_and_broken_pairs_are_rejected",
        allows_intentional_leaks: false,
    },
    MiriTest {
        name: "recording::tests::recording_limits_check_native_writer_boundaries",
        allows_intentional_leaks: false,
    },
    MiriTest {
        name: "snapshot::tests::native_payload_length_is_bounded_before_ffi_conversion",
        allows_intentional_leaks: false,
    },
    MiriTest {
        name: "snapshot::tests::native_payload_growth_between_query_and_fill_is_rejected",
        allows_intentional_leaks: false,
    },
    MiriTest {
        name: "core::identity_registry::tests::dropped_restore_plan_leaves_active_state_but_consumes_nonces",
        allows_intentional_leaks: false,
    },
    MiriTest {
        name: "core::identity_registry::tests::restore_preserves_only_the_exact_registration_intersection",
        allows_intentional_leaks: false,
    },
    MiriTest {
        name: "replay::preflight::lifecycle::tests::pool_reuse_invalidates_the_old_generation_and_preserves_lifo_order",
        allows_intentional_leaks: false,
    },
    MiriTest {
        name: "replay::preflight::tests::explosion_preflight_rejects_double_positions_outside_native_query_bounds",
        allows_intentional_leaks: false,
    },
];

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
                .env(
                    "BOXDD_SYS_PROVIDER",
                    ProviderCapability::WasmCompileOnly.as_str(),
                )
                .args(["check", "--locked", "-p", "boxdd-sys", "--target", target]);
            scope_wasi_sysroot(&mut command, target);
            if let Some(features) = features {
                command.args(["--features", features]);
            }
            run_command(&mut command, &format!("WASM compile contract for {target}"))?;
        }

        for features in [None, Some("double-precision")] {
            let mut command = cargo_command(&toolchain);
            command
                .current_dir(root)
                .env(
                    "BOXDD_SYS_PROVIDER",
                    ProviderCapability::WasmCompileOnly.as_str(),
                )
                .args([
                    "check", "--locked", "-p", "boxdd", "--lib", "--target", target,
                ]);
            scope_wasi_sysroot(&mut command, target);
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

            let mut recording_callback_free = wasm_probe_command(
                root,
                &toolchain,
                target,
                features,
                WASM_RECORDING_CALLBACK_FREE_QUERY_PROBE,
            );
            run_command(
                &mut recording_callback_free,
                &format!("callback-free recording Safe Rust WASM query contract for {target}"),
            )?;

            for probe in WASM_CALLBACK_BOUNDARY_PROBES {
                verify_wasm_callback_boundary(root, &toolchain, target, features, *probe)?;
            }
        }

        if target == WASM_TARGET {
            for features in [None, Some("double-precision")] {
                let mut provider_route = wasm_provider_compile_command(root, &toolchain, features);
                run_command(
                    &mut provider_route,
                    "WASM provider-routed Rust compile contract",
                )?;
            }

            let mut bevy_testbed = wasm_bevy_testbed_command(root, &toolchain);
            run_command(
                &mut bevy_testbed,
                "deployed Bevy Pages testbed WASM compile contract",
            )?;
        }
    }
    Ok(())
}

fn wasm_provider_compile_command(root: &Path, toolchain: &str, features: Option<&str>) -> Command {
    let mut command = cargo_command(toolchain);
    command
        .current_dir(root)
        .env(
            "BOXDD_SYS_PROVIDER",
            ProviderCapability::WasmProvider.as_str(),
        )
        .args([
            "rustc",
            "--locked",
            "-p",
            "boxdd-provider-smoke",
            "--lib",
            "--target",
            WASM_TARGET,
        ]);
    scope_wasi_sysroot(&mut command, WASM_TARGET);
    if let Some(features) = features {
        command.args(["--features", features]);
    }
    add_wasm_app_link_args(&mut command, &[]);
    command
}

fn wasm_bevy_testbed_command(root: &Path, toolchain: &str) -> Command {
    let mut command = cargo_command(toolchain);
    command
        .current_dir(root)
        .env(
            "BOXDD_SYS_PROVIDER",
            ProviderCapability::WasmCompileOnly.as_str(),
        )
        .args([
            "check",
            "--locked",
            "-p",
            "bevy_boxdd",
            "--example",
            BEVY_WEB_EXAMPLE,
            "--target",
            WASM_TARGET,
        ]);
    scope_wasi_sysroot(&mut command, WASM_TARGET);
    command
}

fn verify_wasm_callback_boundary(
    root: &Path,
    toolchain: &str,
    target: &str,
    features: Option<&str>,
    probe: WasmCallbackBoundaryProbe,
) -> Result<()> {
    let mut command = wasm_probe_command(root, toolchain, target, features, probe.binary);
    let output = run_output(&mut command, &format!("cargo check {}", probe.binary))
        .map_err(Error::message)?;
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
        .env(
            "BOXDD_SYS_PROVIDER",
            ProviderCapability::WasmCompileOnly.as_str(),
        )
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
    scope_wasi_sysroot(&mut command, target);
    if let Some(features) = features {
        command.args(["--features", features]);
    }
    command
}

fn scope_wasi_sysroot(command: &mut Command, target: &str) {
    if target != "wasm32-wasip1" {
        command.env_remove("BOXDD_SYS_WASI_SYSROOT");
    }
}

fn verify_wasm_runtime(root: &Path) -> Result<()> {
    for precision in [ProviderPrecision::Single, ProviderPrecision::Double] {
        // The provider command validates the pinned Emscripten and wasm-bindgen identities before
        // compiling, so a missing or stale SDK is an explicit qualification failure.
        let (session, sdk) = provider::provider_smoke_for_precision(root, precision)?;
        let mut browser = browser_provider_smoke_command(root, precision, &session, &sdk)?;
        run_command(
            &mut browser,
            &format!(
                "Chromium provider shared-memory smoke ({})",
                precision.as_str()
            ),
        )?;
        drop(session);
    }
    Ok(())
}

fn browser_provider_smoke_command(
    root: &Path,
    precision: ProviderPrecision,
    session: &provider::ProviderSmokeSession,
    sdk: &crate::emscripten_sdk::EmscriptenTools,
) -> Result<Command> {
    let command = sdk.npm_command().map_err(Error::message)?;
    Ok(configure_browser_provider_smoke_command(
        command,
        root,
        session.target_dir(),
        precision,
    ))
}

pub(crate) fn run_existing_provider_browser_smoke(
    command: Command,
    root: &Path,
    session: &provider::ProviderSmokeSession,
    precision: ProviderPrecision,
) -> Result<()> {
    let mut command =
        configure_browser_provider_smoke_command(command, root, session.target_dir(), precision);
    run_command(
        &mut command,
        &format!(
            "Chromium authenticated provider shared-memory smoke ({})",
            precision.as_str()
        ),
    )
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

    for test in MIRI_TESTS {
        let mut command = miri_test_command(root, *test);
        let label = format!("Miri exact pure-Rust test {}", test.name);
        let output = run_output(&mut command, &label).map_err(Error::message)?;
        require_success(&output, &label)?;
        require_exact_miri_test_result(test.name, &output)?;
        println!("passed {label}");
    }
    Ok(())
}

fn miri_test_command(root: &Path, test: MiriTest) -> Command {
    let mut command = cargo_command(VERIFICATION_NIGHTLY);
    command
        .current_dir(root)
        .env(
            "MIRIFLAGS",
            if test.allows_intentional_leaks {
                MIRI_FLAGS_ALLOW_INTENTIONAL_LEAKS
            } else {
                MIRI_FLAGS
            },
        )
        .args([
            "miri", "test", "--locked", "-p", "boxdd", "--lib", "--color", "never", test.name,
            "--", "--exact",
        ]);
    command
}

fn require_exact_miri_test_result(test: &str, output: &Output) -> Result<()> {
    let report = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    require_exact_miri_test_report(test, &report)
}

fn require_exact_miri_test_report(test: &str, report: &str) -> Result<()> {
    let results = report
        .lines()
        .filter_map(parse_libtest_result)
        .collect::<Vec<_>>();
    match results.as_slice() {
        [result]
            if result.passed == 1
                && result.failed == 0
                && result.ignored == 0
                && result.measured == 0 =>
        {
            Ok(())
        }
        [] => Err(Error::message(format!(
            "Miri exact test `{test}` completed without a parseable libtest result; refusing false-green evidence"
        ))),
        _ => Err(Error::message(format!(
            "Miri exact test `{test}` must execute exactly one passing test; observed {results:?}"
        ))),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LibtestResult {
    passed: usize,
    failed: usize,
    ignored: usize,
    measured: usize,
}

fn parse_libtest_result(line: &str) -> Option<LibtestResult> {
    let fields = line
        .trim()
        .strip_prefix("test result: ok. ")?
        .split(';')
        .map(str::trim)
        .collect::<Vec<_>>();
    Some(LibtestResult {
        passed: parse_libtest_count(&fields, "passed")?,
        failed: parse_libtest_count(&fields, "failed")?,
        ignored: parse_libtest_count(&fields, "ignored")?,
        measured: parse_libtest_count(&fields, "measured")?,
    })
}

fn parse_libtest_count(fields: &[&str], label: &str) -> Option<usize> {
    fields.iter().find_map(|field| {
        field
            .strip_suffix(label)
            .map(str::trim)
            .and_then(|count| count.parse().ok())
    })
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
                "--test",
                "panic_across_ffi_is_caught",
                "--test",
                "material_mix_callbacks",
            ],
            Self::Thread => &[
                "--test",
                "foundation_world_activity",
                "--test",
                "material_mix_callbacks",
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
    require_sanitizer_qualification_host(&host)?;
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
    let mut command = Command::new("rustup");
    command.args(["run", toolchain, "rustc", "--version"]);
    let output = run_output(&mut command, "rustup toolchain probe").map_err(Error::message)?;
    require_success(&output, &format!("required Rust toolchain {toolchain}"))
}

fn require_rust_target(toolchain: &str, target: &str) -> Result<()> {
    let mut command = Command::new("rustup");
    command.args(["target", "list", "--installed", "--toolchain", toolchain]);
    let output = run_output(&mut command, "rustup target list").map_err(Error::message)?;
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
    let mut command = Command::new("rustup");
    command.args(["component", "list", "--toolchain", toolchain]);
    let output = run_output(&mut command, "rustup component list").map_err(Error::message)?;
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
    let mut command = Command::new("rustup");
    command.args(["run", toolchain, "rustc", "-vV"]);
    let output = run_output(&mut command, "rustc -vV").map_err(Error::message)?;
    require_success(&output, "rustc -vV")?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .map(ToOwned::to_owned)
        .ok_or_else(|| Error::message("rustc -vV did not report a host target"))
}

fn require_sanitizer_qualification_host(host: &str) -> Result<()> {
    if host.split('-').any(|component| component == "linux") {
        Ok(())
    } else {
        Err(Error::message(format!(
            "mixed C/Rust sanitizer qualification requires a Linux host; {VERIFICATION_NIGHTLY} reports {host}. Run the protected Linux CI gate instead"
        )))
    }
}

fn verify_c_sanitizer(sanitizer: Sanitizer) -> Result<()> {
    let compiler = env::var("CC").unwrap_or_else(|_| "cc".to_owned());
    let mut command = Command::new(&compiler);
    command
        .args(sanitizer.c_flag().split_whitespace())
        .args(["-x", "c", "-fsyntax-only", "-"]);
    let output = run_output_with_input(
        &mut command,
        b"int main(void) { return 0; }\n",
        &format!("{compiler} sanitizer probe"),
    )
    .map_err(Error::message)?;
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
    use std::ffi::OsStr;

    use super::*;

    #[test]
    fn sanitizers_instrument_c_and_only_use_supported_rust_modes() {
        assert_eq!(Sanitizer::Address.rust_flag(), Some("address"));
        assert_eq!(Sanitizer::Undefined.rust_flag(), None);
        assert_eq!(Sanitizer::Thread.rust_flag(), Some("thread"));
        for sanitizer in [Sanitizer::Address, Sanitizer::Undefined, Sanitizer::Thread] {
            assert!(sanitizer.c_flag().contains("-fsanitize="));
            assert!(!sanitizer.tests().is_empty());
        }
        for required in [
            "buffer_reuse",
            "events_and_sensors",
            "panic_across_ffi_is_caught",
            "material_mix_callbacks",
        ] {
            assert!(Sanitizer::Address.tests().contains(&required));
            assert!(Sanitizer::Undefined.tests().contains(&required));
        }
        assert!(
            Sanitizer::Thread
                .tests()
                .contains(&"material_mix_callbacks")
        );
    }

    #[test]
    fn sanitizer_qualification_rejects_non_linux_hosts_before_compilation() {
        for host in [
            "x86_64-unknown-linux-gnu",
            "aarch64-unknown-linux-musl",
            "riscv64gc-unknown-linux-gnu",
        ] {
            assert!(require_sanitizer_qualification_host(host).is_ok());
        }

        for host in ["aarch64-apple-darwin", "x86_64-pc-windows-msvc"] {
            let error = require_sanitizer_qualification_host(host).unwrap_err();
            assert!(error.to_string().contains("requires a Linux host"));
            assert!(error.to_string().contains(host));
        }
    }

    #[test]
    fn address_sanitizer_leak_allowlist_is_exact_and_narrow() {
        let expected = [
            IntentionalLeakTest {
                target: "material_mix_callbacks",
                filter: "outer_unwind_subprocess::material_mix_cleanup_during_outer_unwind_does_not_abort",
            },
            IntentionalLeakTest {
                target: "panic_across_ffi_is_caught",
                filter: "outer_unwind_subprocess::callback_panics_during_outer_unwind_do_not_abort",
            },
            IntentionalLeakTest {
                target: "replay",
                filter: "replay_mixer_drop_panics_run_all_cleanup_before_resuming",
            },
            IntentionalLeakTest {
                target: "user_data",
                filter: "outer_unwind_subprocess::user_data_destructors_during_outer_unwind_do_not_abort",
            },
        ];
        assert_eq!(ASAN_INTENTIONAL_LEAK_TESTS.len(), expected.len());
        for (actual, expected) in ASAN_INTENTIONAL_LEAK_TESTS.iter().zip(expected) {
            assert_eq!(actual.target, expected.target);
            assert_eq!(actual.filter, expected.filter);
        }
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

        let leak_tolerant = MIRI_TESTS
            .iter()
            .filter(|test| test.allows_intentional_leaks)
            .map(|test| test.name)
            .collect::<Vec<_>>();
        assert_eq!(
            leak_tolerant,
            [
                "core::callback_state::tests::callback_without_owner_frame_retains_cleanup_without_running_or_dropping_it",
                "core::callback_state::tests::concurrent_worker_panics_drop_every_payload_only_on_the_owner_boundary",
                "core::foundation::tests::activity_counter_exhaustion_does_not_wrap",
            ]
        );
        assert!(MIRI_TESTS.iter().any(|test| {
            test.name
                == "core::callback_state::tests::owner_cleanup_runs_only_after_the_callback_returns"
                && !test.allows_intentional_leaks
        }));
        for required in ["recording::tests", "snapshot::tests", "replay::preflight"] {
            assert!(
                MIRI_TESTS
                    .iter()
                    .any(|test| { test.name.contains(required) && !test.allows_intentional_leaks })
            );
        }

        let names = MIRI_TESTS
            .iter()
            .map(|test| test.name)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            names.len(),
            MIRI_TESTS.len(),
            "Miri test names must be unique"
        );
        assert!(names.iter().all(|name| name.contains("::tests::")));

        for test in MIRI_TESTS {
            let command = miri_test_command(Path::new("/workspace/boxdd"), *test);
            let args = command.get_args().collect::<Vec<_>>();
            assert!(args.ends_with(&[
                OsStr::new(test.name),
                OsStr::new("--"),
                OsStr::new("--exact"),
            ]));
        }
    }

    #[test]
    fn miri_result_gate_rejects_zero_or_ambiguous_matches() {
        let exact = "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 412 filtered out; finished in 0.01s";
        assert!(require_exact_miri_test_report("fixture", exact).is_ok());

        for false_green in [
            "test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 413 filtered out; finished in 0.00s",
            "test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 411 filtered out; finished in 0.01s",
            "Finished test target(s) in 0.01s",
            "test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 411 filtered out; finished in 0.01s",
        ] {
            assert!(
                require_exact_miri_test_report("fixture", false_green).is_err(),
                "false-green Miri report must fail: {false_green}"
            );
        }

        let duplicate = format!("{exact}\n{exact}");
        assert!(require_exact_miri_test_report("fixture", &duplicate).is_err());
    }

    #[test]
    fn verification_commands_fail_closed_on_unknown_arguments() {
        assert!(require_check_args("gate", &["--skip".to_owned()]).is_err());
        assert!(Sanitizer::parse(&["--memory".to_owned()]).is_err());
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
            "wasm-debug-draw-boundary",
            "wasm-debug-draw-method-boundary",
            "wasm-query-callback-boundary",
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
        assert!(command.get_envs().any(|(key, value)| {
            key == OsStr::new("BOXDD_SYS_WASI_SYSROOT") && value.is_none()
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
        assert!(
            !positive
                .get_envs()
                .any(|(key, _)| key == OsStr::new("BOXDD_SYS_WASI_SYSROOT"))
        );

        let recording_positive = wasm_probe_command(
            root,
            "1.95.0",
            "wasm32-wasip1",
            None,
            WASM_RECORDING_CALLBACK_FREE_QUERY_PROBE,
        );
        let recording_positive_arguments = recording_positive
            .get_args()
            .map(|argument| argument.to_str().expect("literal cargo argument"))
            .collect::<Vec<_>>();
        assert!(
            recording_positive_arguments
                .windows(2)
                .any(|pair| { pair == ["--bin", WASM_RECORDING_CALLBACK_FREE_QUERY_PROBE] })
        );
    }

    #[test]
    fn wasm_compile_gate_checks_the_deployed_bevy_pages_example() {
        let root = Path::new("/workspace/boxdd");
        let command = wasm_bevy_testbed_command(root, "1.97.1");
        assert_eq!(command.get_current_dir(), Some(root));
        assert_eq!(
            command
                .get_args()
                .map(|argument| argument.to_str().expect("literal cargo argument"))
                .collect::<Vec<_>>(),
            [
                "+1.97.1",
                "check",
                "--locked",
                "-p",
                "bevy_boxdd",
                "--example",
                BEVY_WEB_EXAMPLE,
                "--target",
                WASM_TARGET,
            ]
        );
        assert!(command.get_envs().any(|(key, value)| {
            key == OsStr::new("BOXDD_SYS_PROVIDER")
                && value == Some(OsStr::new("wasm-compile-only"))
        }));
    }

    #[test]
    fn wasm_compile_gate_checks_the_provider_routed_consumer_at_msrv() {
        let root = Path::new("/workspace/boxdd");
        let command = wasm_provider_compile_command(root, "1.95.0", Some("double-precision"));
        assert_eq!(command.get_current_dir(), Some(root));
        let arguments = command
            .get_args()
            .map(|argument| argument.to_str().expect("literal cargo argument"))
            .collect::<Vec<_>>();
        assert_eq!(
            &arguments[..10],
            [
                "+1.95.0",
                "rustc",
                "--locked",
                "-p",
                "boxdd-provider-smoke",
                "--lib",
                "--target",
                WASM_TARGET,
                "--features",
                "double-precision",
            ]
        );
        assert!(arguments.contains(&"--"));
        assert!(arguments.contains(&"link-arg=--import-memory"));
        assert!(command.get_envs().any(|(key, value)| {
            key == OsStr::new("BOXDD_SYS_PROVIDER")
                && value == Some(OsStr::new(ProviderCapability::WasmProvider.as_str()))
        }));
        assert!(command.get_envs().any(|(key, value)| {
            key == OsStr::new("BOXDD_SYS_WASM_PROVIDER_FINAL_LINK")
                && value == Some(OsStr::new("boxdd-xtask-v1"))
        }));
    }
}
