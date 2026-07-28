//! Structural verification for native static-provider archives.
//!
//! Provider manifests bind files by digest. This module proves that the bound archive itself is a
//! normal static archive for the requested target and contains exactly the native adapter identity
//! expected by the caller. It deliberately has no manifest, network, extraction, or linker duties.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use object::{Object, ObjectSection, ObjectSymbol};
use sha2::{Digest, Sha256};

#[allow(dead_code)]
pub(crate) const BUILD_POLICY_SOURCE_SHA256: &str =
    "0dd67a0054767ec23748739fad710a02c14e1b87e2a583bfbfc7e6946f4bf05b";

const MAX_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_ARCHIVE_MEMBERS: usize = 16_384;
const MAX_ARCHIVE_SYMBOLS: usize = 262_144;
const MAX_IDENTITY_VALUES: usize = 4_096;
const EFFECTIVE_SOURCE_SYMBOL: &str = "boxddEffectiveSourceSha256";
const PRIVATE_COUNT_SYMBOL: &str = "boxddPrivateAbiValueCount";
const PRIVATE_VALUES_SYMBOL: &str = "boxddPrivateAbiValues";
const LAYOUT_COUNT_SYMBOL: &str = "boxddSnapshotLayoutValueCount";
const LAYOUT_VALUES_SYMBOL: &str = "boxddSnapshotLayoutValues";

