pub mod api_coverage;
mod api_recording;
pub mod build_policy_sources;
pub mod native_provider;
pub mod package_registry;
pub mod pages;
pub mod precision_contract;
pub mod provider;
pub mod recording_codegen;
pub mod release_contract;
pub mod sample_parity;
pub mod upstream_sync;
pub mod verification;
pub mod wasm_release;

mod support;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdateMode {
    Check,
    Write,
}

pub fn parse_update_mode(command: &str, args: &[String]) -> crate::Result<UpdateMode> {
    match args {
        [] => Ok(UpdateMode::Check),
        [arg] if arg == "--check" => Ok(UpdateMode::Check),
        [arg] if arg == "--write" => Ok(UpdateMode::Write),
        _ => Err(crate::Error::message(format!(
            "{command} expects --check or --write"
        ))),
    }
}

pub(crate) fn set_once<T>(slot: &mut Option<T>, value: T, flag: &str) -> crate::Result<()> {
    if slot.replace(value).is_some() {
        Err(crate::Error::message(format!(
            "{flag} may only be supplied once"
        )))
    } else {
        Ok(())
    }
}
