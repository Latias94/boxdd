use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

pub const ARGUMENT_TAGS: &[&str] = &[
    "AABB",
    "BODYDEF",
    "BODYID",
    "BOOL",
    "CAPSULE",
    "CHAINDEF",
    "CHAINID",
    "CHAINSEG",
    "CIRCLE",
    "DISTANCEJOINTDEF",
    "EXPLOSIONDEF",
    "F32",
    "FILTER",
    "FILTERJOINTDEF",
    "I32",
    "JOINTID",
    "LOCKS",
    "MASSDATA",
    "MATERIAL",
    "MOTORJOINTDEF",
    "POLYGON",
    "POSITION",
    "PRISMATICJOINTDEF",
    "QUERYFILTER",
    "REVOLUTEJOINTDEF",
    "ROT",
    "SEGMENT",
    "SHAPEDEF",
    "SHAPEID",
    "SHAPEPROXY",
    "STR",
    "U64",
    "VEC2",
    "WELDJOINTDEF",
    "WHEELJOINTDEF",
    "WORLDID",
    "WORLDXF",
    "XF",
];

pub const RETURN_TAGS: &[&str] = &[
    "RET_NONE",
    "RET_BODYID",
    "RET_SHAPEID",
    "RET_CHAINID",
    "RET_JOINTID",
];

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecordingArgument {
    pub tag: String,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecordingOp {
    pub opcode: u8,
    pub name: String,
    pub return_tag: String,
    pub arguments: Vec<RecordingArgument>,
}

pub fn parse(source: &str) -> Result<Vec<RecordingOp>> {
    let source = strip_comments(source)?;
    let mut operations = Vec::new();
    let mut rest = source.as_str();
    while !rest.trim().is_empty() {
        let trimmed = rest.trim_start();
        let Some(after_macro) = trimmed.strip_prefix("B2_REC_OP") else {
            let unexpected = trimmed.lines().next().unwrap_or(trimmed).trim();
            return Err(Error::message(format!(
                "unexpected tokens in recording manifest: `{unexpected}`"
            )));
        };
        rest = after_macro;
        let open = rest
            .find('(')
            .ok_or_else(|| Error::message("B2_REC_OP is missing `(`"))?;
        if !rest[..open].trim().is_empty() {
            return Err(Error::message("unexpected tokens after B2_REC_OP"));
        }
        let close = matching_parenthesis(rest, open)
            .ok_or_else(|| Error::message("unterminated B2_REC_OP invocation"))?;
        operations.push(parse_operation(&rest[open + 1..close])?);
        rest = &rest[close + 1..];
    }
    validate_operations(&operations)?;
    Ok(operations)
}

fn parse_operation(arguments: &str) -> Result<RecordingOp> {
    let fields = split_top_level(arguments, ',');
    if fields.len() < 3 {
        return Err(Error::message(format!(
            "B2_REC_OP expected opcode, name, and return tag: `{}`",
            arguments.trim()
        )));
    }
    let opcode_text = fields[0].trim();
    let opcode = if let Some(hex) = opcode_text.strip_prefix("0x") {
        u8::from_str_radix(hex.trim(), 16)
    } else {
        opcode_text.parse()
    }
    .map_err(|error| {
        Error::message(format!("invalid recording opcode `{opcode_text}`: {error}"))
    })?;
    let name = identifier(fields[1].trim(), "operation name")?;
    let return_tag = identifier(fields[2].trim(), "return tag")?;
    let tail_source = fields[3..].join(",");
    let arguments = parse_arguments(&tail_source)?;
    Ok(RecordingOp {
        opcode,
        name,
        return_tag,
        arguments,
    })
}

fn parse_arguments(source: &str) -> Result<Vec<RecordingArgument>> {
    let mut arguments = Vec::new();
    let mut rest = source;
    while !rest.trim().is_empty() {
        let trimmed = rest.trim_start();
        let Some(after_macro) = trimmed.strip_prefix("ARG") else {
            return Err(Error::message(format!(
                "unsupported recording argument syntax `{}`",
                trimmed.trim()
            )));
        };
        let open = after_macro
            .find('(')
            .ok_or_else(|| Error::message("ARG is missing `(`"))?;
        if !after_macro[..open].trim().is_empty() {
            return Err(Error::message("unexpected tokens after ARG"));
        }
        let close = matching_parenthesis(after_macro, open)
            .ok_or_else(|| Error::message("unterminated ARG invocation"))?;
        let fields = split_top_level(&after_macro[open + 1..close], ',');
        if fields.len() != 2 {
            return Err(Error::message("ARG requires exactly a tag and field name"));
        }
        arguments.push(RecordingArgument {
            tag: identifier(fields[0].trim(), "argument tag")?,
            name: identifier(fields[1].trim(), "argument name")?,
        });
        rest = &after_macro[close + 1..];
    }
    Ok(arguments)
}

pub fn validate_operations(operations: &[RecordingOp]) -> Result<()> {
    let allowed_arguments = ARGUMENT_TAGS.iter().copied().collect::<BTreeSet<_>>();
    let allowed_returns = RETURN_TAGS.iter().copied().collect::<BTreeSet<_>>();
    let mut opcodes = BTreeSet::new();
    let mut names = BTreeSet::new();
    let mut errors = Vec::new();
    if operations.is_empty() {
        errors.push("recording operation manifest is empty".to_owned());
    }
    for operation in operations {
        if !opcodes.insert(operation.opcode) {
            errors.push(format!(
                "duplicate recording opcode 0x{:02X}",
                operation.opcode
            ));
        }
        if !names.insert(operation.name.as_str()) {
            errors.push(format!(
                "duplicate recording operation `{}`",
                operation.name
            ));
        }
        if !allowed_returns.contains(operation.return_tag.as_str()) {
            errors.push(format!(
                "unknown return tag `{}` in `{}`",
                operation.return_tag, operation.name
            ));
        }
        let mut fields = BTreeSet::new();
        for argument in &operation.arguments {
            if !allowed_arguments.contains(argument.tag.as_str()) {
                errors.push(format!(
                    "unknown argument tag `{}` in `{}`",
                    argument.tag, operation.name
                ));
            }
            if !fields.insert(argument.name.as_str()) {
                errors.push(format!(
                    "duplicate argument `{}` in `{}`",
                    argument.name, operation.name
                ));
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::message(errors.join("\n")))
    }
}

fn identifier(value: &str, label: &str) -> Result<String> {
    let mut chars = value.chars();
    if chars
        .next()
        .is_some_and(|character| character == '_' || character.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
    {
        Ok(value.to_owned())
    } else {
        Err(Error::message(format!("invalid {label} `{value}`")))
    }
}

fn matching_parenthesis(source: &str, open: usize) -> Option<usize> {
    let mut depth = 0_i32;
    for (offset, character) in source[open..].char_indices() {
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + offset);
                }
            }
            _ => {}
        }
    }
    None
}