#[derive(Clone, Copy, Debug)]
pub struct ArchiveExpectation<'a> {
    pub target: &'a str,
    pub required_symbols: &'a [&'a str],
    pub effective_source_sha256: &'a str,
    pub private_abi_hash: &'a str,
    pub snapshot_layout_hash: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedArchiveIdentity {
    pub effective_source_sha256: String,
    pub private_abi_hash: String,
    pub snapshot_layout_hash: u32,
    pub object_count: usize,
    pub archive_sha256: String,
    pub archive_bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ExpectedObject {
    format: object::BinaryFormat,
    architecture: object::Architecture,
}

impl ExpectedObject {
    fn for_target(target: &str) -> Result<Self, String> {
        let (format, architecture) = match target {
            "x86_64-unknown-linux-gnu" => (object::BinaryFormat::Elf, object::Architecture::X86_64),
            "x86_64-apple-darwin" => (object::BinaryFormat::MachO, object::Architecture::X86_64),
            "aarch64-apple-darwin" => (object::BinaryFormat::MachO, object::Architecture::Aarch64),
            "x86_64-pc-windows-msvc" => (object::BinaryFormat::Coff, object::Architecture::X86_64),
            _ => {
                return Err(format!(
                    "cannot structurally verify a provider archive for unsupported target {target:?}"
                ));
            }
        };
        Ok(Self {
            format,
            architecture,
        })
    }

    fn accepts(self, file: &object::File<'_>) -> bool {
        file.format() == self.format
            && file.architecture() == self.architecture
            && file.sub_architecture().is_none()
            && file.is_little_endian()
            && file.architecture().address_size() == Some(object::AddressSize::U64)
    }
}

fn require_target_platform(file: &object::File<'_>, target: &str) -> Result<(), String> {
    if !target.ends_with("-apple-darwin") {
        return Ok(());
    }
    let object::File::MachO64(file) = file else {
        return Err("Darwin provider member is not a 64-bit Mach-O object".to_owned());
    };
    let build_version = file
        .build_version()
        .map_err(|error| format!("failed to parse Mach-O LC_BUILD_VERSION: {error}"))?
        .ok_or_else(|| "Darwin provider member is missing LC_BUILD_VERSION".to_owned())?;
    let platform = build_version.platform.get(file.endian());
    if platform == object::macho::PLATFORM_MACOS {
        Ok(())
    } else {
        Err(format!(
            "Darwin provider member declares Mach-O platform {platform}, expected macOS"
        ))
    }
}

pub fn verify_provider_archive(
    archive_path: &Path,
    expected: &ArchiveExpectation<'_>,
) -> Result<VerifiedArchiveIdentity, String> {
    validate_expectation(expected)?;
    let expected_object = ExpectedObject::for_target(expected.target)?;
    let metadata = fs::symlink_metadata(archive_path).map_err(|error| {
        format!(
            "failed to inspect provider archive {}: {error}",
            archive_path.display()
        )
    })?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(format!(
            "provider archive must be a regular non-symlink file: {}",
            archive_path.display()
        ));
    }
    if metadata.len() > MAX_ARCHIVE_BYTES {
        return Err(format!(
            "provider archive {} exceeds the {} byte verification limit",
            archive_path.display(),
            MAX_ARCHIVE_BYTES
        ));
    }
    let bytes = fs::read(archive_path).map_err(|error| {
        format!(
            "failed to read provider archive {}: {error}",
            archive_path.display()
        )
    })?;
    if bytes.len() as u64 > MAX_ARCHIVE_BYTES {
        return Err(format!(
            "provider archive {} grew beyond the {} byte verification limit while being read",
            archive_path.display(),
            MAX_ARCHIVE_BYTES
        ));
    }
    let archive_sha256 = hex_sha256(&bytes);
    let archive = object::read::archive::ArchiveFile::parse(bytes.as_slice()).map_err(|error| {
        format!(
            "provider library {} is not a supported static archive: {error}",
            archive_path.display()
        )
    })?;
    if archive.is_thin() {
        return Err(format!(
            "provider library {} must not be a thin archive",
            archive_path.display()
        ));
    }

    let mut object_bytes = Vec::new();
    let mut member_definitions = BTreeMap::new();
    let mut required_definitions = expected
        .required_symbols
        .iter()
        .map(|symbol| ((*symbol).to_owned(), 0_usize))
        .collect::<BTreeMap<_, _>>();
    let mut member_count = 0_usize;
    for member in archive.members() {
        member_count = member_count
            .checked_add(1)
            .ok_or_else(|| "provider archive member count overflow".to_owned())?;
        if member_count > MAX_ARCHIVE_MEMBERS {
            return Err(format!(
                "provider archive {} exceeds the {} member verification limit",
                archive_path.display(),
                MAX_ARCHIVE_MEMBERS
            ));
        }
        let member = member.map_err(|error| {
            format!(
                "failed to read a static archive member in {}: {error}",
                archive_path.display()
            )
        })?;
        let data = member.data(bytes.as_slice()).map_err(|error| {
            format!(
                "failed to read static archive member data in {}: {error}",
                archive_path.display()
            )
        })?;
        let file = object::File::parse(data).map_err(|error| {
            format!(
                "provider archive {} contains unsupported member {:?}: {error}",
                archive_path.display(),
                String::from_utf8_lossy(member.name())
            )
        })?;
        if file.kind() != object::ObjectKind::Relocatable {
            return Err(format!(
                "provider archive {} member {:?} is {:?}, expected a relocatable object",
                archive_path.display(),
                String::from_utf8_lossy(member.name()),
                file.kind()
            ));
        }
        if !expected_object.accepts(&file) {
            return Err(format!(
                "provider archive {} contains {:?}/{:?}/{:?}/{}-endian object, expected target {}",
                archive_path.display(),
                file.format(),
                file.architecture(),
                file.architecture().address_size(),
                if file.is_little_endian() {
                    "little"
                } else {
                    "big"
                },
                expected.target
            ));
        }
        require_target_platform(&file, expected.target)?;
        let definitions = count_required_definitions(&file, &mut required_definitions)?;
        let range = member.file_range();
        if member_definitions.insert(range, definitions).is_some() {
            return Err(format!(
                "provider archive {} contains duplicate member ranges",
                archive_path.display()
            ));
        }
        object_bytes.push(data);
    }
    if object_bytes.is_empty() {
        return Err(format!(
            "provider archive {} contains no target object files",
            archive_path.display()
        ));
    }
    require_exact_definitions(archive_path, &required_definitions)?;
    verify_archive_symbol_index(
        &archive,
        expected_object.format,
        expected.required_symbols,
        &member_definitions,
        archive_path,
    )?;

    let effective_bytes = unique_symbol_bytes(&object_bytes, EFFECTIVE_SOURCE_SYMBOL, 65)?;
    let effective_source_sha256 = parse_effective_source_sha256(&effective_bytes)?;
    if effective_source_sha256 != expected.effective_source_sha256 {
        return Err(format!(
            "provider archive effective-source SHA-256 {} does not match {}",
            effective_source_sha256, expected.effective_source_sha256
        ));
    }

    let private_count = read_identity_count(&object_bytes, PRIVATE_COUNT_SYMBOL)?;
    let layout_count = read_identity_count(&object_bytes, LAYOUT_COUNT_SYMBOL)?;
    let private_values = read_identity_values(&object_bytes, PRIVATE_VALUES_SYMBOL, private_count)?;
    let layout_values = read_identity_values(&object_bytes, LAYOUT_VALUES_SYMBOL, layout_count)?;
    let private_abi_hash = private_abi_hash_hex(private_abi_hash(&private_values, true));
    if private_abi_hash != expected.private_abi_hash {
        return Err(format!(
            "provider archive private ABI hash {private_abi_hash} does not match {}",
            expected.private_abi_hash
        ));
    }
    let snapshot_layout_hash = snapshot_layout_hash(&layout_values);
    if snapshot_layout_hash != expected.snapshot_layout_hash {
        return Err(format!(
            "provider archive snapshot layout hash {snapshot_layout_hash:#010x} does not match {:#010x}",
            expected.snapshot_layout_hash
        ));
    }

    Ok(VerifiedArchiveIdentity {
        effective_source_sha256,
        private_abi_hash,
        snapshot_layout_hash,
        object_count: object_bytes.len(),
        archive_sha256,
        archive_bytes: bytes,
    })
}

