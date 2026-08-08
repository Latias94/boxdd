//! Closed memory contract for the browser WASM provider route.

#![allow(dead_code)] // Shared verbatim by the library, build script, and xtask policy contexts.

use std::ffi::OsStr;

pub(crate) const WASM_PAGE_BYTES: u64 = 64 * 1024;
pub(crate) const PROVIDER_STATIC_BASE_BYTES: u64 = 1024;
pub(crate) const PROVIDER_HEAP_LIMIT_BYTES: u64 = 64 * 1024 * 1024;
pub(crate) const CONSUMER_GLOBAL_BASE_BYTES: u64 = PROVIDER_HEAP_LIMIT_BYTES;
pub(crate) const INITIAL_MEMORY_BYTES: u64 = 128 * 1024 * 1024;
pub(crate) const MAXIMUM_MEMORY_BYTES: u64 = 512 * 1024 * 1024;

pub(crate) const MEMORY_MODEL: &str = "shared-partitioned-heaps-v2";

pub(crate) const FINAL_LINK_OPT_IN_ENV: &str = "BOXDD_SYS_WASM_PROVIDER_FINAL_LINK";
pub(crate) const FINAL_LINK_OPT_IN_VALUE: &str = "boxdd-xtask-v1";

pub(crate) const INITIAL_MEMORY_PAGES: u64 = INITIAL_MEMORY_BYTES / WASM_PAGE_BYTES;
pub(crate) const MAXIMUM_MEMORY_PAGES: u64 = MAXIMUM_MEMORY_BYTES / WASM_PAGE_BYTES;

pub(crate) fn validate_final_link_opt_in(
    provider_selected: bool,
    value: Option<&OsStr>,
) -> Result<(), &'static str> {
    match (provider_selected, value) {
        (true, Some(value)) if value == OsStr::new(FINAL_LINK_OPT_IN_VALUE) => Ok(()),
        (true, _) => Err(
            "wasm-provider requires a controlled final-link build; use the repository xtask provider or Pages entry point instead of setting BOXDD_SYS_PROVIDER directly",
        ),
        (false, None) => Ok(()),
        (false, Some(_)) => Err(
            "the wasm-provider final-link opt-in is valid only when BOXDD_SYS_PROVIDER=wasm-provider",
        ),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SharedMemoryLayout {
    pub(crate) provider_static_base_bytes: u64,
    pub(crate) provider_heap_limit_bytes: u64,
    pub(crate) consumer_global_base_bytes: u64,
    pub(crate) initial_memory_bytes: u64,
    pub(crate) maximum_memory_bytes: u64,
}

impl SharedMemoryLayout {
    pub(crate) const CLOSED: Self = Self {
        provider_static_base_bytes: PROVIDER_STATIC_BASE_BYTES,
        provider_heap_limit_bytes: PROVIDER_HEAP_LIMIT_BYTES,
        consumer_global_base_bytes: CONSUMER_GLOBAL_BASE_BYTES,
        initial_memory_bytes: INITIAL_MEMORY_BYTES,
        maximum_memory_bytes: MAXIMUM_MEMORY_BYTES,
    };

    pub(crate) fn validate(self) -> Result<(), &'static str> {
        if self.provider_static_base_bytes < 1024
            || !self.provider_static_base_bytes.is_multiple_of(16)
        {
            return Err("the provider static base must be at least 1024 and 16-byte aligned");
        }
        if [
            self.provider_heap_limit_bytes,
            self.consumer_global_base_bytes,
            self.initial_memory_bytes,
            self.maximum_memory_bytes,
        ]
        .into_iter()
        .any(|value| value == 0 || !value.is_multiple_of(WASM_PAGE_BYTES))
        {
            return Err("WASM heap boundaries must be non-zero and WebAssembly-page aligned");
        }
        if self.provider_static_base_bytes >= self.provider_heap_limit_bytes {
            return Err("the provider partition must leave room for static data, stack, and heap");
        }
        if self.provider_heap_limit_bytes != self.consumer_global_base_bytes {
            return Err("the provider heap limit must equal the Rust global base");
        }
        if self.initial_memory_bytes <= self.consumer_global_base_bytes {
            return Err("initial memory must leave a non-empty Rust heap partition");
        }
        if self.maximum_memory_bytes <= self.initial_memory_bytes {
            return Err("maximum memory must leave Rust heap growth headroom");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_layout_is_valid() {
        SharedMemoryLayout::CLOSED.validate().unwrap();
        assert_eq!(INITIAL_MEMORY_PAGES, 2048);
        assert_eq!(MAXIMUM_MEMORY_PAGES, 8192);
    }

    #[test]
    fn every_layout_coordinate_is_fail_closed() {
        let closed = SharedMemoryLayout::CLOSED;
        for mutated in [
            SharedMemoryLayout {
                provider_static_base_bytes: 1000,
                ..closed
            },
            SharedMemoryLayout {
                provider_heap_limit_bytes: closed.provider_heap_limit_bytes + WASM_PAGE_BYTES,
                ..closed
            },
            SharedMemoryLayout {
                consumer_global_base_bytes: closed.consumer_global_base_bytes + WASM_PAGE_BYTES,
                ..closed
            },
            SharedMemoryLayout {
                initial_memory_bytes: closed.consumer_global_base_bytes,
                ..closed
            },
            SharedMemoryLayout {
                maximum_memory_bytes: closed.initial_memory_bytes,
                ..closed
            },
            SharedMemoryLayout {
                initial_memory_bytes: closed.initial_memory_bytes + 1,
                ..closed
            },
        ] {
            assert_ne!(mutated, closed);
            assert!(
                mutated.validate().is_err(),
                "accepted mutation: {mutated:?}"
            );
        }
    }

    #[test]
    fn provider_final_link_opt_in_is_exact_and_scoped() {
        assert!(
            validate_final_link_opt_in(true, Some(OsStr::new(FINAL_LINK_OPT_IN_VALUE)),).is_ok()
        );
        assert!(validate_final_link_opt_in(true, None).is_err());
        assert!(validate_final_link_opt_in(true, Some(OsStr::new("1"))).is_err());
        assert!(validate_final_link_opt_in(false, None).is_ok());
        assert!(
            validate_final_link_opt_in(false, Some(OsStr::new(FINAL_LINK_OPT_IN_VALUE)),).is_err()
        );
    }

    #[cfg(unix)]
    #[test]
    fn provider_final_link_opt_in_rejects_non_unicode_values() {
        use std::os::unix::ffi::OsStrExt;

        let non_unicode = OsStr::from_bytes(b"boxdd-xtask-v1\xff");
        assert!(validate_final_link_opt_in(true, Some(non_unicode)).is_err());
        assert!(validate_final_link_opt_in(false, Some(non_unicode)).is_err());
    }
}
