#![allow(
    clippy::all,
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals
)]

use boxdd_sys::ffi::*;

#[used]
static FORCE_NATIVE_LINK: fn() -> bool = boxdd_abi_probe::precision_matches;

include!(concat!(env!("OUT_DIR"), "/abi_ctest.rs"));
