use std::{
    ffi::OsStr,
    fs,
    path::Path,
    process::{Command, Output},
};

use crate::{Error, Result};

const EXTERNAL_OVERRIDE_DIAGNOSTIC: &str = "external-precision-override.txt";
const MIXED_DEPENDENCY_DIAGNOSTIC: &str = "mixed-dependency.txt";
const CARGO_CHECK_ARGS: &[&str] = &[
    "check",
    "--quiet",
    "--locked",
    "--color",
    "never",
    "--manifest-path",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExpectedOutcome {
    Success,
    Failure { diagnostic: &'static str },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct VerificationCase {
    label: &'static str,
    feature: &'static str,
    cflags: Option<&'static str>,
    expected: ExpectedOutcome,
}

const VERIFICATION_CASES: &[VerificationCase] = &[
    VerificationCase {
        label: "single",
        feature: "single",
        cflags: None,
        expected: ExpectedOutcome::Success,
    },
    VerificationCase {
        label: "double",
        feature: "double",
        cflags: None,
        expected: ExpectedOutcome::Success,
    },
    VerificationCase {
        label: "mixed-dependency",
        feature: "mixed-dependency",
        cflags: None,
        expected: ExpectedOutcome::Failure {
            diagnostic: MIXED_DEPENDENCY_DIAGNOSTIC,
        },
    },
    VerificationCase {
        label: "c-double-rust-single",
        feature: "single",
        cflags: Some("-DBOX2D_DOUBLE_PRECISION=1"),
        expected: ExpectedOutcome::Failure {
            diagnostic: EXTERNAL_OVERRIDE_DIAGNOSTIC,
        },
    },
    VerificationCase {
        label: "c-single-rust-double",
        feature: "double",
        cflags: Some("-UBOX2D_DOUBLE_PRECISION"),
        expected: ExpectedOutcome::Failure {
            diagnostic: EXTERNAL_OVERRIDE_DIAGNOSTIC,
        },
    },
];

pub fn run(root: &Path, args: &[String]) -> Result<()> {
    if !args.is_empty() {
        return Err(Error::message(
            "verify-precision-contract does not accept arguments",
        ));
    }

    let fixture = root.join("tools/precision-contract");
    let manifest = fixture.join("Cargo.toml");
    if !manifest.is_file() {
        return Err(Error::message(format!(
            "precision contract fixture not found: {}",
            manifest.display()
        )));
    }

    for case in VERIFICATION_CASES {
        verify_case(root, &fixture, &manifest, *case)?;
        println!("precision contract {}: ok", case.label);
    }

    Ok(())
}

fn verify_case(root: &Path, fixture: &Path, manifest: &Path, case: VerificationCase) -> Result<()> {
    let output = run_cargo_check(root, manifest, case)?;
    match case.expected {
        ExpectedOutcome::Success if output.status.success() => Ok(()),
        ExpectedOutcome::Success => Err(command_failure(case, &output, "succeed")),
        ExpectedOutcome::Failure { diagnostic } if output.status.success() => {
            Err(Error::message(format!(
                "precision contract `{}` unexpectedly compiled; expected diagnostic `{diagnostic}`",
                case.label
            )))
        }
        ExpectedOutcome::Failure { diagnostic } => {
            let expected = read_expected_diagnostic(fixture, diagnostic)?;
            let actual = command_output(&output);
            if actual.contains(&expected) {
                Ok(())
            } else {
                Err(Error::message(format!(
                    "precision contract `{}` failed without expected diagnostic `{expected}`\n{}",
                    case.label,
                    indent_output(&actual)
                )))
            }
        }
    }
}

fn run_cargo_check(root: &Path, manifest: &Path, case: VerificationCase) -> Result<Output> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsStr::new("cargo").to_os_string());
    let target_dir = root
        .join("target")
        .join("precision-contract")
        .join(case.label);
    let mut command = Command::new(cargo);
    command
        .args(CARGO_CHECK_ARGS)
        .arg(manifest)
        .args(["--no-default-features", "--features", case.feature])
        .arg("--target-dir")
        .arg(target_dir)
        .current_dir(root)
        .env("CARGO_TERM_COLOR", "never")
        .env("RUST_BACKTRACE", "0")
        .env_remove("CFLAGS")
        .env_remove("CPPFLAGS")
        .env_remove("CL")
        .env_remove("BINDGEN_EXTRA_CLANG_ARGS");
    if let Some(cflags) = case.cflags {
        command.env("CFLAGS", cflags);
    }

    command.output().map_err(|source| {
        Error::io(
            format!("precision contract cargo check ({})", case.label),
            source,
        )
    })
}

fn read_expected_diagnostic(fixture: &Path, file_name: &str) -> Result<String> {
    let path = fixture.join("expected").join(file_name);
    let diagnostic = fs::read_to_string(&path).map_err(|source| Error::io(&path, source))?;
    let diagnostic = diagnostic.trim();
    if diagnostic.is_empty() {
        return Err(Error::message(format!(
            "expected precision diagnostic is empty: {}",
            path.display()
        )));
    }
    Ok(diagnostic.to_owned())
}

fn command_failure(case: VerificationCase, output: &Output, expectation: &str) -> Error {
    let actual = command_output(output);
    Error::message(format!(
        "precision contract `{}` was expected to {expectation}\n{}",
        case.label,
        indent_output(&actual)
    ))
}

fn command_output(output: &Output) -> String {
    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&output.stderr));
    combined
}

fn indent_output(output: &str) -> String {
    output
        .lines()
        .map(|line| format!("  {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn matrix_covers_matching_and_mismatched_precision_routes() {
        assert_eq!(VERIFICATION_CASES.len(), 5);
        assert!(
            VERIFICATION_CASES.iter().any(|case| {
                case.feature == "single" && case.expected == ExpectedOutcome::Success
            })
        );
        assert!(
            VERIFICATION_CASES.iter().any(|case| {
                case.feature == "double" && case.expected == ExpectedOutcome::Success
            })
        );
        assert!(VERIFICATION_CASES.iter().any(|case| {
            case.feature == "mixed-dependency"
                && matches!(case.expected, ExpectedOutcome::Failure { .. })
        }));
        assert!(VERIFICATION_CASES.iter().any(|case| {
            case.feature == "single" && case.cflags == Some("-DBOX2D_DOUBLE_PRECISION=1")
        }));
        assert!(VERIFICATION_CASES.iter().any(|case| {
            case.feature == "double" && case.cflags == Some("-UBOX2D_DOUBLE_PRECISION")
        }));
    }

    #[test]
    fn cargo_checks_use_the_fixture_lockfile() {
        assert!(CARGO_CHECK_ARGS.contains(&"--locked"));
    }

    #[test]
    fn nested_fixture_stays_out_of_the_root_workspace() {
        let fixture_manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tools/precision-contract/Cargo.toml");
        let manifest = fs::read_to_string(&fixture_manifest).expect("fixture manifest");
        assert!(manifest.contains("publish = false"));
        assert!(manifest.contains("[workspace]"));
    }
}
