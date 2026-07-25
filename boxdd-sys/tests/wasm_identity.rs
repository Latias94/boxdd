#[path = "../src/wasm_identity.rs"]
mod wasm_identity;

use wasm_identity::{IdentityObjectError, symbol_bytes};

#[derive(Clone, Copy)]
struct DataSymbol<'a> {
    name: &'a str,
    segment: u32,
    offset: u32,
    size: u32,
}

fn push_u32_leb(bytes: &mut Vec<u8>, mut value: u32) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        bytes.push(byte);
        if value == 0 {
            return;
        }
    }
}

fn push_bytes(bytes: &mut Vec<u8>, value: &[u8]) {
    push_u32_leb(bytes, value.len().try_into().expect("test fixture length"));
    bytes.extend_from_slice(value);
}

fn push_section(module: &mut Vec<u8>, id: u8, payload: &[u8]) {
    module.push(id);
    push_bytes(module, payload);
}

fn identity_object(data_segments: &[&[u8]], symbols: &[DataSymbol<'_>]) -> Vec<u8> {
    let mut module = b"\0asm\x01\0\0\0".to_vec();

    let mut data = Vec::new();
    push_u32_leb(
        &mut data,
        data_segments.len().try_into().expect("test segment count"),
    );
    for segment in data_segments {
        data.push(0); // Active segment in memory zero.
        data.extend_from_slice(&[0x41, 0x00, 0x0b]); // i32.const 0; end
        push_bytes(&mut data, segment);
    }
    push_section(&mut module, 11, &data);

    let mut symbol_table = Vec::new();
    push_u32_leb(
        &mut symbol_table,
        symbols.len().try_into().expect("test symbol count"),
    );
    for symbol in symbols {
        symbol_table.push(1); // WASM_SYMBOL_TYPE_DATA
        push_u32_leb(&mut symbol_table, 0); // Defined, global symbol.
        push_bytes(&mut symbol_table, symbol.name.as_bytes());
        push_u32_leb(&mut symbol_table, symbol.segment);
        push_u32_leb(&mut symbol_table, symbol.offset);
        push_u32_leb(&mut symbol_table, symbol.size);
    }

    let mut linking = Vec::new();
    push_bytes(&mut linking, b"linking");
    push_u32_leb(&mut linking, 2); // Linking metadata version.
    linking.push(8); // WASM_SYMBOL_TABLE subsection.
    push_bytes(&mut linking, &symbol_table);
    push_section(&mut module, 0, &linking);
    module
}

#[test]
fn reads_defined_data_symbol_from_its_segment() {
    let object = identity_object(
        &[&[1, 2, 3, 4], &[9, 8, 7, 6]],
        &[DataSymbol {
            name: "identity",
            segment: 1,
            offset: 1,
            size: 2,
        }],
    );

    assert_eq!(symbol_bytes(&object, "identity", 2), Ok(&[8, 7][..]));
}

#[test]
fn rejects_missing_identity_symbol() {
    let object = identity_object(&[&[1]], &[]);

    assert!(matches!(
        symbol_bytes(&object, "identity", 1),
        Err(IdentityObjectError::MissingSymbol { name }) if name == "identity"
    ));
}

#[test]
fn rejects_duplicate_identity_symbol() {
    let symbol = DataSymbol {
        name: "identity",
        segment: 0,
        offset: 0,
        size: 1,
    };
    let object = identity_object(&[&[1]], &[symbol, symbol]);

    assert!(matches!(
        symbol_bytes(&object, "identity", 1),
        Err(IdentityObjectError::DuplicateSymbol { name }) if name == "identity"
    ));
}

#[test]
fn rejects_incorrect_identity_symbol_size() {
    let object = identity_object(
        &[&[1, 2, 3, 4]],
        &[DataSymbol {
            name: "identity",
            segment: 0,
            offset: 0,
            size: 3,
        }],
    );

    assert!(matches!(
        symbol_bytes(&object, "identity", 4),
        Err(IdentityObjectError::InvalidSymbolSize {
            actual: 3,
            expected: 4,
            ..
        })
    ));
}

#[test]
fn rejects_missing_identity_segment() {
    let object = identity_object(
        &[&[1]],
        &[DataSymbol {
            name: "identity",
            segment: 7,
            offset: 0,
            size: 1,
        }],
    );

    assert!(matches!(
        symbol_bytes(&object, "identity", 1),
        Err(IdentityObjectError::InvalidSegmentIndex { index: 7, .. })
    ));
}

#[test]
fn rejects_identity_range_outside_segment() {
    let object = identity_object(
        &[&[1, 2, 3, 4]],
        &[DataSymbol {
            name: "identity",
            segment: 0,
            offset: 3,
            size: 2,
        }],
    );

    assert!(matches!(
        symbol_bytes(&object, "identity", 2),
        Err(IdentityObjectError::TruncatedSymbol {
            offset: 3,
            size: 2,
            segment_size: 4,
            ..
        })
    ));
}
