//! Provider-selection vocabulary shared by the build script and repository tooling.
//!
//! Release matrices and artifact target sets belong to their concrete build and release commands;
//! this module only centralizes names and classifications that have production consumers.

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum ProviderCapability {
    Vendored,
    System,
    Prebuilt,
    WasmCompileOnly,
    WasmProvider,
}

impl ProviderCapability {
    pub(crate) const ALL: [Self; 5] = [
        Self::Vendored,
        Self::System,
        Self::Prebuilt,
        Self::WasmCompileOnly,
        Self::WasmProvider,
    ];

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Vendored => "vendored",
            Self::System => "system",
            Self::Prebuilt => "prebuilt",
            Self::WasmCompileOnly => "wasm-compile-only",
            Self::WasmProvider => "wasm-provider",
        }
    }

    pub(crate) const fn is_wasm(self) -> bool {
        matches!(self, Self::WasmCompileOnly | Self::WasmProvider)
    }

    pub(crate) const fn supports_native_qualification(self) -> bool {
        matches!(self, Self::System | Self::Prebuilt)
    }

    pub(crate) fn parse_build_name(value: &str) -> Result<Self, String> {
        match value {
            "vendored" => Ok(Self::Vendored),
            "system" => Ok(Self::System),
            "prebuilt" => Ok(Self::Prebuilt),
            "wasm-compile-only" => Ok(Self::WasmCompileOnly),
            "wasm-provider" => Ok(Self::WasmProvider),
            _ => Err(format!(
                "unsupported provider {value:?}; expected vendored, system, prebuilt, wasm-compile-only, wasm-provider"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_names_round_trip_through_the_shared_vocabulary() {
        assert_eq!(
            ProviderCapability::ALL.map(ProviderCapability::as_str),
            [
                "vendored",
                "system",
                "prebuilt",
                "wasm-compile-only",
                "wasm-provider",
            ]
        );
        for provider in ProviderCapability::ALL {
            assert_eq!(
                ProviderCapability::parse_build_name(provider.as_str()),
                Ok(provider)
            );
        }
        assert!(ProviderCapability::parse_build_name("unknown").is_err());
    }

    #[test]
    fn provider_classifications_match_their_production_routes() {
        assert!(!ProviderCapability::Vendored.is_wasm());
        assert!(!ProviderCapability::System.is_wasm());
        assert!(!ProviderCapability::Prebuilt.is_wasm());
        assert!(ProviderCapability::WasmCompileOnly.is_wasm());
        assert!(ProviderCapability::WasmProvider.is_wasm());

        assert!(!ProviderCapability::Vendored.supports_native_qualification());
        assert!(ProviderCapability::System.supports_native_qualification());
        assert!(ProviderCapability::Prebuilt.supports_native_qualification());
        assert!(!ProviderCapability::WasmCompileOnly.supports_native_qualification());
        assert!(!ProviderCapability::WasmProvider.supports_native_qualification());
    }
}