fn hex_sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn validate_expectation(expected: &ArchiveExpectation<'_>) -> Result<(), String> {
    validate_lower_hex(
        "expected effective-source SHA-256",
        expected.effective_source_sha256,
    )?;
    validate_lower_hex("expected private ABI hash", expected.private_abi_hash)?;
    if expected.required_symbols.is_empty() {
        return Err("provider archive required-symbol contract must not be empty".to_owned());
    }
    for pair in expected.required_symbols.windows(2) {
        if pair[0] >= pair[1] {
            return Err(
                "provider archive required symbols must be strictly sorted without duplicates"
                    .to_owned(),
            );
        }
    }
    for symbol in expected.required_symbols {
        if symbol.is_empty() || symbol.starts_with('_') {
            return Err(format!(
                "provider archive required symbol {symbol:?} is not canonical"
            ));
        }
    }
    Ok(())
}

fn validate_lower_hex(label: &str, value: &str) -> Result<(), String> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(format!(
            "{label} must be exactly 64 lowercase hexadecimal bytes"
        ))
    }
}

fn count_required_definitions(
    file: &object::File<'_>,
    required: &mut BTreeMap<String, usize>,
) -> Result<BTreeSet<String>, String> {
    let mut definitions = BTreeSet::new();
    for symbol in file.symbols() {
        let Ok(raw_name) = symbol.name() else {
            continue;
        };
        let Some(name) = canonical_symbol_name(file.format(), raw_name) else {
            continue;
        };
        if !required.contains_key(name) || symbol.is_undefined() {
            continue;
        }
        require_external_definition(&symbol, name)?;
        require_symbol_kind(&symbol, name, required_symbol_kind(name))?;
        let count = required
            .get_mut(name)
            .expect("required symbol membership was checked");
        *count = count
            .checked_add(1)
            .ok_or_else(|| format!("definition count overflow for provider symbol {name}"))?;
        definitions.insert(name.to_owned());
    }
    Ok(definitions)
}

fn verify_archive_symbol_index(
    archive: &object::read::archive::ArchiveFile<'_>,
    format: object::BinaryFormat,
    required_symbols: &[&str],
    member_definitions: &BTreeMap<(u64, u64), BTreeSet<String>>,
    archive_path: &Path,
) -> Result<(), String> {
    let symbols = archive
        .symbols()
        .map_err(|error| {
            format!(
                "failed to parse provider archive symbol index in {}: {error}",
                archive_path.display()
            )
        })?
        .ok_or_else(|| {
            format!(
                "provider archive {} must contain a linker symbol index",
                archive_path.display()
            )
        })?;
    let mut indexed = required_symbols
        .iter()
        .map(|symbol| ((*symbol).to_owned(), 0_usize))
        .collect::<BTreeMap<_, _>>();
    for (symbol_index, symbol) in symbols.enumerate() {
        if symbol_index >= MAX_ARCHIVE_SYMBOLS {
            return Err(format!(
                "provider archive {} exceeds the {} symbol-index entry limit",
                archive_path.display(),
                MAX_ARCHIVE_SYMBOLS
            ));
        }
        let symbol = symbol.map_err(|error| {
            format!(
                "failed to read provider archive symbol index in {}: {error}",
                archive_path.display()
            )
        })?;
        let Ok(raw_name) = std::str::from_utf8(symbol.name()) else {
            continue;
        };
        let Some(name) = canonical_symbol_name(format, raw_name) else {
            continue;
        };
        let Some(count) = indexed.get_mut(name) else {
            continue;
        };
        let member = archive.member(symbol.offset()).map_err(|error| {
            format!(
                "provider archive symbol {name} points to an invalid member in {}: {error}",
                archive_path.display()
            )
        })?;
        let definitions = member_definitions
            .get(&member.file_range())
            .ok_or_else(|| {
                format!(
                    "provider archive symbol {name} points outside the verified member set in {}",
                    archive_path.display()
                )
            })?;
        if !definitions.contains(name) {
            return Err(format!(
                "provider archive symbol index maps {name} to a member that does not define it"
            ));
        }
        *count = count
            .checked_add(1)
            .ok_or_else(|| format!("archive index count overflow for provider symbol {name}"))?;
    }
    require_exact_definitions(archive_path, &indexed)
        .map_err(|error| format!("provider archive linker symbol index is incomplete: {error}"))
}

fn require_external_definition(symbol: &object::Symbol<'_, '_>, name: &str) -> Result<(), String> {
    if !symbol.is_definition()
        || !symbol.is_global()
        || symbol.is_weak()
        || symbol.section_index().is_none()
    {
        Err(format!(
            "provider symbol {name} must be a strong external/global section definition"
        ))
    } else {
        Ok(())
    }
}

fn require_symbol_kind(
    symbol: &object::Symbol<'_, '_>,
    name: &str,
    expected: object::SymbolKind,
) -> Result<(), String> {
    if symbol.kind() == expected {
        Ok(())
    } else {
        Err(format!(
            "provider symbol {name} has kind {:?}, expected {expected:?}",
            symbol.kind()
        ))
    }
}

fn required_symbol_kind(name: &str) -> object::SymbolKind {
    if name == EFFECTIVE_SOURCE_SYMBOL {
        object::SymbolKind::Data
    } else {
        object::SymbolKind::Text
    }
}

