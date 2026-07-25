use std::fmt;

use wasmparser::{KnownCustom, Linking, Parser, Payload, SymbolInfo};

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum IdentityObjectError {
    Parse {
        context: &'static str,
        message: String,
    },
    MissingSymbol {
        name: String,
    },
    DuplicateSymbol {
        name: String,
    },
    InvalidSymbolSize {
        name: String,
        actual: u32,
        expected: usize,
    },
    InvalidSegmentIndex {
        name: String,
        index: u32,
    },
    InvalidSymbolOffset {
        name: String,
        offset: u32,
    },
    TruncatedSymbol {
        name: String,
        offset: u32,
        size: usize,
        segment_size: usize,
    },
}

impl fmt::Display for IdentityObjectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse { context, message } => {
                write!(formatter, "failed to parse WASM {context}: {message}")
            }
            Self::MissingSymbol { name } => {
                write!(formatter, "target object is missing identity symbol {name}")
            }
            Self::DuplicateSymbol { name } => {
                write!(
                    formatter,
                    "WASM identity object defines {name} more than once"
                )
            }
            Self::InvalidSymbolSize {
                name,
                actual,
                expected,
            } => write!(
                formatter,
                "WASM identity symbol {name} has size {actual}, expected {expected}"
            ),
            Self::InvalidSegmentIndex { name, index } => write!(
                formatter,
                "WASM identity symbol {name} references missing segment {index}"
            ),
            Self::InvalidSymbolOffset { name, offset } => write!(
                formatter,
                "WASM identity symbol {name} has invalid offset {offset}"
            ),
            Self::TruncatedSymbol {
                name,
                offset,
                size,
                segment_size,
            } => write!(
                formatter,
                "WASM identity symbol {name} range {offset}..{} exceeds segment size {segment_size}",
                usize::try_from(*offset)
                    .ok()
                    .and_then(|start| start.checked_add(*size))
                    .map_or_else(|| "overflow".to_owned(), |end| end.to_string())
            ),
        }
    }
}

impl std::error::Error for IdentityObjectError {}

pub(crate) fn symbol_bytes<'data>(
    object_bytes: &'data [u8],
    expected_name: &str,
    width: usize,
) -> Result<&'data [u8], IdentityObjectError> {
    let mut data_segments = Vec::new();
    let mut definition = None;

    for payload in Parser::new(0).parse_all(object_bytes) {
        match payload.map_err(|error| parse_error("identity object", error))? {
            Payload::DataSection(reader) => {
                for data in reader {
                    let data = data.map_err(|error| parse_error("identity data segment", error))?;
                    data_segments.push(data.data);
                }
            }
            Payload::CustomSection(custom) if custom.name() == "linking" => {
                let KnownCustom::Linking(linking) = custom.as_known() else {
                    return Err(IdentityObjectError::Parse {
                        context: "linking metadata",
                        message: "unsupported linking section".to_owned(),
                    });
                };
                for subsection in linking {
                    let subsection =
                        subsection.map_err(|error| parse_error("linking metadata", error))?;
                    let Linking::SymbolTable(symbols) = subsection else {
                        continue;
                    };
                    for symbol in symbols {
                        let symbol =
                            symbol.map_err(|error| parse_error("identity symbol table", error))?;
                        if let SymbolInfo::Data {
                            name,
                            symbol: Some(symbol),
                            ..
                        } = symbol
                            && name == expected_name
                            && definition.replace(symbol).is_some()
                        {
                            return Err(IdentityObjectError::DuplicateSymbol {
                                name: expected_name.to_owned(),
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    let definition = definition.ok_or_else(|| IdentityObjectError::MissingSymbol {
        name: expected_name.to_owned(),
    })?;
    let symbol_size = usize::try_from(definition.size).ok();
    if symbol_size != Some(width) {
        return Err(IdentityObjectError::InvalidSymbolSize {
            name: expected_name.to_owned(),
            actual: definition.size,
            expected: width,
        });
    }
    let segment_index = usize::try_from(definition.index).map_err(|_| {
        IdentityObjectError::InvalidSegmentIndex {
            name: expected_name.to_owned(),
            index: definition.index,
        }
    })?;
    let segment = data_segments.get(segment_index).copied().ok_or_else(|| {
        IdentityObjectError::InvalidSegmentIndex {
            name: expected_name.to_owned(),
            index: definition.index,
        }
    })?;
    let start = usize::try_from(definition.offset).map_err(|_| {
        IdentityObjectError::InvalidSymbolOffset {
            name: expected_name.to_owned(),
            offset: definition.offset,
        }
    })?;
    let end = start
        .checked_add(width)
        .ok_or_else(|| IdentityObjectError::InvalidSymbolOffset {
            name: expected_name.to_owned(),
            offset: definition.offset,
        })?;
    segment
        .get(start..end)
        .ok_or_else(|| IdentityObjectError::TruncatedSymbol {
            name: expected_name.to_owned(),
            offset: definition.offset,
            size: width,
            segment_size: segment.len(),
        })
}

fn parse_error(context: &'static str, error: wasmparser::BinaryReaderError) -> IdentityObjectError {
    IdentityObjectError::Parse {
        context,
        message: error.to_string(),
    }
}