fn split_top_level(source: &str, separator: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0_i32;
    let mut start = 0;
    for (offset, character) in source.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth -= 1,
            value if value == separator && depth == 0 => {
                parts.push(&source[start..offset]);
                start = offset + value.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&source[start..]);
    parts
}

fn strip_comments(source: &str) -> Result<String> {
    let bytes = source.as_bytes();
    let mut output = String::with_capacity(source.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"//") {
            while index < bytes.len() && bytes[index] != b'\n' {
                output.push(' ');
                index += 1;
            }
        } else if bytes[index..].starts_with(b"/*") {
            output.push_str("  ");
            index += 2;
            let mut closed = false;
            while index + 1 < bytes.len() {
                if bytes[index..].starts_with(b"*/") {
                    output.push_str("  ");
                    index += 2;
                    closed = true;
                    break;
                }
                output.push(if bytes[index] == b'\n' { '\n' } else { ' ' });
                index += 1;
            }
            if !closed {
                return Err(Error::message(
                    "unterminated block comment in recording manifest",
                ));
            }
        } else {
            output.push(char::from(bytes[index]));
            index += 1;
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_operations() -> Vec<RecordingOp> {
        parse(
            r#"
                // B2_REC_OP(0x01, CommentOnly, RET_NONE)
                B2_REC_OP(0x10, CreateBody, RET_BODYID,
                    ARG(WORLDID, world) ARG(BODYDEF, def))
                B2_REC_OP(0x80, Step, RET_NONE,
                    ARG(WORLDID, world) ARG(F32, dt) ARG(I32, subSteps))
            "#,
        )
        .expect("fixture should parse")
    }

    #[test]
    fn parser_is_comment_and_multiline_safe() {
        let operations = sample_operations();
        assert_eq!(operations.len(), 2);
        assert_eq!(operations[0].opcode, 0x10);
        assert_eq!(operations[0].arguments[1].tag, "BODYDEF");
    }

    #[test]
    fn parser_rejects_duplicate_opcode_and_unknown_tag() {
        let duplicate = parse("B2_REC_OP(0x01, One, RET_NONE)\nB2_REC_OP(0x01, Two, RET_NONE)")
            .expect_err("duplicate opcode must fail");
        assert!(duplicate.to_string().contains("duplicate recording opcode"));
        let unknown = parse("B2_REC_OP(0x01, One, RET_NONE, ARG(POINTER, value))")
            .expect_err("unknown tags must fail");
        assert!(unknown.to_string().contains("unknown argument tag"));
    }

    #[test]
    fn parser_rejects_leading_interstitial_and_trailing_tokens() {
        for source in [
            "#define FORGED 1\nB2_REC_OP(0x01, One, RET_NONE)",
            "B2_REC_OP(0x01, One, RET_NONE)\nFORGED()\nB2_REC_OP(0x02, Two, RET_NONE)",
            "B2_REC_OP(0x01, One, RET_NONE)\nstatic int forged;",
            "B2_REC_UNKNOWN(0x01, One, RET_NONE)",
        ] {
            let error = parse(source).expect_err("residual tokens must fail closed");
            assert!(error.to_string().contains("unexpected tokens"));
        }
    }

    #[test]
    fn parser_rejects_an_empty_manifest_but_allows_comments_and_whitespace() {
        let error = parse("// no operations\n/* still empty */\n")
            .expect_err("empty manifests must fail closed");
        assert!(error.to_string().contains("manifest is empty"));

        let operations = parse("/* prefix */\n B2_REC_OP(0x01, One, RET_NONE) // suffix\n\n")
            .expect("comments and whitespace remain valid");
        assert_eq!(operations.len(), 1);
    }
}