fn require_exact_definitions(
    archive_path: &Path,
    definitions: &BTreeMap<String, usize>,
) -> Result<(), String> {
    let invalid = definitions
        .iter()
        .filter_map(|(symbol, count)| {
            (*count != 1).then_some(format!("{symbol} ({count} definitions)"))
        })
        .collect::<Vec<_>>();
    if invalid.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "provider archive {} must define every required adapter symbol exactly once: {}",
            archive_path.display(),
            invalid.join(", ")
        ))
    }
}

fn canonical_symbol_name(format: object::BinaryFormat, name: &str) -> Option<&str> {
    if format == object::BinaryFormat::MachO {
        name.strip_prefix('_')
    } else {
        Some(name)
    }
}

fn read_identity_count(objects: &[&[u8]], name: &str) -> Result<usize, String> {
    let bytes = unique_symbol_bytes(objects, name, 8)?;
    let value = u64::from_le_bytes(
        bytes
            .as_slice()
            .try_into()
            .expect("identity count has a fixed width"),
    );
    let count = usize::try_from(value)
        .map_err(|_| format!("provider identity count {value} for {name} is too large"))?;
    if count == 0 || count > MAX_IDENTITY_VALUES {
        return Err(format!(
            "provider identity count {count} for {name} is outside 1..={MAX_IDENTITY_VALUES}"
        ));
    }
    Ok(count)
}

fn read_identity_values(objects: &[&[u8]], name: &str, count: usize) -> Result<Vec<u64>, String> {
    let width = count
        .checked_mul(8)
        .ok_or_else(|| format!("provider identity array {name} byte width overflow"))?;
    unique_symbol_bytes(objects, name, width)?
        .chunks_exact(8)
        .map(|bytes| {
            Ok(u64::from_le_bytes(
                bytes
                    .try_into()
                    .expect("identity array values have a fixed width"),
            ))
        })
        .collect()
}

fn unique_symbol_bytes(objects: &[&[u8]], name: &str, width: usize) -> Result<Vec<u8>, String> {
    let mut payload = None;
    let mut definitions = 0_usize;
    for bytes in objects {
        let file = object::File::parse(*bytes)
            .map_err(|error| format!("failed to reparse verified target object: {error}"))?;
        for symbol in file.symbols() {
            let Ok(raw_name) = symbol.name() else {
                continue;
            };
            if canonical_symbol_name(file.format(), raw_name) != Some(name) || symbol.is_undefined()
            {
                continue;
            }
            require_external_definition(&symbol, name)?;
            require_symbol_kind(&symbol, name, object::SymbolKind::Data)?;
            definitions = definitions
                .checked_add(1)
                .ok_or_else(|| format!("definition count overflow for identity symbol {name}"))?;
            if definitions > 1 {
                continue;
            }
            let symbol_size = symbol.size();
            if symbol_size != 0 && symbol_size != width as u64 {
                return Err(format!(
                    "provider identity symbol {name} has size {symbol_size}, expected {width}"
                ));
            }
            let section_index = symbol
                .section_index()
                .expect("external section definition must have a section");
            let section = file.section_by_index(section_index).map_err(|error| {
                format!("provider identity symbol {name} has an invalid section: {error}")
            })?;
            let offset = symbol
                .address()
                .checked_sub(section.address())
                .and_then(|offset| usize::try_from(offset).ok())
                .ok_or_else(|| format!("provider identity symbol {name} has an invalid address"))?;
            let end = offset
                .checked_add(width)
                .ok_or_else(|| format!("provider identity symbol {name} byte range overflow"))?;
            if section.relocations().next().is_some() {
                return Err(format!(
                    "provider identity section for {name} must not contain relocations"
                ));
            }
            let data = section.data().map_err(|error| {
                format!("provider identity section for {name} is unreadable: {error}")
            })?;
            payload = Some(
                data.get(offset..end)
                    .ok_or_else(|| format!("provider identity symbol {name} is truncated"))?
                    .to_vec(),
            );
        }
    }
    match (definitions, payload) {
        (1, Some(payload)) => Ok(payload),
        (0, _) => Err(format!(
            "provider archive is missing identity symbol {name}"
        )),
        (count, _) => Err(format!(
            "provider archive contains {count} definitions of identity symbol {name}"
        )),
    }
}

fn parse_effective_source_sha256(bytes: &[u8]) -> Result<String, String> {
    if bytes.len() != 65 || bytes[64] != 0 {
        return Err(
            "boxddEffectiveSourceSha256 must contain 64 lowercase hexadecimal bytes followed by NUL"
                .to_owned(),
        );
    }
    let digest = std::str::from_utf8(&bytes[..64])
        .map_err(|error| format!("boxddEffectiveSourceSha256 is not canonical UTF-8: {error}"))?;
    validate_lower_hex("boxddEffectiveSourceSha256", digest)?;
    Ok(digest.to_owned())
}

