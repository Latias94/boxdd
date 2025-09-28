#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

pub mod ffi {
    #[cfg(has_pregenerated)]
    include!("bindings_pregenerated.rs");
    #[cfg(not(has_pregenerated))]
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}
