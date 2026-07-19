pub mod api_coverage;
mod api_recording;
pub mod pages;
pub mod precision_contract;
pub mod provider;
pub mod sample_parity;
pub mod upstream_sync;

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