pub fn private_abi_hash(values: &[u64], little_endian: bool) -> [u8; 32] {
    let mut state = [
        14_695_981_039_346_656_037_u64,
        0x6A09_E667_F3BC_C909,
        0xBB67_AE85_84CA_A73B,
        0x3C6E_F372_FE94_F82B,
    ];
    let primes = [
        1_099_511_628_211_u64,
        14_029_467_366_897_019_727,
        1_609_587_929_392_839_161,
        9_650_029_242_287_828_579,
    ];
    for value in values {
        for index in 0..4 {
            state[index] ^= value.wrapping_add((index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
            state[index] = state[index].wrapping_mul(primes[index]);
            state[index] ^= state[index] >> 29;
        }
    }
    let mut hash = [0; 32];
    for (index, value) in state.into_iter().enumerate() {
        let bytes = if little_endian {
            value.to_le_bytes()
        } else {
            value.to_be_bytes()
        };
        hash[index * 8..(index + 1) * 8].copy_from_slice(&bytes);
    }
    hash
}

pub fn snapshot_layout_hash(values: &[u64]) -> u32 {
    values.iter().fold(2_166_136_261_u32, |hash, value| {
        (hash ^ *value as u32).wrapping_mul(16_777_619)
    })
}

pub fn private_abi_hash_hex(hash: [u8; 32]) -> String {
    hash.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use object::write::{Object as WritableObject, StandardSection, Symbol, SymbolSection};
    use object::{
        Architecture, BinaryFormat, Endianness, RelocationEncoding, RelocationFlags,
        RelocationKind, SubArchitecture, SymbolFlags, SymbolKind, SymbolScope,
        macho::{PLATFORM_IOS, PLATFORM_MACOS},
        write::{MachOBuildVersion, Relocation as WritableRelocation},
    };
    use std::io::Write;
    use tempfile::tempdir;

    const EFFECTIVE: &str = "9948291f4ea6e14b01304d19473e4539f47313133b4c2e7c6f3ae312d4f2c112";
    const REQUIRED: &[&str] = &[
        "boxddAdapter_AbiVersion",
        "boxddAdapter_GetIdentity",
        "boxddAdapter_GetSnapshotLayoutHash",
        "boxddEffectiveSourceSha256",
        "boxddRecPlayer_IsHealthy",
        "boxddSnapshot_Validate",
    ];
    const PRIVATE_VALUES: &[u64] = &[1, 2, 3, 0x1234_5678_9abc_def0];
    const LAYOUT_VALUES: &[u64] = &[4, 5, 6];

    #[derive(Clone)]
    struct Fixture {
        format: BinaryFormat,
        architecture: Architecture,
        target: &'static str,
        effective: Vec<u8>,
        private_count: u64,
        private_values: Vec<u64>,
        layout_count: u64,
        layout_values: Vec<u64>,
        expected_private_abi_hash: String,
        omitted_required: Option<&'static str>,
        local_required: Option<&'static str>,
        weak_required: Option<&'static str>,
        data_required: Option<&'static str>,
        sub_architecture: Option<SubArchitecture>,
        macho_platform: u32,
        relocate_private_values: bool,
    }

    impl Fixture {
        fn elf() -> Self {
            Self {
                format: BinaryFormat::Elf,
                architecture: Architecture::X86_64,
                target: "x86_64-unknown-linux-gnu",
                effective: format!("{EFFECTIVE}\0").into_bytes(),
                private_count: PRIVATE_VALUES.len() as u64,
                private_values: PRIVATE_VALUES.to_vec(),
                layout_count: LAYOUT_VALUES.len() as u64,
                layout_values: LAYOUT_VALUES.to_vec(),
                expected_private_abi_hash: private_abi_hash_hex(private_abi_hash(
                    PRIVATE_VALUES,
                    true,
                )),
                omitted_required: None,
                local_required: None,
                weak_required: None,
                data_required: None,
                sub_architecture: None,
                macho_platform: PLATFORM_MACOS,
                relocate_private_values: false,
            }
        }

        fn expectation(&self) -> ArchiveExpectation<'_> {
            ArchiveExpectation {
                target: self.target,
                required_symbols: REQUIRED,
                effective_source_sha256: EFFECTIVE,
                private_abi_hash: &self.expected_private_abi_hash,
                snapshot_layout_hash: snapshot_layout_hash(&self.layout_values),
            }
        }

        fn object(&self) -> Vec<u8> {
            let mut object =
                WritableObject::new(self.format, self.architecture, Endianness::Little);
            object.set_sub_architecture(self.sub_architecture);
            if self.format == BinaryFormat::MachO {
                let mut version = MachOBuildVersion::default();
                version.platform = self.macho_platform;
                version.minos = 0x000b_0000;
                version.sdk = 0x000b_0000;
                object.set_macho_build_version(version);
            }
            let text = object.section_id(StandardSection::Text);
            let data = object.section_id(StandardSection::Data);
            for required in REQUIRED {
                if self.omitted_required == Some(*required) {
                    continue;
                }
                let payload = if *required == EFFECTIVE_SOURCE_SYMBOL {
                    self.effective.clone()
                } else {
                    vec![0]
                };
                let is_data =
                    *required == EFFECTIVE_SOURCE_SYMBOL || self.data_required == Some(*required);
                let section = if is_data { data } else { text };
                let offset = object.append_section_data(section, &payload, 1);
                object.add_symbol(Symbol {
                    name: required.as_bytes().to_vec(),
                    value: offset,
                    size: payload.len() as u64,
                    kind: if is_data {
                        SymbolKind::Data
                    } else {
                        SymbolKind::Text
                    },
                    scope: if self.local_required == Some(*required) {
                        SymbolScope::Compilation
                    } else {
                        SymbolScope::Linkage
                    },
                    weak: self.weak_required == Some(*required),
                    section: SymbolSection::Section(section),
                    flags: SymbolFlags::None,
                });
            }
            self.add_u64_symbol(&mut object, data, PRIVATE_COUNT_SYMBOL, self.private_count);
            let private_values_offset = self.add_values_symbol(
                &mut object,
                data,
                PRIVATE_VALUES_SYMBOL,
                &self.private_values,
            );
            self.add_u64_symbol(&mut object, data, LAYOUT_COUNT_SYMBOL, self.layout_count);
            self.add_values_symbol(&mut object, data, LAYOUT_VALUES_SYMBOL, &self.layout_values);
            if self.relocate_private_values {
                let target = object.add_symbol(Symbol {
                    name: b"externalIdentityValue".to_vec(),
                    value: 0,
                    size: 0,
                    kind: SymbolKind::Data,
                    scope: SymbolScope::Unknown,
                    weak: false,
                    section: SymbolSection::Undefined,
                    flags: SymbolFlags::None,
                });
                object
                    .add_relocation(
                        data,
                        WritableRelocation {
                            offset: private_values_offset,
                            symbol: target,
                            addend: 0,
                            flags: RelocationFlags::Generic {
                                kind: RelocationKind::Absolute,
                                encoding: RelocationEncoding::Generic,
                                size: 64,
                            },
                        },
                    )
                    .unwrap();
            }
            object.write().unwrap()
        }

        fn add_u64_symbol(
            &self,
            object: &mut WritableObject<'_>,
            section: object::write::SectionId,
            name: &str,
            value: u64,
        ) {
            self.add_data_symbol(object, section, name, &value.to_le_bytes());
        }

        fn add_values_symbol(
            &self,
            object: &mut WritableObject<'_>,
            section: object::write::SectionId,
            name: &str,
            values: &[u64],
        ) -> u64 {
            let bytes = values
                .iter()
                .flat_map(|value| value.to_le_bytes())
                .collect::<Vec<_>>();
            self.add_data_symbol(object, section, name, &bytes)
        }

        fn add_data_symbol(
            &self,
            object: &mut WritableObject<'_>,
            section: object::write::SectionId,
            name: &str,
            bytes: &[u8],
        ) -> u64 {
            let offset = object.append_section_data(section, bytes, 8);
            object.add_symbol(Symbol {
                name: name.as_bytes().to_vec(),
                value: offset,
                size: bytes.len() as u64,
                kind: SymbolKind::Data,
                scope: SymbolScope::Linkage,
                weak: false,
                section: SymbolSection::Section(section),
                flags: SymbolFlags::None,
            });
            offset
        }
    }

    fn archive(members: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut symbols = Vec::new();
        for (member_index, (_, data)) in members.iter().enumerate() {
            let Ok(file) = object::File::parse(data.as_slice()) else {
                continue;
            };
            for symbol in file.symbols() {
                if symbol.is_definition()
                    && symbol.is_global()
                    && !symbol.is_weak()
                    && symbol.section_index().is_some()
                    && let Ok(name) = symbol.name_bytes()
                {
                    symbols.push((name.to_vec(), member_index));
                }
            }
        }
        let symbol_table_size = 4
            + symbols.len() * 4
            + symbols
                .iter()
                .map(|(name, _)| name.len() + 1)
                .sum::<usize>();
        let mut next_member_offset = 8 + 60 + symbol_table_size + symbol_table_size % 2;
        let mut member_offsets = Vec::with_capacity(members.len());
        for (_, data) in members {
            member_offsets.push(next_member_offset);
            next_member_offset += 60 + data.len() + data.len() % 2;
        }

        let mut archive = b"!<arch>\n".to_vec();
        append_archive_header(&mut archive, "/", symbol_table_size);
        archive.extend_from_slice(&u32::try_from(symbols.len()).unwrap().to_be_bytes());
        for (_, member_index) in &symbols {
            archive.extend_from_slice(
                &u32::try_from(member_offsets[*member_index])
                    .unwrap()
                    .to_be_bytes(),
            );
        }
        for (name, _) in &symbols {
            archive.extend_from_slice(name);
            archive.push(0);
        }
        if symbol_table_size % 2 != 0 {
            archive.push(b'\n');
        }
        for (name, data) in members {
            assert!(name.len() <= 15);
            append_archive_header(&mut archive, &format!("{name}/"), data.len());
            archive.extend_from_slice(data);
            if data.len() % 2 != 0 {
                archive.push(b'\n');
            }
        }
        archive
    }

    fn append_archive_header(archive: &mut Vec<u8>, name: &str, size: usize) {
        writeln!(
            archive,
            "{name:<16}{:<12}{:<6}{:<6}{:<8}{size:<10}`",
            0, 0, 0, 0o100644
        )
        .unwrap();
    }

    fn archive_without_index(members: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut archive = b"!<arch>\n".to_vec();
        for (name, data) in members {
            assert!(name.len() <= 15);
            append_archive_header(&mut archive, &format!("{name}/"), data.len());
            archive.extend_from_slice(data);
            if data.len() % 2 != 0 {
                archive.push(b'\n');
            }
        }
        archive
    }

    fn verify_fixture(fixture: &Fixture, members: &[(&str, Vec<u8>)]) -> Result<(), String> {
        let directory = tempdir().unwrap();
        let path = directory.path().join("libbox2d.a");
        fs::write(&path, archive(members)).unwrap();
        verify_provider_archive(&path, &fixture.expectation()).map(|_| ())
    }

    #[test]
    fn accepts_supported_structured_archives() {
        for fixture in [
            Fixture::elf(),
            Fixture {
                format: BinaryFormat::MachO,
                architecture: Architecture::X86_64,
                target: "x86_64-apple-darwin",
                ..Fixture::elf()
            },
            Fixture {
                format: BinaryFormat::MachO,
                architecture: Architecture::Aarch64,
                target: "aarch64-apple-darwin",
                ..Fixture::elf()
            },
            Fixture {
                format: BinaryFormat::Coff,
                architecture: Architecture::X86_64,
                target: "x86_64-pc-windows-msvc",
                ..Fixture::elf()
            },
        ] {
            verify_fixture(&fixture, &[("identity.o", fixture.object())]).unwrap();
        }
    }

    #[test]
    fn returns_the_exact_verified_archive_snapshot() {
        let fixture = Fixture::elf();
        let bytes = archive(&[("identity.o", fixture.object())]);
        let directory = tempdir().unwrap();
        let path = directory.path().join("snapshot.a");
        fs::write(&path, &bytes).unwrap();
        let verified = verify_provider_archive(&path, &fixture.expectation()).unwrap();
        assert_eq!(verified.archive_bytes, bytes);
        assert_eq!(verified.archive_sha256, hex_sha256(&verified.archive_bytes));
        assert_eq!(verified.object_count, 1);
    }

    #[test]
    fn rejects_non_archive_thin_and_wrong_target_inputs() {
        let fixture = Fixture::elf();
        let directory = tempdir().unwrap();
        let path = directory.path().join("library.a");
        fs::write(&path, b"not an archive").unwrap();
        assert!(verify_provider_archive(&path, &fixture.expectation()).is_err());

        fs::write(&path, b"!<thin>\n").unwrap();
        let error = verify_provider_archive(&path, &fixture.expectation()).unwrap_err();
        assert!(error.contains("thin archive"), "{error}");

        let wrong = Fixture {
            format: BinaryFormat::MachO,
            architecture: Architecture::Aarch64,
            target: fixture.target,
            ..fixture.clone()
        };
        let error = verify_fixture(&fixture, &[("wrong.o", wrong.object())]).unwrap_err();
        assert!(error.contains("expected target"), "{error}");
    }

    #[test]
    fn rejects_opaque_non_relocatable_and_wrong_darwin_members() {
        let fixture = Fixture::elf();
        let error = verify_fixture(
            &fixture,
            &[
                ("identity.o", fixture.object()),
                ("opaque.o", vec![1, 2, 3]),
            ],
        )
        .unwrap_err();
        assert!(error.contains("unsupported member"), "{error}");

        let mut executable = fixture.object();
        executable[16..18].copy_from_slice(&2_u16.to_le_bytes());
        let error = verify_fixture(&fixture, &[("executable.o", executable)]).unwrap_err();
        assert!(error.contains("expected a relocatable object"), "{error}");

        let ios = Fixture {
            format: BinaryFormat::MachO,
            architecture: Architecture::Aarch64,
            target: "aarch64-apple-darwin",
            macho_platform: PLATFORM_IOS,
            ..Fixture::elf()
        };
        let error = verify_fixture(&ios, &[("ios.o", ios.object())]).unwrap_err();
        assert!(error.contains("expected macOS"), "{error}");

        let arm64e = Fixture {
            format: BinaryFormat::MachO,
            architecture: Architecture::Aarch64,
            target: "aarch64-apple-darwin",
            sub_architecture: Some(SubArchitecture::Arm64E),
            ..Fixture::elf()
        };
        let error = verify_fixture(&arm64e, &[("arm64e.o", arm64e.object())]).unwrap_err();
        assert!(error.contains("expected target"), "{error}");
    }

    #[test]
    fn rejects_missing_or_misdirected_linker_indexes() {
        let fixture = Fixture::elf();
        let object = fixture.object();
        let directory = tempdir().unwrap();
        let path = directory.path().join("index.a");
        fs::write(
            &path,
            archive_without_index(&[("identity.o", object.clone())]),
        )
        .unwrap();
        let error = verify_provider_archive(&path, &fixture.expectation()).unwrap_err();
        assert!(error.contains("linker symbol index"), "{error}");

        let mut bytes = archive(&[("identity.o", object)]);
        let symbol_count = u32::from_be_bytes(bytes[68..72].try_into().unwrap()) as usize;
        assert!(symbol_count >= REQUIRED.len());
        for index in 0..symbol_count {
            let offset = 72 + index * 4;
            bytes[offset..offset + 4].copy_from_slice(&8_u32.to_be_bytes());
        }
        fs::write(&path, bytes).unwrap();
        let error = verify_provider_archive(&path, &fixture.expectation()).unwrap_err();
        assert!(error.contains("points to an invalid member"), "{error}");
    }

    #[test]
    fn rejects_relocated_identity_payloads() {
        let mut fixture = Fixture::elf();
        fixture.relocate_private_values = true;
        let error = verify_fixture(&fixture, &[("relocated.o", fixture.object())]).unwrap_err();
        assert!(error.contains("must not contain relocations"), "{error}");
    }

    #[test]
    fn rejects_missing_duplicate_weak_non_global_or_wrong_kind_required_symbols() {
        let mut missing = Fixture::elf();
        missing.omitted_required = Some("boxddAdapter_GetIdentity");
        let error = verify_fixture(&missing, &[("missing.o", missing.object())]).unwrap_err();
        assert!(error.contains("0 definitions"), "{error}");

        let fixture = Fixture::elf();
        let error = verify_fixture(
            &fixture,
            &[
                ("first.o", fixture.object()),
                ("second.o", fixture.object()),
            ],
        )
        .unwrap_err();
        assert!(error.contains("2 definitions"), "{error}");

        let mut local = Fixture::elf();
        local.local_required = Some("boxddAdapter_GetIdentity");
        let error = verify_fixture(&local, &[("local.o", local.object())]).unwrap_err();
        assert!(error.contains("external/global"), "{error}");

        let mut weak = Fixture::elf();
        weak.weak_required = Some("boxddAdapter_GetIdentity");
        let error = verify_fixture(&weak, &[("weak.o", weak.object())]).unwrap_err();
        assert!(error.contains("strong external/global"), "{error}");

        let mut data = Fixture::elf();
        data.data_required = Some("boxddAdapter_GetIdentity");
        let error = verify_fixture(&data, &[("data.o", data.object())]).unwrap_err();
        assert!(error.contains("expected Text"), "{error}");
    }

    #[test]
    fn applies_the_exact_target_symbol_prefix_contract() {
        assert_eq!(
            canonical_symbol_name(BinaryFormat::MachO, "_boxddAdapter_AbiVersion"),
            Some("boxddAdapter_AbiVersion")
        );
        assert_eq!(
            canonical_symbol_name(BinaryFormat::MachO, "__boxddAdapter_AbiVersion"),
            Some("_boxddAdapter_AbiVersion")
        );
        assert_eq!(
            canonical_symbol_name(BinaryFormat::MachO, "boxddAdapter_AbiVersion"),
            None
        );
        assert_eq!(
            canonical_symbol_name(BinaryFormat::Elf, "_boxddAdapter_AbiVersion"),
            Some("_boxddAdapter_AbiVersion")
        );
    }

    #[test]
    fn rejects_noncanonical_or_drifted_effective_source_payloads() {
        let mut fixture = Fixture::elf();
        fixture.effective = format!("{}\0", EFFECTIVE.to_ascii_uppercase()).into_bytes();
        let error = verify_fixture(&fixture, &[("uppercase.o", fixture.object())]).unwrap_err();
        assert!(error.contains("lowercase hexadecimal"), "{error}");

        fixture = Fixture::elf();
        fixture.effective[64] = b'x';
        let error = verify_fixture(&fixture, &[("no-nul.o", fixture.object())]).unwrap_err();
        assert!(error.contains("followed by NUL"), "{error}");

        fixture = Fixture::elf();
        let mut expected = fixture.expectation();
        expected.effective_source_sha256 =
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let directory = tempdir().unwrap();
        let path = directory.path().join("drift.a");
        fs::write(&path, archive(&[("drift.o", fixture.object())])).unwrap();
        let error = verify_provider_archive(&path, &expected).unwrap_err();
        assert!(error.contains("does not match"), "{error}");
    }

    #[test]
    fn rejects_identity_count_overflow_truncation_and_hash_drift() {
        let mut overflow = Fixture::elf();
        overflow.private_count = u64::MAX;
        let error = verify_fixture(&overflow, &[("overflow.o", overflow.object())]).unwrap_err();
        assert!(error.contains("outside 1..="), "{error}");

        let mut truncated = Fixture::elf();
        truncated.private_count += 1;
        let error = verify_fixture(&truncated, &[("truncated.o", truncated.object())]).unwrap_err();
        assert!(
            error.contains("has size") || error.contains("truncated"),
            "{error}"
        );

        let fixture = Fixture::elf();
        let directory = tempdir().unwrap();
        let path = directory.path().join("drift.a");
        fs::write(&path, archive(&[("drift.o", fixture.object())])).unwrap();
        let mut expected = fixture.expectation();
        expected.private_abi_hash =
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let error = verify_provider_archive(&path, &expected).unwrap_err();
        assert!(error.contains("private ABI hash"), "{error}");

        let mut expected = fixture.expectation();
        expected.snapshot_layout_hash ^= 1;
        let error = verify_provider_archive(&path, &expected).unwrap_err();
        assert!(error.contains("snapshot layout hash"), "{error}");
    }
}
