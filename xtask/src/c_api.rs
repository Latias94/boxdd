use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FunctionDecl {
    pub name: String,
    pub signature: String,
    pub fingerprint: String,
    pub parameters: Vec<String>,
    pub physical_symbols: BTreeMap<String, String>,
    pub availability: Vec<String>,
    pub header: String,
    pub line: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OverlayDecl {
    pub group: String,
    pub alternative: String,
    pub relative_path: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FieldDecl {
    pub name: String,
    pub signature: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub overlays: Vec<OverlayDecl>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructDecl {
    pub name: String,
    pub fingerprint: String,
    pub fields: Vec<FieldDecl>,
    pub header: String,
    pub line: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CallbackDecl {
    pub name: String,
    pub signature: String,
    pub fingerprint: String,
    pub header: String,
    pub line: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct CApiInventory {
    pub functions: Vec<FunctionDecl>,
    pub structs: Vec<StructDecl>,
    pub callbacks: Vec<CallbackDecl>,
}

/// Precision mode used while evaluating the public C header conditions.
///
/// This is intentionally local to the C API inventory. The manifest has its own precision
/// enum because the two domains have different serialization and validation responsibilities.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CAbiPrecision {
    #[default]
    Single,
    Double,
}

impl CAbiPrecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Double => "double",
        }
    }
}

/// Primitive ABI atoms shared by the C-header and generated-Rust inventories.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AbiPrimitive {
    Void,
    Bool,
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    Usize,
    Isize,
    F32,
    F64,
}

/// A normalized, recursive C ABI type shape.
///
/// Named aggregates remain references when reached through a pointer. By-value references are
/// expanded by the precision inventory resolver, which makes a change such as `b2Pos` from
/// `b2Vec2` to a two-double aggregate visible in every enclosing fingerprint.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum AbiTypeShape {
    Primitive {
        primitive: AbiPrimitive,
    },
    Named {
        name: String,
    },
    Pointer {
        mutable: bool,
        pointee: Box<Self>,
    },
    Array {
        element: Box<Self>,
        length: String,
    },
    Function {
        result: Box<Self>,
        parameters: Vec<Self>,
        variadic: bool,
    },
    Qualified {
        is_const: bool,
        is_volatile: bool,
        inner: Box<Self>,
    },
    Aggregate {
        fields: Vec<AbiFieldShape>,
    },
    RecursiveRef {
        name: String,
    },
}

impl AbiTypeShape {
    /// Return the stable recursive fingerprint used by C/Rust ABI comparisons.
    pub fn fingerprint(&self) -> String {
        let mut canonical = String::new();
        self.write_canonical(&mut canonical);
        fingerprint(&canonical)
    }

    fn write_canonical(&self, output: &mut String) {
        match self {
            Self::Primitive { primitive } => {
                push_fingerprint_component(output, "primitive");
                push_fingerprint_component(output, &format!("{primitive:?}"));
            }
            Self::Named { name } => {
                push_fingerprint_component(output, "named");
                push_fingerprint_component(output, name);
            }
            Self::Pointer { mutable, pointee } => {
                if matches!(pointee.as_ref(), Self::Function { .. }) {
                    pointee.write_canonical(output);
                    return;
                }
                push_fingerprint_component(output, "pointer");
                // C spells a const pointee as `const T *`, while bindgen spells the same
                // boundary as `*const T`. Normalize the two representations before hashing;
                // retain volatile qualifiers because Rust has no equivalent ABI contract.
                let (pointee_const, normalized_pointee) = match pointee.as_ref() {
                    Self::Qualified {
                        is_const: true,
                        is_volatile: false,
                        inner,
                    } => (true, inner.as_ref()),
                    _ => (false, pointee.as_ref()),
                };
                push_fingerprint_component(
                    output,
                    if !*mutable || pointee_const {
                        "const"
                    } else {
                        "mut"
                    },
                );
                normalized_pointee.write_canonical(output);
            }
            Self::Array { element, length } => {
                push_fingerprint_component(output, "array");
                push_fingerprint_component(output, length);
                element.write_canonical(output);
            }
            Self::Function {
                result,
                parameters,
                variadic,
            } => {
                push_fingerprint_component(output, "function");
                push_fingerprint_component(output, if *variadic { "variadic" } else { "fixed" });
                result.write_canonical(output);
                write!(output, "{}:", parameters.len()).expect("write to string");
                for parameter in parameters {
                    parameter.write_canonical(output);
                }
            }
            Self::Qualified {
                is_const,
                is_volatile,
                inner,
            } => {
                push_fingerprint_component(output, "qualified");
                push_fingerprint_component(output, if *is_const { "const" } else { "plain" });
                push_fingerprint_component(
                    output,
                    if *is_volatile {
                        "volatile"
                    } else {
                        "nonvolatile"
                    },
                );
                inner.write_canonical(output);
            }
            Self::Aggregate { fields } => {
                push_fingerprint_component(output, "aggregate");
                write!(output, "{}:", fields.len()).expect("write to string");
                for field in fields {
                    push_fingerprint_component(output, &field.name);
                    field.shape.write_canonical(output);
                    write!(output, "{}:", field.overlays.len()).expect("write to string");
                    for overlay in &field.overlays {
                        push_fingerprint_component(output, &overlay.group);
                        push_fingerprint_component(output, &overlay.alternative);
                        write!(output, "{}:", overlay.relative_path.len())
                            .expect("write to string");
                        for segment in &overlay.relative_path {
                            push_fingerprint_component(output, segment);
                        }
                    }
                }
            }
            Self::RecursiveRef { name } => {
                push_fingerprint_component(output, "recursive-ref");
                push_fingerprint_component(output, name);
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AbiFieldShape {
    pub name: String,
    pub shape: AbiTypeShape,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub overlays: Vec<OverlayDecl>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AbiAliasDecl {
    pub name: String,
    pub target: AbiTypeShape,
    pub fingerprint: String,
    pub header: String,
    pub line: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AbiStructShape {
    pub name: String,
    pub fields: Vec<AbiFieldShape>,
    pub fingerprint: String,
    pub header: String,
    pub line: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AbiEnumShape {
    pub name: String,
    pub underlying: AbiPrimitive,
    pub fingerprint: String,
    pub header: String,
    pub line: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AbiOpaqueShape {
    pub name: String,
    pub fingerprint: String,
    pub header: String,
    pub line: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AbiCallableShape {
    pub name: String,
    pub shape: AbiTypeShape,
    pub fingerprint: String,
    pub signature: String,
    pub header: String,
    pub line: usize,
}

/// Effective declarations selected for one precision mode.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct PrecisionCApiInventory {
    pub precision: CAbiPrecision,
    pub aliases: Vec<AbiAliasDecl>,
    pub structs: Vec<AbiStructShape>,
    pub enums: Vec<AbiEnumShape>,
    pub opaques: Vec<AbiOpaqueShape>,
    pub functions: Vec<AbiCallableShape>,
    pub callbacks: Vec<AbiCallableShape>,
    pub effective_types: BTreeMap<String, AbiTypeShape>,
}

impl PrecisionCApiInventory {
    pub fn alias(&self, name: &str) -> Option<&AbiAliasDecl> {
        self.aliases
            .iter()
            .find(|declaration| declaration.name == name)
    }

    pub fn structure(&self, name: &str) -> Option<&AbiStructShape> {
        self.structs
            .iter()
            .find(|declaration| declaration.name == name)
    }

    pub fn function(&self, name: &str) -> Option<&AbiCallableShape> {
        self.functions
            .iter()
            .find(|declaration| declaration.name == name)
    }

    pub fn callback(&self, name: &str) -> Option<&AbiCallableShape> {
        self.callbacks
            .iter()
            .find(|declaration| declaration.name == name)
    }

    pub fn type_shape(&self, name: &str) -> Option<&AbiTypeShape> {
        self.effective_types.get(name)
    }

    pub fn type_fingerprint(&self, name: &str) -> Option<String> {
        self.type_shape(name).map(AbiTypeShape::fingerprint)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Token {
    text: String,
    line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum PreprocessorCondition {
    DoublePrecision,
    SinglePrecision,
    DebugOrAssertions,
    Other(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AliasScope {
    All,
    Single,
    Double,
    Unsupported,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PrecisionAlias {
    logical: String,
    physical: String,
    scope: AliasScope,
    line: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct PreprocessorMetadata {
    conditions_by_line: BTreeMap<usize, Vec<PreprocessorCondition>>,
    precision_aliases: Vec<PrecisionAlias>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct RawPrecisionInventory {
    aliases: BTreeMap<String, RawAliasDecl>,
    structs: BTreeMap<String, RawStructDecl>,
    enums: BTreeMap<String, RawEnumDecl>,
    opaques: BTreeMap<String, RawOpaqueDecl>,
    functions: BTreeMap<String, RawCallableDecl>,
    callbacks: BTreeMap<String, RawCallableDecl>,
    integer_constants: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RawAliasDecl {
    name: String,
    target: AbiTypeShape,
    header: String,
    line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RawStructDecl {
    name: String,
    fields: Vec<AbiFieldShape>,
    header: String,
    line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RawEnumDecl {
    name: String,
    underlying: AbiPrimitive,
    header: String,
    line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RawOpaqueDecl {
    name: String,
    header: String,
    line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RawCallableDecl {
    name: String,
    shape: AbiTypeShape,
    signature: String,
    header: String,
    line: usize,
}

/// Parse all public C headers in one effective precision mode.
pub fn parse_headers_for_precision(
    include_dir: &Path,
    precision: CAbiPrecision,
) -> Result<PrecisionCApiInventory> {
    let headers = header_paths(include_dir)?;
    if headers.is_empty() {
        return Err(Error::message(format!(
            "no C headers found under {}",
            include_dir.display()
        )));
    }

    let mut raw = RawPrecisionInventory::default();
    for header in headers {
        let source = fs::read_to_string(&header).map_err(|error| Error::io(&header, error))?;
        let relative = header
            .strip_prefix(include_dir)
            .unwrap_or(&header)
            .to_string_lossy()
            .replace('\\', "/");
        parse_precision_header(&source, &relative, precision, &mut raw)?;
    }
    resolve_precision_inventory(precision, raw)
}

fn parse_precision_header(
    source: &str,
    header: &str,
    precision: CAbiPrecision,
    raw: &mut RawPrecisionInventory,
) -> Result<()> {
    record_integer_macros(source, header, raw)?;
    let metadata = parse_preprocessor(source, header)?;
    let tokens = effective_precision_tokens(source, &metadata, precision)?;
    parse_precision_structs(&tokens, header, raw)?;
    parse_precision_typedefs(&tokens, header, raw)?;
    record_tag_references(&tokens, header, raw);
    parse_precision_functions(&tokens, header, raw)?;
    Ok(())
}

fn record_integer_macros(
    source: &str,
    header: &str,
    raw: &mut RawPrecisionInventory,
) -> Result<()> {
    for (line_index, line) in source.lines().enumerate() {
        let Some(directive) = line.trim_start().strip_prefix("#define") else {
            continue;
        };
        let mut parts = directive.split_whitespace();
        let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
            continue;
        };
        if !is_identifier(name) || name.contains('(') {
            continue;
        }
        let Some(value) = normalize_c_integer_literal(value) else {
            continue;
        };
        if let Some(previous) = raw.integer_constants.insert(name.to_owned(), value.clone())
            && previous != value
        {
            return Err(Error::message(format!(
                "{header}:{}: integer macro `{name}` conflicts with the earlier value `{previous}`",
                line_index + 1
            )));
        }
    }
    Ok(())
}

fn normalize_c_integer_literal(value: &str) -> Option<String> {
    let mut value = value.trim();
    while value.starts_with('(') && value.ends_with(')') && value.len() > 2 {
        value = value[1..value.len() - 1].trim();
    }
    value = value.trim_end_matches(['u', 'U', 'l', 'L']);
    let (radix, digits) = if let Some(digits) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        (16, digits)
    } else if let Some(digits) = value
        .strip_prefix("0b")
        .or_else(|| value.strip_prefix("0B"))
    {
        (2, digits)
    } else {
        (10, value)
    };
    (!digits.is_empty())
        .then(|| {
            u128::from_str_radix(digits, radix)
                .ok()
                .map(|value| value.to_string())
        })
        .flatten()
}

fn record_tag_references(tokens: &[Token], header: &str, raw: &mut RawPrecisionInventory) {
    for pair in tokens.windows(2) {
        if !matches!(pair[0].text.as_str(), "struct" | "union")
            || !pair[1].text.starts_with("b2")
            || !is_identifier(&pair[1].text)
        {
            continue;
        }
        raw.opaques
            .entry(pair[1].text.clone())
            .or_insert_with(|| RawOpaqueDecl {
                name: pair[1].text.clone(),
                header: header.to_owned(),
                line: pair[1].line,
            });
    }
}

fn effective_precision_tokens(
    source: &str,
    metadata: &PreprocessorMetadata,
    precision: CAbiPrecision,
) -> Result<Vec<Token>> {
    let tokens = tokenize(source)?
        .into_iter()
        .filter(|token| {
            metadata
                .conditions_by_line
                .get(&token.line)
                .is_none_or(|conditions| precision_conditions_active(conditions, precision))
        })
        .collect::<Vec<_>>();
    Ok(tokens)
}

fn precision_conditions_active(
    conditions: &[PreprocessorCondition],
    precision: CAbiPrecision,
) -> bool {
    conditions.iter().all(|condition| match condition {
        PreprocessorCondition::DoublePrecision => precision == CAbiPrecision::Double,
        PreprocessorCondition::SinglePrecision => precision == CAbiPrecision::Single,
        PreprocessorCondition::DebugOrAssertions | PreprocessorCondition::Other(_) => true,
    })
}

fn parse_precision_structs(
    tokens: &[Token],
    header: &str,
    raw: &mut RawPrecisionInventory,
) -> Result<()> {
    let mut structs = BTreeMap::new();
    parse_structs(tokens, header, &mut structs)?;
    for declaration in structs.into_values() {
        let fields = declaration
            .fields
            .iter()
            .map(|field| {
                let shape =
                    parse_abi_field_signature(&field.signature, &field.name).map_err(|error| {
                        Error::message(format!(
                            "{header}:{}: failed to parse effective field `{}`: {error}",
                            declaration.line, field.name
                        ))
                    })?;
                Ok(AbiFieldShape {
                    name: field.name.clone(),
                    shape,
                    overlays: field.overlays.clone(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        insert_unique(
            &mut raw.structs,
            declaration.name.clone(),
            RawStructDecl {
                name: declaration.name,
                fields,
                header: declaration.header,
                line: declaration.line,
            },
            header,
            declaration.line,
        )?;
    }
    Ok(())
}

fn parse_precision_typedefs(
    tokens: &[Token],
    header: &str,
    raw: &mut RawPrecisionInventory,
) -> Result<()> {
    for (start, marker) in tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.text == "typedef")
    {
        let Some(end) = declaration_end(tokens, start) else {
            return Err(Error::message(format!(
                "{header}:{}: unterminated typedef declaration",
                marker.line
            )));
        };
        let declaration = &tokens[start + 1..end];
        if declaration.is_empty() {
            continue;
        }

        if declaration[0].text == "struct" && declaration.iter().any(|token| token.text == "{") {
            continue;
        }
        if declaration[0].text == "enum" && declaration.iter().any(|token| token.text == "{") {
            let (name, underlying) = parse_enum_typedef(declaration, header, marker.line)?;
            insert_unique(
                &mut raw.enums,
                name.clone(),
                RawEnumDecl {
                    name,
                    underlying,
                    header: header.to_owned(),
                    line: marker.line,
                },
                header,
                marker.line,
            )?;
            continue;
        }

        let Some((name, shape)) = parse_abi_declaration(declaration, header, marker.line)? else {
            continue;
        };
        if !name.starts_with("b2") {
            continue;
        }
        if matches!(
            declaration.first().map(|token| token.text.as_str()),
            Some("struct" | "union")
        ) {
            insert_unique(
                &mut raw.opaques,
                name.clone(),
                RawOpaqueDecl {
                    name,
                    header: header.to_owned(),
                    line: marker.line,
                },
                header,
                marker.line,
            )?;
            continue;
        }
        if matches!(shape, AbiTypeShape::Function { .. }) {
            insert_unique(
                &mut raw.callbacks,
                name.clone(),
                RawCallableDecl {
                    name,
                    shape,
                    signature: canonical(declaration),
                    header: header.to_owned(),
                    line: marker.line,
                },
                header,
                marker.line,
            )?;
        } else {
            insert_unique(
                &mut raw.aliases,
                name.clone(),
                RawAliasDecl {
                    name,
                    target: shape,
                    header: header.to_owned(),
                    line: marker.line,
                },
                header,
                marker.line,
            )?;
        }
    }
    Ok(())
}

fn parse_precision_functions(
    tokens: &[Token],
    header: &str,
    raw: &mut RawPrecisionInventory,
) -> Result<()> {
    for (start, marker) in tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.text == "B2_API")
    {
        let Some(end) = declaration_end(tokens, start) else {
            return Err(Error::message(format!(
                "{header}:{}: unterminated B2_API declaration",
                marker.line
            )));
        };
        let declaration = &tokens[start + 1..end];
        let Some((name, shape)) = parse_abi_declaration(declaration, header, marker.line)? else {
            return Err(Error::message(format!(
                "{header}:{}: B2_API declaration has no ABI type",
                marker.line
            )));
        };
        if !matches!(shape, AbiTypeShape::Function { .. }) {
            return Err(Error::message(format!(
                "{header}:{}: B2_API `{name}` is not a function",
                marker.line
            )));
        }
        insert_unique(
            &mut raw.functions,
            name.clone(),
            RawCallableDecl {
                name,
                shape,
                signature: canonical(declaration),
                header: header.to_owned(),
                line: marker.line,
            },
            header,
            marker.line,
        )?;
    }
    Ok(())
}

fn parse_enum_typedef(
    declaration: &[Token],
    header: &str,
    line: usize,
) -> Result<(String, AbiPrimitive)> {
    let open = declaration
        .iter()
        .position(|token| token.text == "{")
        .ok_or_else(|| Error::message(format!("{header}:{line}: enum body is missing")))?;
    let close = matching(declaration, open, "{", "}")
        .ok_or_else(|| Error::message(format!("{header}:{line}: enum body is unterminated")))?;
    let name = declaration[close + 1..]
        .iter()
        .rev()
        .find(|token| is_identifier(&token.text) && token.text.starts_with("b2"))
        .map(|token| token.text.clone())
        .ok_or_else(|| Error::message(format!("{header}:{line}: enum typedef name is missing")))?;
    let underlying = if declaration[open + 1..close]
        .iter()
        .any(|token| token.text == "-")
    {
        AbiPrimitive::I32
    } else {
        AbiPrimitive::U32
    };
    Ok((name, underlying))
}

fn parse_abi_field_signature(signature: &str, field_name: &str) -> Result<AbiTypeShape> {
    let tokens = tokenize(signature)?;
    let name = field_name
        .rsplit('.')
        .next()
        .filter(|name| is_identifier(name))
        .unwrap_or(field_name);
    parse_abi_declaration_with_name(&tokens, Some(name), "field", 0)
        .map(|result| result.map(|(_, shape)| shape))
        .and_then(|shape| shape.ok_or_else(|| Error::message("field declaration has no type")))
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum AbiDerivedDeclarator {
    Name(String),
    Pointer {
        mutable: bool,
        inner: Box<Self>,
    },
    Array {
        inner: Box<Self>,
        length: String,
    },
    Function {
        inner: Box<Self>,
        parameters: Vec<Vec<Token>>,
        variadic: bool,
    },
}

fn parse_abi_declaration(
    tokens: &[Token],
    header: &str,
    line: usize,
) -> Result<Option<(String, AbiTypeShape)>> {
    parse_abi_declaration_with_name(tokens, None, header, line)
}

fn parse_abi_declaration_with_name(
    tokens: &[Token],
    expected_name: Option<&str>,
    header: &str,
    line: usize,
) -> Result<Option<(String, AbiTypeShape)>> {
    if tokens.is_empty() {
        return Ok(None);
    }

    for split in 1..=tokens.len() {
        let base_tokens = &tokens[..split];
        if !valid_declaration_specifiers(base_tokens) {
            continue;
        }
        let mut declarator_tokens = tokens[split..].to_vec();
        if declarator_tokens.is_empty() {
            let base = abi_base_shape(base_tokens, header, line)?;
            return Ok(expected_name.map(|name| (name.to_owned(), base)));
        }
        if expected_name.is_none()
            && !declarator_tokens
                .iter()
                .any(|token| is_identifier(&token.text) && !is_c_keyword(&token.text))
        {
            declarator_tokens.push(Token {
                text: "__boxdd_unnamed".to_owned(),
                line,
            });
        }
        let Ok((declarator, consumed)) = parse_abi_derived(&declarator_tokens, 0) else {
            continue;
        };
        if consumed != declarator_tokens.len() {
            continue;
        }
        let Some(name) = abi_declarator_name(&declarator) else {
            continue;
        };
        if expected_name.is_some_and(|expected| expected != name) {
            continue;
        }
        let base = abi_base_shape(base_tokens, header, line)?;
        let shape = apply_abi_declarator(&declarator, base, header, line)?;
        return Ok(Some((name.to_owned(), shape)));
    }

    // An abstract declaration such as `void *` has no identifier at all. Add a synthetic
    // declarator so the same grammar can normalize it without guessing from the spelling.
    if expected_name.is_none() {
        let mut abstract_tokens = tokens.to_vec();
        abstract_tokens.push(Token {
            text: "__boxdd_unnamed".to_owned(),
            line,
        });
        if let Some((_, shape)) = parse_abi_declaration_with_name(
            &abstract_tokens,
            Some("__boxdd_unnamed"),
            header,
            line,
        )? {
            return Ok(Some(("__boxdd_unnamed".to_owned(), shape)));
        }
        if let Ok(base) = abi_base_shape(tokens, header, line) {
            return Ok(Some(("__boxdd_unnamed".to_owned(), base)));
        }
    }
    Ok(None)
}

fn parse_abi_derived(tokens: &[Token], mut index: usize) -> Result<(AbiDerivedDeclarator, usize)> {
    let mut pointers = Vec::new();
    while tokens.get(index).is_some_and(|token| token.text == "*") {
        index += 1;
        let mut is_const = false;
        let mut is_volatile = false;
        while let Some(token) = tokens.get(index) {
            match token.text.as_str() {
                "const" => is_const = true,
                "volatile" => is_volatile = true,
                "restrict" | "__restrict" | "__restrict__" => {}
                _ => break,
            }
            index += 1;
        }
        pointers.push((is_const, is_volatile));
    }

    let (mut declarator, mut index) = match tokens.get(index).map(|token| token.text.as_str()) {
        Some(name) if is_identifier(name) => {
            (AbiDerivedDeclarator::Name(name.to_owned()), index + 1)
        }
        Some("(") => {
            let close = matching(tokens, index, "(", ")")
                .ok_or_else(|| Error::message("unterminated C declarator group"))?;
            let (inner, consumed) = parse_abi_derived(&tokens[index + 1..close], 0)?;
            if consumed != close - index - 1 {
                return Err(Error::message("C declarator group has trailing tokens"));
            }
            (inner, close + 1)
        }
        _ => return Err(Error::message("C declarator name is missing")),
    };

    loop {
        match tokens.get(index).map(|token| token.text.as_str()) {
            Some("[") => {
                let close = matching(tokens, index, "[", "]")
                    .ok_or_else(|| Error::message("unterminated C array declarator"))?;
                declarator = AbiDerivedDeclarator::Array {
                    inner: Box::new(declarator),
                    length: canonical(&tokens[index + 1..close]),
                };
                index = close + 1;
            }
            Some("(") => {
                let close = matching(tokens, index, "(", ")")
                    .ok_or_else(|| Error::message("unterminated C function declarator"))?;
                let parts = split_top_level(&tokens[index + 1..close], ",");
                let variadic = parts
                    .iter()
                    .any(|part| part.iter().any(|token| token.text == "..."));
                let parameters = parts
                    .into_iter()
                    .filter(|part| !part.is_empty() && canonical(part) != "void")
                    .map(|part| part.to_vec())
                    .collect();
                declarator = AbiDerivedDeclarator::Function {
                    inner: Box::new(declarator),
                    parameters,
                    variadic,
                };
                index = close + 1;
            }
            _ => break,
        }
    }

    for (is_const, is_volatile) in pointers.into_iter().rev() {
        declarator = AbiDerivedDeclarator::Pointer {
            mutable: !is_const,
            inner: Box::new(declarator),
        };
        // Volatile pointer qualification does not change the C ABI, but retaining it in the
        // normalized shape would make equivalent bindgen declarations compare differently.
        let _ = is_volatile;
    }
    Ok((declarator, index))
}

fn abi_declarator_name(declarator: &AbiDerivedDeclarator) -> Option<&str> {
    match declarator {
        AbiDerivedDeclarator::Name(name) => Some(name),
        AbiDerivedDeclarator::Pointer { inner, .. }
        | AbiDerivedDeclarator::Array { inner, .. }
        | AbiDerivedDeclarator::Function { inner, .. } => abi_declarator_name(inner),
    }
}

fn apply_abi_declarator(
    declarator: &AbiDerivedDeclarator,
    base: AbiTypeShape,
    header: &str,
    line: usize,
) -> Result<AbiTypeShape> {
    match declarator {
        AbiDerivedDeclarator::Name(_) => Ok(base),
        AbiDerivedDeclarator::Pointer { mutable, inner } => apply_abi_declarator(
            inner,
            AbiTypeShape::Pointer {
                mutable: *mutable,
                pointee: Box::new(base),
            },
            header,
            line,
        ),
        AbiDerivedDeclarator::Array { inner, length } => apply_abi_declarator(
            inner,
            AbiTypeShape::Array {
                element: Box::new(base),
                length: length.clone(),
            },
            header,
            line,
        ),
        AbiDerivedDeclarator::Function {
            inner,
            parameters,
            variadic,
        } => {
            let parameters = parameters
                .iter()
                .map(|parameter| parse_abi_parameter(parameter, header, line))
                .collect::<Result<Vec<_>>>()?;
            apply_abi_declarator(
                inner,
                AbiTypeShape::Function {
                    result: Box::new(base),
                    parameters,
                    variadic: *variadic,
                },
                header,
                line,
            )
        }
    }
}

fn parse_abi_parameter(tokens: &[Token], header: &str, line: usize) -> Result<AbiTypeShape> {
    if tokens.is_empty() || canonical(tokens) == "void" {
        return Ok(AbiTypeShape::Primitive {
            primitive: AbiPrimitive::Void,
        });
    }
    let (_, shape) = parse_abi_declaration_with_name(tokens, None, header, line)?
        .ok_or_else(|| Error::message(format!("{header}:{line}: unsupported ABI parameter")))?;
    // C array/function parameters decay to pointers at the call boundary.
    Ok(decay_parameter_shape(shape))
}

fn decay_parameter_shape(shape: AbiTypeShape) -> AbiTypeShape {
    match shape {
        AbiTypeShape::Array { element, .. } => AbiTypeShape::Pointer {
            mutable: true,
            pointee: element,
        },
        AbiTypeShape::Function { .. } => AbiTypeShape::Pointer {
            mutable: true,
            pointee: Box::new(shape),
        },
        shape => shape,
    }
}

fn abi_base_shape(tokens: &[Token], header: &str, line: usize) -> Result<AbiTypeShape> {
    let mut specifiers = Vec::new();
    let mut is_const = false;
    let mut is_volatile = false;
    let mut index = 0;
    while index < tokens.len() {
        let token = &tokens[index].text;
        match token.as_str() {
            "const" => is_const = true,
            "volatile" => is_volatile = true,
            "restrict" | "__restrict" | "__restrict__" => {}
            "struct" | "union" | "enum" => specifiers.push(token.clone()),
            name if is_identifier(name)
                && tokens.get(index + 1).is_some_and(|next| next.text == "(") =>
            {
                let close = matching(tokens, index + 1, "(", ")").ok_or_else(|| {
                    Error::message(format!("{header}:{line}: unterminated C type attribute"))
                })?;
                index = close;
            }
            _ => specifiers.push(token.clone()),
        }
        index += 1;
    }
    if specifiers.is_empty() {
        return Err(Error::message(format!(
            "{header}:{line}: C ABI base type is missing"
        )));
    }
    let canonical_specifiers = specifiers.join(" ");
    let primitive = match canonical_specifiers.as_str() {
        "void" => Some(AbiPrimitive::Void),
        "bool" | "_Bool" => Some(AbiPrimitive::Bool),
        "char" | "signed char" | "int8_t" | "int8" => Some(AbiPrimitive::I8),
        "unsigned char" | "uint8_t" | "uint8" => Some(AbiPrimitive::U8),
        "short" | "short int" | "signed short" | "signed short int" | "int16_t" => {
            Some(AbiPrimitive::I16)
        }
        "unsigned short" | "unsigned short int" | "uint16_t" => Some(AbiPrimitive::U16),
        "int" | "signed" | "signed int" | "int32_t" => Some(AbiPrimitive::I32),
        "unsigned" | "unsigned int" | "uint32_t" => Some(AbiPrimitive::U32),
        "long long" | "long long int" | "signed long long" | "int64_t" => Some(AbiPrimitive::I64),
        "unsigned long long" | "unsigned long long int" | "uint64_t" => Some(AbiPrimitive::U64),
        "size_t" => Some(AbiPrimitive::Usize),
        "ptrdiff_t" => Some(AbiPrimitive::Isize),
        "float" => Some(AbiPrimitive::F32),
        "double" => Some(AbiPrimitive::F64),
        _ => None,
    };
    let mut shape = if let Some(primitive) = primitive {
        AbiTypeShape::Primitive { primitive }
    } else if specifiers
        .first()
        .is_some_and(|specifier| matches!(specifier.as_str(), "struct" | "union" | "enum"))
    {
        let name = specifiers
            .iter()
            .skip(1)
            .find(|specifier| is_identifier(specifier) && specifier.starts_with("b2"))
            .cloned()
            .ok_or_else(|| {
                Error::message(format!("{header}:{line}: tagged C type name is missing"))
            })?;
        AbiTypeShape::Named { name }
    } else if specifiers.len() == 1 && is_identifier(&specifiers[0]) {
        AbiTypeShape::Named {
            name: specifiers[0].clone(),
        }
    } else {
        return Err(Error::message(format!(
            "{header}:{line}: unsupported C ABI base type `{canonical_specifiers}`"
        )));
    };
    if is_const || is_volatile {
        shape = AbiTypeShape::Qualified {
            is_const,
            is_volatile,
            inner: Box::new(shape),
        };
    }
    Ok(shape)
}

fn is_c_keyword(value: &str) -> bool {
    matches!(
        value,
        "const"
            | "volatile"
            | "restrict"
            | "static"
            | "inline"
            | "extern"
            | "struct"
            | "union"
            | "enum"
            | "void"
            | "bool"
            | "char"
            | "short"
            | "int"
            | "long"
            | "float"
            | "double"
            | "signed"
            | "unsigned"
            | "size_t"
    )
}

fn resolve_precision_inventory(
    precision: CAbiPrecision,
    raw: RawPrecisionInventory,
) -> Result<PrecisionCApiInventory> {
    let mut resolver = AbiShapeResolver::new(&raw);
    let mut effective_types = BTreeMap::new();
    for name in raw
        .aliases
        .keys()
        .chain(raw.structs.keys())
        .chain(raw.enums.keys())
        .chain(raw.opaques.keys())
        .chain(raw.callbacks.keys())
    {
        effective_types.insert(name.clone(), resolver.resolve_named(name, true)?);
    }

    let aliases = raw
        .aliases
        .values()
        .map(|declaration| {
            let target = effective_types
                .get(&declaration.name)
                .cloned()
                .ok_or_else(|| Error::message("resolved alias is missing"))?;
            Ok(AbiAliasDecl {
                name: declaration.name.clone(),
                fingerprint: target.fingerprint(),
                target,
                header: declaration.header.clone(),
                line: declaration.line,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let structs = raw
        .structs
        .values()
        .map(|declaration| {
            let shape = effective_types
                .get(&declaration.name)
                .cloned()
                .ok_or_else(|| Error::message("resolved struct is missing"))?;
            let AbiTypeShape::Aggregate { fields } = shape else {
                return Err(Error::message(format!(
                    "resolved struct `{}` is not an aggregate",
                    declaration.name
                )));
            };
            let fingerprint = AbiTypeShape::Aggregate {
                fields: fields.clone(),
            }
            .fingerprint();
            Ok(AbiStructShape {
                name: declaration.name.clone(),
                fields,
                fingerprint,
                header: declaration.header.clone(),
                line: declaration.line,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let enums = raw
        .enums
        .values()
        .map(|declaration| {
            let shape = AbiTypeShape::Primitive {
                primitive: declaration.underlying,
            };
            AbiEnumShape {
                name: declaration.name.clone(),
                underlying: declaration.underlying,
                fingerprint: shape.fingerprint(),
                header: declaration.header.clone(),
                line: declaration.line,
            }
        })
        .collect();
    let opaques = raw
        .opaques
        .values()
        .filter(|declaration| {
            !raw.aliases.contains_key(&declaration.name)
                && !raw.structs.contains_key(&declaration.name)
                && !raw.enums.contains_key(&declaration.name)
                && !raw.callbacks.contains_key(&declaration.name)
        })
        .map(|declaration| {
            let shape = AbiTypeShape::Named {
                name: declaration.name.clone(),
            };
            AbiOpaqueShape {
                name: declaration.name.clone(),
                fingerprint: shape.fingerprint(),
                header: declaration.header.clone(),
                line: declaration.line,
            }
        })
        .collect();
    let functions = resolve_callable_declarations(&raw.functions, &mut resolver)?;
    let callbacks = resolve_callable_declarations(&raw.callbacks, &mut resolver)?;

    Ok(PrecisionCApiInventory {
        precision,
        aliases,
        structs,
        enums,
        opaques,
        functions,
        callbacks,
        effective_types,
    })
}

fn resolve_callable_declarations(
    declarations: &BTreeMap<String, RawCallableDecl>,
    resolver: &mut AbiShapeResolver<'_>,
) -> Result<Vec<AbiCallableShape>> {
    declarations
        .values()
        .map(|declaration| {
            let shape = resolver.resolve_shape(&declaration.shape, true)?;
            Ok(AbiCallableShape {
                name: declaration.name.clone(),
                fingerprint: shape.fingerprint(),
                shape,
                signature: declaration.signature.clone(),
                header: declaration.header.clone(),
                line: declaration.line,
            })
        })
        .collect()
}

struct AbiShapeResolver<'a> {
    raw: &'a RawPrecisionInventory,
    alias_stack: Vec<String>,
    aggregate_stack: Vec<String>,
}

impl<'a> AbiShapeResolver<'a> {
    fn new(raw: &'a RawPrecisionInventory) -> Self {
        Self {
            raw,
            alias_stack: Vec::new(),
            aggregate_stack: Vec::new(),
        }
    }

    fn resolve_shape(&mut self, shape: &AbiTypeShape, by_value: bool) -> Result<AbiTypeShape> {
        match shape {
            AbiTypeShape::Primitive { primitive } => Ok(AbiTypeShape::Primitive {
                primitive: *primitive,
            }),
            AbiTypeShape::Named { name } => self.resolve_named(name, by_value),
            AbiTypeShape::Pointer { mutable, pointee } => Ok(AbiTypeShape::Pointer {
                mutable: *mutable,
                pointee: Box::new(self.resolve_pointee(pointee)?),
            }),
            AbiTypeShape::Array { element, length } => Ok(AbiTypeShape::Array {
                element: Box::new(self.resolve_shape(element, true)?),
                length: self
                    .raw
                    .integer_constants
                    .get(length)
                    .cloned()
                    .unwrap_or_else(|| length.clone()),
            }),
            AbiTypeShape::Function {
                result,
                parameters,
                variadic,
            } => Ok(AbiTypeShape::Function {
                result: Box::new(self.resolve_shape(result, true)?),
                parameters: parameters
                    .iter()
                    .map(|parameter| self.resolve_shape(parameter, true))
                    .collect::<Result<Vec<_>>>()?,
                variadic: *variadic,
            }),
            AbiTypeShape::Qualified {
                is_const,
                is_volatile,
                inner,
            } => Ok(AbiTypeShape::Qualified {
                is_const: *is_const,
                is_volatile: *is_volatile,
                inner: Box::new(self.resolve_shape(inner, by_value)?),
            }),
            AbiTypeShape::Aggregate { fields } => Ok(AbiTypeShape::Aggregate {
                fields: fields
                    .iter()
                    .map(|field| {
                        Ok(AbiFieldShape {
                            name: field.name.clone(),
                            shape: self.resolve_shape(&field.shape, true)?,
                            overlays: field.overlays.clone(),
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            }),
            AbiTypeShape::RecursiveRef { name } => {
                Ok(AbiTypeShape::RecursiveRef { name: name.clone() })
            }
        }
    }

    fn resolve_pointee(&mut self, pointee: &AbiTypeShape) -> Result<AbiTypeShape> {
        match pointee {
            AbiTypeShape::Named { name } if self.raw.callbacks.contains_key(name) => {
                let shape = self.raw.callbacks[name].shape.clone();
                self.resolve_shape(&shape, true)
            }
            AbiTypeShape::Qualified {
                is_const,
                is_volatile,
                inner,
            } => Ok(AbiTypeShape::Qualified {
                is_const: *is_const,
                is_volatile: *is_volatile,
                inner: Box::new(self.resolve_pointee(inner)?),
            }),
            AbiTypeShape::Primitive { .. }
            | AbiTypeShape::Function { .. }
            | AbiTypeShape::Pointer { .. }
            | AbiTypeShape::Array { .. }
            | AbiTypeShape::Aggregate { .. }
            | AbiTypeShape::RecursiveRef { .. } => self.resolve_shape(pointee, false),
            AbiTypeShape::Named { name } => {
                self.ensure_known_named_type(name)?;
                Ok(AbiTypeShape::Named { name: name.clone() })
            }
        }
    }

    fn resolve_named(&mut self, name: &str, by_value: bool) -> Result<AbiTypeShape> {
        if let Some(alias) = self.raw.aliases.get(name) {
            if self.alias_stack.iter().any(|active| active == name) {
                let mut cycle = self.alias_stack.clone();
                cycle.push(name.to_owned());
                return Err(Error::message(format!(
                    "C ABI type alias cycle: {}",
                    cycle.join(" -> ")
                )));
            }
            if !by_value {
                return Ok(AbiTypeShape::Named {
                    name: name.to_owned(),
                });
            }
            let target = alias.target.clone();
            self.alias_stack.push(name.to_owned());
            let resolved = self.resolve_shape(&target, true);
            self.alias_stack.pop();
            return resolved;
        }
        if let Some(structure) = self.raw.structs.get(name) {
            if !by_value {
                return Ok(AbiTypeShape::Named {
                    name: name.to_owned(),
                });
            }
            if self.aggregate_stack.iter().any(|active| active == name) {
                return Ok(AbiTypeShape::RecursiveRef {
                    name: name.to_owned(),
                });
            }
            let fields = structure.fields.clone();
            self.aggregate_stack.push(name.to_owned());
            let resolved = fields
                .iter()
                .map(|field| {
                    Ok(AbiFieldShape {
                        name: field.name.clone(),
                        shape: self.resolve_shape(&field.shape, true)?,
                        overlays: field.overlays.clone(),
                    })
                })
                .collect::<Result<Vec<_>>>();
            self.aggregate_stack.pop();
            return resolved.map(|fields| AbiTypeShape::Aggregate { fields });
        }
        if let Some(enumeration) = self.raw.enums.get(name) {
            return Ok(AbiTypeShape::Primitive {
                primitive: enumeration.underlying,
            });
        }
        if self.raw.opaques.contains_key(name) {
            return Ok(AbiTypeShape::Named {
                name: name.to_owned(),
            });
        }
        if let Some(callback) = self.raw.callbacks.get(name) {
            let shape = callback.shape.clone();
            return self.resolve_shape(&shape, true);
        }
        self.ensure_known_named_type(name)?;
        Ok(AbiTypeShape::Named {
            name: name.to_owned(),
        })
    }

    fn ensure_known_named_type(&self, name: &str) -> Result<()> {
        if !name.starts_with("b2")
            || self.raw.aliases.contains_key(name)
            || self.raw.structs.contains_key(name)
            || self.raw.enums.contains_key(name)
            || self.raw.opaques.contains_key(name)
            || self.raw.callbacks.contains_key(name)
        {
            return Ok(());
        }
        Err(Error::message(format!(
            "unknown Box2D ABI type `{name}` in effective header inventory"
        )))
    }
}

pub fn parse_headers(include_dir: &Path) -> Result<CApiInventory> {
    let mut headers = Vec::new();
    for entry in fs::read_dir(include_dir).map_err(|source| Error::io(include_dir, source))? {
        let entry = entry.map_err(|source| Error::io(include_dir, source))?;
        let path = entry.path();
        if path.extension().is_some_and(|extension| extension == "h") {
            headers.push(path);
        }
    }
    headers.sort();
    if headers.is_empty() {
        return Err(Error::message(format!(
            "no C headers found under {}",
            include_dir.display()
        )));
    }

    let mut functions = BTreeMap::new();
    let mut structs = BTreeMap::new();
    let mut callbacks = BTreeMap::new();
    for header in headers {
        let source = fs::read_to_string(&header).map_err(|error| Error::io(&header, error))?;
        let relative = header
            .strip_prefix(include_dir)
            .unwrap_or(&header)
            .to_string_lossy()
            .replace('\\', "/");
        parse_header(
            &source,
            &relative,
            &mut functions,
            &mut structs,
            &mut callbacks,
        )?;
    }

    Ok(CApiInventory {
        functions: functions.into_values().collect(),
        structs: structs.into_values().collect(),
        callbacks: callbacks.into_values().collect(),
    })
}

fn parse_header(
    source: &str,
    header: &str,
    functions: &mut BTreeMap<String, FunctionDecl>,
    structs: &mut BTreeMap<String, StructDecl>,
    callbacks: &mut BTreeMap<String, CallbackDecl>,
) -> Result<()> {
    let tokens = tokenize(source)?;
    let preprocessor = parse_preprocessor(source, header)?;
    let mut local_functions = BTreeMap::new();
    parse_functions(&tokens, header, &preprocessor, &mut local_functions)?;
    apply_precision_aliases(
        &mut local_functions,
        &preprocessor.precision_aliases,
        header,
    )?;
    for function in local_functions.into_values() {
        insert_unique(
            functions,
            function.name.clone(),
            function.clone(),
            header,
            function.line,
        )?;
    }
    parse_structs(&tokens, header, structs)?;
    parse_callbacks(&tokens, header, callbacks)?;
    Ok(())
}

fn parse_functions(
    tokens: &[Token],
    header: &str,
    preprocessor: &PreprocessorMetadata,
    output: &mut BTreeMap<String, FunctionDecl>,
) -> Result<()> {
    for (start, marker) in tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.text == "B2_API")
    {
        let Some(end) = declaration_end(tokens, start) else {
            return Err(Error::message(format!(
                "{header}:{}: unterminated B2_API declaration",
                marker.line
            )));
        };
        let declaration = &tokens[start + 1..end];
        let open = declaration
            .iter()
            .position(|token| token.text == "(")
            .ok_or_else(|| {
                Error::message(format!(
                    "{header}:{}: B2_API declaration is not a function",
                    marker.line
                ))
            })?;
        let name = declaration[..open]
            .iter()
            .rev()
            .find(|token| is_identifier(&token.text) && token.text.starts_with("b2"))
            .map(|token| token.text.clone())
            .ok_or_else(|| {
                Error::message(format!(
                    "{header}:{}: B2_API function name is missing",
                    marker.line
                ))
            })?;
        let signature = canonical(declaration);
        let close = matching(declaration, open, "(", ")").ok_or_else(|| {
            Error::message(format!(
                "{header}:{}: B2_API function parameter list is unterminated",
                marker.line
            ))
        })?;
        let mut parameters = split_top_level(&declaration[open + 1..close], ",")
            .into_iter()
            .filter(|parameter| !parameter.is_empty())
            .map(canonical)
            .collect::<Vec<_>>();
        if parameters.as_slice() == ["void"] {
            parameters.clear();
        }
        let availability = function_availability(
            preprocessor
                .conditions_by_line
                .get(&marker.line)
                .map(Vec::as_slice)
                .unwrap_or_default(),
            header,
            marker.line,
        )?;
        insert_unique(
            output,
            name.clone(),
            FunctionDecl {
                name,
                fingerprint: fingerprint(&signature),
                signature,
                parameters,
                physical_symbols: BTreeMap::new(),
                availability,
                header: header.to_owned(),
                line: marker.line,
            },
            header,
            marker.line,
        )?;
    }
    Ok(())
}

fn parse_structs(
    tokens: &[Token],
    header: &str,
    output: &mut BTreeMap<String, StructDecl>,
) -> Result<()> {
    let mut index = 0;
    while index + 2 < tokens.len() {
        if tokens[index].text != "typedef" || tokens[index + 1].text != "struct" {
            index += 1;
            continue;
        }
        let mut parentheses = 0_i32;
        let mut brackets = 0_i32;
        let boundary = tokens[index + 2..].iter().position(|token| {
            match token.text.as_str() {
                "(" => parentheses += 1,
                ")" => parentheses -= 1,
                "[" => brackets += 1,
                "]" => brackets -= 1,
                _ => {}
            }
            parentheses == 0 && brackets == 0 && matches!(token.text.as_str(), "{" | ";")
        });
        let Some(boundary) = boundary else {
            index += 2;
            continue;
        };
        let open = index + 2 + boundary;
        if tokens[open].text == ";" {
            index = open + 1;
            continue;
        }
        let Some(close) = matching(tokens, open, "{", "}") else {
            return Err(Error::message(format!(
                "{header}:{}: unterminated struct declaration",
                tokens[index].line
            )));
        };
        let Some(end_offset) = tokens[close + 1..]
            .iter()
            .position(|token| token.text == ";")
        else {
            return Err(Error::message(format!(
                "{header}:{}: struct typedef has no terminator",
                tokens[index].line
            )));
        };
        let end = close + 1 + end_offset;
        let name = tokens[close + 1..end]
            .iter()
            .rev()
            .find(|token| is_identifier(&token.text) && token.text.starts_with("b2"))
            .or_else(|| {
                tokens[index + 2..open]
                    .iter()
                    .find(|token| is_identifier(&token.text) && token.text.starts_with("b2"))
            })
            .map(|token| token.text.clone())
            .ok_or_else(|| {
                Error::message(format!(
                    "{header}:{}: Box2D struct name is missing",
                    tokens[index].line
                ))
            })?;
        let fields = parse_fields(&tokens[open + 1..close], &name, header)?;
        let layout = canonical_struct_layout(&fields);
        insert_unique(
            output,
            name.clone(),
            StructDecl {
                name,
                fingerprint: fingerprint(&layout),
                fields,
                header: header.to_owned(),
                line: tokens[index].line,
            },
            header,
            tokens[index].line,
        )?;
        index = end + 1;
    }
    Ok(())
}

// Nested leaves use a C member designator as `FieldDecl::name`. Overlay metadata remains
// structured so layout probes never need to parse annotations out of C declarations.
#[derive(Clone, Debug, Eq, PartialEq)]
struct OverlayMembership {
    group: String,
    alternative: String,
    alternative_path_len: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AggregateKind {
    Struct,
    Union,
}

#[derive(Clone, Debug)]
struct FieldContext {
    member_path: Vec<String>,
    overlays: Vec<OverlayMembership>,
    scope_id: String,
}

#[derive(Clone, Debug)]
struct ParsedDeclarator<'a> {
    name: String,
    tokens: &'a [Token],
}

#[derive(Clone, Debug)]
struct InlineAggregate<'a> {
    kind: AggregateKind,
    body: &'a [Token],
    declarators: Vec<ParsedDeclarator<'a>>,
}

fn parse_fields(tokens: &[Token], struct_name: &str, header: &str) -> Result<Vec<FieldDecl>> {
    parse_field_scope(
        tokens,
        &FieldContext {
            member_path: Vec::new(),
            overlays: Vec::new(),
            scope_id: struct_name.to_owned(),
        },
        AggregateKind::Struct,
        header,
    )
}

fn parse_field_scope(
    tokens: &[Token],
    context: &FieldContext,
    kind: AggregateKind,
    header: &str,
) -> Result<Vec<FieldDecl>> {
    let mut fields = Vec::new();
    for (index, declaration) in split_top_level(tokens, ";")
        .into_iter()
        .filter(|declaration| !declaration.is_empty())
        .enumerate()
    {
        if is_static_assertion(declaration) {
            continue;
        }
        let line = declaration.first().map_or(0, |token| token.line);
        let parsed = match kind {
            AggregateKind::Struct => {
                parse_struct_field_declaration(declaration, context, index, header)
            }
            AggregateKind::Union => {
                parse_union_field_declaration(declaration, context, index, header)
            }
        }
        .map_err(|error| {
            Error::message(format!(
                "{header}:{line}: failed to parse struct field declaration `{}`: {error}",
                canonical(declaration)
            ))
        })?;
        fields.extend(parsed);
    }
    Ok(fields)
}

fn parse_struct_field_declaration(
    declaration: &[Token],
    context: &FieldContext,
    declaration_index: usize,
    header: &str,
) -> Result<Vec<FieldDecl>> {
    if let Some(aggregate) = inline_aggregate(declaration)? {
        return expand_inline_aggregate(
            aggregate.kind,
            aggregate.body,
            aggregate.declarators,
            context,
            declaration_index,
            header,
        );
    }

    let (base, declarators) = parse_declarator_list(declaration)?;
    Ok(declarators
        .into_iter()
        .map(|declarator| field_from_declarator(base, declarator, context))
        .collect())
}

fn parse_union_field_declaration(
    declaration: &[Token],
    context: &FieldContext,
    declaration_index: usize,
    header: &str,
) -> Result<Vec<FieldDecl>> {
    let group = context.scope_id.clone();
    if let Some(aggregate) = inline_aggregate(declaration)? {
        if aggregate.declarators.is_empty() {
            let alternative = format!(
                "anonymous_{}@{declaration_index}",
                aggregate_name(aggregate.kind)
            );
            let mut nested = context.clone();
            nested.overlays.push(OverlayMembership {
                group,
                alternative,
                alternative_path_len: context.member_path.len(),
            });
            nested.scope_id = format!(
                "{}/{}@{declaration_index}",
                context.scope_id,
                aggregate_name(aggregate.kind)
            );
            return parse_field_scope(aggregate.body, &nested, aggregate.kind, header);
        }

        let mut fields = Vec::new();
        for declarator in aggregate.declarators {
            ensure_plain_aggregate_declarator(&declarator)?;
            let mut nested = context.clone();
            nested.member_path.push(declarator.name.clone());
            nested.overlays.push(OverlayMembership {
                group: group.clone(),
                alternative: declarator.name.clone(),
                alternative_path_len: nested.member_path.len(),
            });
            nested.scope_id = format!(
                "{}/{}:{}",
                context.scope_id,
                aggregate_name(aggregate.kind),
                declarator.name
            );
            fields.extend(parse_field_scope(
                aggregate.body,
                &nested,
                aggregate.kind,
                header,
            )?);
        }
        return Ok(fields);
    }

    let (base, declarators) = parse_declarator_list(declaration)?;
    Ok(declarators
        .into_iter()
        .map(|declarator| {
            let mut nested = context.clone();
            nested.overlays.push(OverlayMembership {
                group: group.clone(),
                alternative: declarator.name.clone(),
                alternative_path_len: context.member_path.len() + 1,
            });
            field_from_declarator(base, declarator, &nested)
        })
        .collect())
}

fn expand_inline_aggregate(
    kind: AggregateKind,
    body: &[Token],
    declarators: Vec<ParsedDeclarator<'_>>,
    context: &FieldContext,
    declaration_index: usize,
    header: &str,
) -> Result<Vec<FieldDecl>> {
    if declarators.is_empty() {
        let mut nested = context.clone();
        nested.scope_id = format!(
            "{}/{}@{declaration_index}",
            context.scope_id,
            aggregate_name(kind)
        );
        return parse_field_scope(body, &nested, kind, header);
    }

    let mut fields = Vec::new();
    for declarator in declarators {
        ensure_plain_aggregate_declarator(&declarator)?;
        let mut nested = context.clone();
        nested.member_path.push(declarator.name.clone());
        nested.scope_id = format!(
            "{}/{}:{}",
            context.scope_id,
            aggregate_name(kind),
            declarator.name
        );
        fields.extend(parse_field_scope(body, &nested, kind, header)?);
    }
    Ok(fields)
}

fn aggregate_name(kind: AggregateKind) -> &'static str {
    match kind {
        AggregateKind::Struct => "struct",
        AggregateKind::Union => "union",
    }
}

fn inline_aggregate(declaration: &[Token]) -> Result<Option<InlineAggregate<'_>>> {
    let kind = match declaration.first().map(|token| token.text.as_str()) {
        Some("struct") => AggregateKind::Struct,
        Some("union") => AggregateKind::Union,
        _ => return Ok(None),
    };
    let Some(open) = declaration.iter().position(|token| token.text == "{") else {
        return Ok(None);
    };
    let close = matching(declaration, open, "{", "}").ok_or_else(|| {
        Error::message(format!(
            "unterminated inline {} declaration",
            aggregate_name(kind)
        ))
    })?;
    let trailing = &declaration[close + 1..];
    let declarators = if trailing.is_empty() {
        Vec::new()
    } else {
        parse_declarators_without_base(trailing)?
    };
    Ok(Some(InlineAggregate {
        kind,
        body: &declaration[open + 1..close],
        declarators,
    }))
}

fn parse_declarator_list(tokens: &[Token]) -> Result<(&[Token], Vec<ParsedDeclarator<'_>>)> {
    let parts = split_top_level(tokens, ",");
    let first = parts
        .first()
        .copied()
        .filter(|part| !part.is_empty())
        .ok_or_else(|| Error::message("empty field declaration"))?;
    let (base, first_declarator) = split_declaration_specifiers(first).ok_or_else(|| {
        Error::message(format!(
            "cannot separate declaration specifiers from declarator in `{}`",
            canonical(first)
        ))
    })?;
    let mut declarators = vec![first_declarator];
    for part in parts.into_iter().skip(1) {
        declarators.push(parse_declarator_exact(part).ok_or_else(|| {
            Error::message(format!("unsupported C declarator `{}`", canonical(part)))
        })?);
    }
    Ok((base, declarators))
}

fn parse_declarators_without_base(tokens: &[Token]) -> Result<Vec<ParsedDeclarator<'_>>> {
    split_top_level(tokens, ",")
        .into_iter()
        .map(|part| {
            parse_declarator_exact(part).ok_or_else(|| {
                Error::message(format!(
                    "unsupported inline aggregate declarator `{}`",
                    canonical(part)
                ))
            })
        })
        .collect()
}

fn split_declaration_specifiers(tokens: &[Token]) -> Option<(&[Token], ParsedDeclarator<'_>)> {
    (1..tokens.len()).find_map(|split| {
        let base = &tokens[..split];
        if !valid_declaration_specifiers(base) {
            return None;
        }
        parse_declarator_exact(&tokens[split..]).map(|declarator| (base, declarator))
    })
}

fn valid_declaration_specifiers(tokens: &[Token]) -> bool {
    if tokens.is_empty() {
        return false;
    }
    let mut index = 0;
    while index < tokens.len() {
        if !is_identifier(&tokens[index].text) {
            return false;
        }
        index += 1;
        if index < tokens.len() && tokens[index].text == "(" {
            let Some(close) = matching(tokens, index, "(", ")") else {
                return false;
            };
            index = close + 1;
        }
    }
    true
}

fn parse_declarator_exact(tokens: &[Token]) -> Option<ParsedDeclarator<'_>> {
    let (name, mut index) = parse_declarator(tokens, 0)?;
    if tokens.get(index).is_some_and(|token| token.text == ":") {
        index += 1;
        if index == tokens.len() {
            return None;
        }
        index = tokens.len();
    }
    (index == tokens.len()).then_some(ParsedDeclarator { name, tokens })
}

fn parse_declarator(tokens: &[Token], mut index: usize) -> Option<(String, usize)> {
    while tokens.get(index).is_some_and(|token| token.text == "*") {
        index += 1;
        while tokens.get(index).is_some_and(|token| {
            matches!(
                token.text.as_str(),
                "const" | "volatile" | "restrict" | "_Atomic"
            )
        }) {
            index += 1;
        }
    }

    let (name, mut index) = match tokens.get(index) {
        Some(token) if is_identifier(&token.text) => (token.text.clone(), index + 1),
        Some(token) if token.text == "(" => {
            let close = matching(tokens, index, "(", ")")?;
            let parsed = parse_declarator_exact(&tokens[index + 1..close])?;
            (parsed.name, close + 1)
        }
        _ => return None,
    };

    while matches!(
        tokens.get(index).map(|token| token.text.as_str()),
        Some("[") | Some("(")
    ) {
        let (left, right) = if tokens[index].text == "[" {
            ("[", "]")
        } else {
            ("(", ")")
        };
        index = matching(tokens, index, left, right)? + 1;
    }
    Some((name, index))
}

fn ensure_plain_aggregate_declarator(declarator: &ParsedDeclarator<'_>) -> Result<()> {
    if declarator.tokens.len() == 1 && declarator.tokens[0].text == declarator.name {
        return Ok(());
    }
    Err(Error::message(format!(
        "inline aggregate field `{}` cannot be expanded into addressable member paths; only a plain named field is supported",
        canonical(declarator.tokens)
    )))
}

fn field_from_declarator(
    base: &[Token],
    declarator: ParsedDeclarator<'_>,
    context: &FieldContext,
) -> FieldDecl {
    let mut path = context.member_path.clone();
    path.push(declarator.name);
    let name = path.join(".");
    let signature = canonical_join(base, declarator.tokens);
    let overlays = context
        .overlays
        .iter()
        .map(|overlay| OverlayDecl {
            group: overlay.group.clone(),
            alternative: overlay.alternative.clone(),
            relative_path: path
                .get(overlay.alternative_path_len..)
                .unwrap_or_default()
                .to_vec(),
        })
        .collect();
    FieldDecl {
        name,
        signature,
        overlays,
    }
}

fn canonical_struct_layout(fields: &[FieldDecl]) -> String {
    let mut output = String::new();
    for field in fields {
        push_fingerprint_component(&mut output, "field");
        push_fingerprint_component(&mut output, &field.name);
        push_fingerprint_component(&mut output, &field.signature);
        write!(output, "{}:", field.overlays.len()).expect("write to string");
        for overlay in &field.overlays {
            push_fingerprint_component(&mut output, &overlay.group);
            push_fingerprint_component(&mut output, &overlay.alternative);
            write!(output, "{}:", overlay.relative_path.len()).expect("write to string");
            for segment in &overlay.relative_path {
                push_fingerprint_component(&mut output, segment);
            }
        }
    }
    output
}

fn push_fingerprint_component(output: &mut String, value: &str) {
    write!(output, "{}:", value.len()).expect("write to string");
    output.push_str(value);
}

fn canonical_join(base: &[Token], declarator: &[Token]) -> String {
    if base.is_empty() {
        return canonical(declarator);
    }
    format!("{} {}", canonical(base), canonical(declarator))
}

fn is_static_assertion(declaration: &[Token]) -> bool {
    declaration.first().is_some_and(|token| {
        matches!(
            token.text.as_str(),
            "_Static_assert" | "static_assert" | "B2_STATIC_ASSERT"
        )
    })
}

fn parse_callbacks(
    tokens: &[Token],
    header: &str,
    output: &mut BTreeMap<String, CallbackDecl>,
) -> Result<()> {
    for (start, marker) in tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.text == "typedef")
    {
        let Some(end) = declaration_end(tokens, start) else {
            continue;
        };
        let declaration = &tokens[start + 1..end];
        if declaration
            .first()
            .is_some_and(|token| matches!(token.text.as_str(), "struct" | "enum" | "union"))
        {
            continue;
        }
        let Some(open) = declaration.iter().position(|token| token.text == "(") else {
            continue;
        };
        let Some(name_token) = declaration[..open]
            .iter()
            .rev()
            .find(|token| is_identifier(&token.text) && token.text.starts_with("b2"))
        else {
            continue;
        };
        let name = name_token.text.clone();
        if !name.ends_with("Fcn") && !name.ends_with("Callback") {
            continue;
        }
        let signature = canonical(declaration);
        insert_unique(
            output,
            name.clone(),
            CallbackDecl {
                name,
                fingerprint: fingerprint(&signature),
                signature,
                header: header.to_owned(),
                line: marker.line,
            },
            header,
            marker.line,
        )?;
    }
    Ok(())
}

fn insert_unique<T: Eq>(
    output: &mut BTreeMap<String, T>,
    name: String,
    value: T,
    header: &str,
    line: usize,
) -> Result<()> {
    if let Some(previous) = output.get(&name) {
        if previous == &value {
            return Ok(());
        }
        return Err(Error::message(format!(
            "{header}:{line}: duplicate C capability `{name}`"
        )));
    }
    output.insert(name, value);
    Ok(())
}

fn declaration_end(tokens: &[Token], start: usize) -> Option<usize> {
    let mut round = 0_i32;
    let mut square = 0_i32;
    let mut braces = 0_i32;
    for (index, token) in tokens.iter().enumerate().skip(start + 1) {
        match token.text.as_str() {
            "(" => round += 1,
            ")" => round -= 1,
            "[" => square += 1,
            "]" => square -= 1,
            "{" => braces += 1,
            "}" => braces -= 1,
            ";" if round == 0 && square == 0 && braces == 0 => return Some(index),
            _ => {}
        }
    }
    None
}

fn matching(tokens: &[Token], open: usize, left: &str, right: &str) -> Option<usize> {
    let mut depth = 0;
    for (index, token) in tokens.iter().enumerate().skip(open) {
        if token.text == left {
            depth += 1;
        } else if token.text == right {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

fn split_top_level<'a>(tokens: &'a [Token], separator: &str) -> Vec<&'a [Token]> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut round = 0_i32;
    let mut square = 0_i32;
    let mut braces = 0_i32;
    for (index, token) in tokens.iter().enumerate() {
        match token.text.as_str() {
            "(" => round += 1,
            ")" => round -= 1,
            "[" => square += 1,
            "]" => square -= 1,
            "{" => braces += 1,
            "}" => braces -= 1,
            value if value == separator && round == 0 && square == 0 && braces == 0 => {
                parts.push(&tokens[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(&tokens[start..]);
    parts
}

fn canonical(tokens: &[Token]) -> String {
    tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn fingerprint(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in value.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    chars
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn parse_preprocessor(source: &str, header: &str) -> Result<PreprocessorMetadata> {
    let mut metadata = PreprocessorMetadata::default();
    let mut conditions = Vec::new();
    for (offset, line) in source.lines().enumerate() {
        let line_number = offset + 1;
        let trimmed = line.trim_start();
        let Some(directive) = trimmed.strip_prefix('#').map(str::trim_start) else {
            metadata
                .conditions_by_line
                .insert(line_number, conditions.clone());
            continue;
        };
        let (name, argument) = directive
            .split_once(char::is_whitespace)
            .map_or((directive, ""), |(name, argument)| (name, argument.trim()));
        match name {
            "if" => conditions.push(validated_preprocessor_condition(
                argument,
                header,
                line_number,
            )?),
            "ifdef" => conditions.push(classify_ifdef_condition(argument, false)),
            "ifndef" => conditions.push(classify_ifdef_condition(argument, true)),
            "elif" => {
                if conditions.pop().is_none() {
                    return Err(Error::message(format!(
                        "{header}:{line_number}: unmatched #elif"
                    )));
                }
                conditions.push(validated_preprocessor_condition(
                    argument,
                    header,
                    line_number,
                )?);
            }
            "else" => {
                let Some(condition) = conditions.last_mut() else {
                    return Err(Error::message(format!(
                        "{header}:{line_number}: unmatched #else"
                    )));
                };
                *condition = invert_preprocessor_condition(condition);
            }
            "endif" => {
                if conditions.pop().is_none() {
                    return Err(Error::message(format!(
                        "{header}:{line_number}: unmatched #endif"
                    )));
                }
            }
            "define" => {
                if let Some((logical, physical)) = parse_object_alias(argument) {
                    metadata.precision_aliases.push(PrecisionAlias {
                        logical,
                        physical,
                        scope: alias_scope(&conditions),
                        line: line_number,
                    });
                }
            }
            _ => {}
        }
    }
    if !conditions.is_empty() {
        return Err(Error::message(format!(
            "{header}: unterminated preprocessor conditional"
        )));
    }
    Ok(metadata)
}

fn classify_preprocessor_condition(expression: &str) -> PreprocessorCondition {
    let normalized = expression
        .chars()
        .filter(|character| !character.is_ascii_whitespace())
        .collect::<String>();
    match normalized.as_str() {
        "defined(BOX2D_DOUBLE_PRECISION)" => PreprocessorCondition::DoublePrecision,
        "!defined(BOX2D_DOUBLE_PRECISION)" => PreprocessorCondition::SinglePrecision,
        "!defined(NDEBUG)||defined(B2_ENABLE_ASSERT)"
        | "defined(B2_ENABLE_ASSERT)||!defined(NDEBUG)" => PreprocessorCondition::DebugOrAssertions,
        _ => PreprocessorCondition::Other(normalized),
    }
}

fn validated_preprocessor_condition(
    expression: &str,
    header: &str,
    line: usize,
) -> Result<PreprocessorCondition> {
    let condition = classify_preprocessor_condition(expression);
    if matches!(&condition, PreprocessorCondition::Other(value) if value.contains("BOX2D_DOUBLE_PRECISION"))
    {
        return Err(Error::message(format!(
            "{header}:{line}: unsupported precision condition `{expression}`"
        )));
    }
    Ok(condition)
}

fn classify_ifdef_condition(name: &str, negated: bool) -> PreprocessorCondition {
    if name.trim() == "BOX2D_DOUBLE_PRECISION" {
        if negated {
            PreprocessorCondition::SinglePrecision
        } else {
            PreprocessorCondition::DoublePrecision
        }
    } else {
        PreprocessorCondition::Other(if negated {
            format!("!defined({})", name.trim())
        } else {
            format!("defined({})", name.trim())
        })
    }
}

fn invert_preprocessor_condition(condition: &PreprocessorCondition) -> PreprocessorCondition {
    match condition {
        PreprocessorCondition::DoublePrecision => PreprocessorCondition::SinglePrecision,
        PreprocessorCondition::SinglePrecision => PreprocessorCondition::DoublePrecision,
        PreprocessorCondition::DebugOrAssertions => {
            PreprocessorCondition::Other("!(debug-profile||assertions-enabled)".to_owned())
        }
        PreprocessorCondition::Other(expression) => {
            PreprocessorCondition::Other(format!("!({expression})"))
        }
    }
}

fn parse_object_alias(argument: &str) -> Option<(String, String)> {
    let mut parts = argument.split_whitespace();
    let logical = parts.next()?;
    let physical = parts.next()?;
    if logical.contains('(')
        || !logical.starts_with("b2")
        || !physical.starts_with("b2")
        || !is_identifier(logical)
        || !is_identifier(physical)
    {
        return None;
    }
    Some((logical.to_owned(), physical.to_owned()))
}

fn alias_scope(conditions: &[PreprocessorCondition]) -> AliasScope {
    let mut scope = AliasScope::All;
    for condition in conditions {
        scope = match (scope, condition) {
            (AliasScope::All, PreprocessorCondition::DoublePrecision)
            | (AliasScope::Double, PreprocessorCondition::DoublePrecision) => AliasScope::Double,
            (AliasScope::All, PreprocessorCondition::SinglePrecision)
            | (AliasScope::Single, PreprocessorCondition::SinglePrecision) => AliasScope::Single,
            (_, PreprocessorCondition::DebugOrAssertions | PreprocessorCondition::Other(_))
            | (AliasScope::Single, PreprocessorCondition::DoublePrecision)
            | (AliasScope::Double, PreprocessorCondition::SinglePrecision)
            | (AliasScope::Unsupported, _) => AliasScope::Unsupported,
        };
    }
    scope
}

fn function_availability(
    conditions: &[PreprocessorCondition],
    header: &str,
    line: usize,
) -> Result<Vec<String>> {
    if conditions.is_empty() {
        return Ok(vec!["always".to_owned()]);
    }
    if conditions == [PreprocessorCondition::DebugOrAssertions] {
        return Ok(vec![
            "debug-profile".to_owned(),
            "assertions-enabled".to_owned(),
        ]);
    }
    Err(Error::message(format!(
        "{header}:{line}: B2_API declaration has an unsupported preprocessor availability condition {conditions:?}"
    )))
}

fn apply_precision_aliases(
    functions: &mut BTreeMap<String, FunctionDecl>,
    aliases: &[PrecisionAlias],
    header: &str,
) -> Result<()> {
    for function in functions.values_mut() {
        for mode in ["single", "double"] {
            let physical = resolve_precision_alias(&function.name, mode, aliases, header)?;
            function.physical_symbols.insert(mode.to_owned(), physical);
        }
    }
    Ok(())
}

fn resolve_precision_alias(
    logical: &str,
    mode: &str,
    aliases: &[PrecisionAlias],
    header: &str,
) -> Result<String> {
    let mut current = logical.to_owned();
    let mut visited = BTreeSet::new();
    loop {
        if !visited.insert(current.clone()) {
            return Err(Error::message(format!(
                "{header}: precision alias cycle while resolving `{logical}` for `{mode}`"
            )));
        }
        let relevant = aliases
            .iter()
            .filter(|alias| alias.logical == current)
            .filter(|alias| {
                alias.scope == AliasScope::All
                    || (mode == "single" && alias.scope == AliasScope::Single)
                    || (mode == "double" && alias.scope == AliasScope::Double)
                    || alias.scope == AliasScope::Unsupported
            })
            .collect::<Vec<_>>();
        if relevant
            .iter()
            .any(|alias| alias.scope == AliasScope::Unsupported)
        {
            return Err(Error::message(format!(
                "{header}:{}: `{}` has an unsupported conditional physical alias",
                relevant[0].line, current
            )));
        }
        let targets = relevant
            .iter()
            .map(|alias| alias.physical.as_str())
            .collect::<BTreeSet<_>>();
        match targets.len() {
            0 => return Ok(current),
            1 => current = targets.into_iter().next().expect("one target").to_owned(),
            _ => {
                return Err(Error::message(format!(
                    "{header}: ambiguous physical aliases for `{current}` in `{mode}` mode"
                )));
            }
        }
    }
}

fn tokenize(source: &str) -> Result<Vec<Token>> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    let mut line = 1;
    let mut at_line_start = true;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'\n' {
            line += 1;
            index += 1;
            at_line_start = true;
            continue;
        }
        if byte.is_ascii_whitespace() {
            index += 1;
            continue;
        }
        if at_line_start && byte == b'#' {
            index += 1;
            loop {
                let Some(relative) = bytes[index..]
                    .iter()
                    .position(|candidate| *candidate == b'\n')
                else {
                    index = bytes.len();
                    break;
                };
                let newline = index + relative;
                let continuation = newline > 0 && bytes[newline - 1] == b'\\';
                index = newline;
                if !continuation {
                    break;
                }
                line += 1;
                index += 1;
            }
            continue;
        }
        at_line_start = false;
        if bytes[index..].starts_with(b"//") {
            index += bytes[index..]
                .iter()
                .position(|candidate| *candidate == b'\n')
                .unwrap_or(bytes.len() - index);
            continue;
        }
        if bytes[index..].starts_with(b"/*") {
            let start_line = line;
            index += 2;
            let mut closed = false;
            while index + 1 < bytes.len() {
                if bytes[index..].starts_with(b"*/") {
                    index += 2;
                    closed = true;
                    break;
                }
                if bytes[index] == b'\n' {
                    line += 1;
                    at_line_start = true;
                }
                index += 1;
            }
            if !closed {
                return Err(Error::message(format!(
                    "unterminated C block comment starting on line {start_line}"
                )));
            }
            continue;
        }

        let token_line = line;
        if byte == b'_' || byte.is_ascii_alphabetic() {
            let start = index;
            index += 1;
            while index < bytes.len()
                && (bytes[index] == b'_' || bytes[index].is_ascii_alphanumeric())
            {
                index += 1;
            }
            tokens.push(Token {
                text: source[start..index].to_owned(),
                line: token_line,
            });
            continue;
        }
        if byte.is_ascii_digit() {
            let start = index;
            index += 1;
            while index < bytes.len()
                && (bytes[index].is_ascii_alphanumeric()
                    || matches!(bytes[index], b'.' | b'_' | b'+' | b'-'))
            {
                index += 1;
            }
            tokens.push(Token {
                text: source[start..index].to_owned(),
                line: token_line,
            });
            continue;
        }
        if matches!(byte, b'"' | b'\'') {
            let delimiter = byte;
            let start = index;
            index += 1;
            let mut closed = false;
            while index < bytes.len() {
                if bytes[index] == b'\\' {
                    index += 2;
                    continue;
                }
                if bytes[index] == delimiter {
                    index += 1;
                    closed = true;
                    break;
                }
                if bytes[index] == b'\n' {
                    line += 1;
                }
                index += 1;
            }
            if !closed {
                return Err(Error::message(format!(
                    "unterminated C literal starting on line {token_line}"
                )));
            }
            tokens.push(Token {
                text: source[start..index].to_owned(),
                line: token_line,
            });
            continue;
        }
        let punctuation = [
            "...", "<<=", ">>=", "->", "++", "--", "<<", ">>", "<=", ">=", "==", "!=", "&&", "||",
            "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "##",
        ]
        .into_iter()
        .find(|candidate| source[index..].starts_with(candidate))
        .unwrap_or(&source[index..index + 1]);
        tokens.push(Token {
            text: punctuation.to_owned(),
            line: token_line,
        });
        index += punctuation.len();
    }
    Ok(tokens)
}

pub fn inventory_by_name<T>(items: &[T], name: impl Fn(&T) -> &str) -> BTreeMap<String, &T> {
    items
        .iter()
        .map(|item| (name(item).to_owned(), item))
        .collect()
}

pub fn header_paths(include_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = fs::read_dir(include_dir)
        .map_err(|source| Error::io(include_dir, source))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|extension| extension == "h"))
        .collect::<Vec<_>>();
    paths.sort();
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_precision_header_fixture(
        source: &str,
        precision: CAbiPrecision,
    ) -> Result<PrecisionCApiInventory> {
        let mut raw = RawPrecisionInventory::default();
        parse_precision_header(source, "fixture.h", precision, &mut raw)?;
        resolve_precision_inventory(precision, raw)
    }

    fn parse_struct_fixture(source: &str, name: &str) -> StructDecl {
        let mut structs = BTreeMap::new();
        parse_header(
            source,
            "fixture.h",
            &mut BTreeMap::new(),
            &mut structs,
            &mut BTreeMap::new(),
        )
        .expect("fixture should parse");
        structs.remove(name).expect("fixture struct should exist")
    }

    #[test]
    fn parser_ignores_comments_and_preprocessor_definitions() {
        let source = r#"
            #define B2_API extern
            // B2_API void b2CommentOnly(void);
            /* B2_API void b2AlsoCommentOnly(void); */
            B2_API
            const b2Thing* b2Real(
                const b2Thing* thing,
                int count
            );
        "#;
        let mut functions = BTreeMap::new();
        parse_header(
            source,
            "fixture.h",
            &mut functions,
            &mut BTreeMap::new(),
            &mut BTreeMap::new(),
        )
        .expect("fixture should parse");
        assert_eq!(functions.keys().collect::<Vec<_>>(), ["b2Real"]);
        assert!(
            functions["b2Real"]
                .signature
                .contains("const b2Thing * b2Real")
        );
        assert_eq!(
            functions["b2Real"].physical_symbols,
            BTreeMap::from([
                ("double".to_owned(), "b2Real".to_owned()),
                ("single".to_owned(), "b2Real".to_owned()),
            ])
        );
        assert_eq!(functions["b2Real"].availability, ["always"]);
    }

    #[test]
    fn parser_derives_precision_specific_physical_symbols_from_header_aliases() {
        let source = r#"
            #if defined(BOX2D_DOUBLE_PRECISION)
            #define b2CreateWorld b2CreateWorldDoublePrecision
            #endif

            B2_API b2WorldId b2CreateWorld(const b2WorldDef* def);
        "#;
        let mut functions = BTreeMap::new();
        parse_header(
            source,
            "fixture.h",
            &mut functions,
            &mut BTreeMap::new(),
            &mut BTreeMap::new(),
        )
        .expect("fixture should parse");

        assert_eq!(
            functions["b2CreateWorld"].physical_symbols,
            BTreeMap::from([
                (
                    "double".to_owned(),
                    "b2CreateWorldDoublePrecision".to_owned(),
                ),
                ("single".to_owned(), "b2CreateWorld".to_owned()),
            ])
        );
    }

    #[test]
    fn parser_derives_debug_or_assertion_availability_from_header_condition() {
        let source = r#"
            #if !defined(NDEBUG) || defined(B2_ENABLE_ASSERT)
            B2_API int b2InternalAssert(const char* condition, const char* file, int line);
            #endif
            B2_API int b2Always(void);
        "#;
        let mut functions = BTreeMap::new();
        parse_header(
            source,
            "fixture.h",
            &mut functions,
            &mut BTreeMap::new(),
            &mut BTreeMap::new(),
        )
        .expect("fixture should parse");

        assert_eq!(
            functions["b2InternalAssert"].availability,
            ["debug-profile", "assertions-enabled"]
        );
        assert_eq!(
            functions["b2InternalAssert"].parameters,
            ["const char * condition", "const char * file", "int line"]
        );
        assert_eq!(functions["b2Always"].availability, ["always"]);
        assert!(functions["b2Always"].parameters.is_empty());
    }

    #[test]
    fn parser_rejects_ambiguous_and_cyclic_physical_aliases() {
        let ambiguous = r#"
            #if defined(BOX2D_DOUBLE_PRECISION)
            #define b2CreateWorld b2CreateWorldDoublePrecision
            #define b2CreateWorld b2CreateWorldOtherPrecision
            #endif
            B2_API b2WorldId b2CreateWorld(const b2WorldDef* def);
        "#;
        let error = parse_header(
            ambiguous,
            "ambiguous.h",
            &mut BTreeMap::new(),
            &mut BTreeMap::new(),
            &mut BTreeMap::new(),
        )
        .expect_err("ambiguous physical aliases must fail closed");
        assert!(error.to_string().contains("ambiguous physical aliases"));

        let cyclic = r#"
            #if defined(BOX2D_DOUBLE_PRECISION)
            #define b2CreateWorld b2CreateWorldDoublePrecision
            #define b2CreateWorldDoublePrecision b2CreateWorld
            #endif
            B2_API b2WorldId b2CreateWorld(const b2WorldDef* def);
        "#;
        let error = parse_header(
            cyclic,
            "cyclic.h",
            &mut BTreeMap::new(),
            &mut BTreeMap::new(),
            &mut BTreeMap::new(),
        )
        .expect_err("cyclic physical aliases must fail closed");
        assert!(error.to_string().contains("precision alias cycle"));
    }

    #[test]
    fn parser_rejects_unknown_api_availability_conditions() {
        let source = r#"
            #if defined(B2_EXPERIMENTAL_API)
            B2_API void b2Experimental(void);
            #endif
        "#;
        let error = parse_header(
            source,
            "fixture.h",
            &mut BTreeMap::new(),
            &mut BTreeMap::new(),
            &mut BTreeMap::new(),
        )
        .expect_err("unknown API availability must fail closed");
        assert!(
            error
                .to_string()
                .contains("unsupported preprocessor availability condition")
        );
    }

    #[test]
    fn parser_indexes_struct_fields_and_callbacks() {
        let source = r#"
            typedef struct b2Pair {
                float values[2];
                const void* context;
            } b2Pair;
            typedef bool b2VisitFcn(const b2Pair* pair, void* context);
        "#;
        let mut structs = BTreeMap::new();
        let mut callbacks = BTreeMap::new();
        parse_header(
            source,
            "fixture.h",
            &mut BTreeMap::new(),
            &mut structs,
            &mut callbacks,
        )
        .expect("fixture should parse");
        assert_eq!(structs["b2Pair"].fields[0].name, "values");
        assert_eq!(structs["b2Pair"].fields[1].name, "context");
        assert!(callbacks.contains_key("b2VisitFcn"));
    }

    #[test]
    fn parser_does_not_cross_forward_declaration_terminators() {
        let source = r#"
            typedef struct b2Forward b2Forward;
            static inline int helper(void) { return 1; }
            typedef struct b2Real {
                int value;
            } b2Real;
        "#;
        let mut structs = BTreeMap::new();
        parse_header(
            source,
            "fixture.h",
            &mut BTreeMap::new(),
            &mut structs,
            &mut BTreeMap::new(),
        )
        .expect("fixture should parse");

        assert!(!structs.contains_key("b2Forward"));
        assert_eq!(structs.keys().collect::<Vec<_>>(), ["b2Real"]);
        assert_eq!(structs["b2Real"].fields[0].name, "value");
    }

    #[test]
    fn parser_expands_each_declarator_with_its_shared_type() {
        let declaration = parse_struct_fixture(
            r#"
                typedef struct b2Simplex {
                    b2SimplexVertex v1, v2, v3;
                    const void *first, *second;
                    float values[2], matrix[2][3];
                } b2Simplex;
            "#,
            "b2Simplex",
        );

        assert_eq!(
            declaration
                .fields
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>(),
            ["v1", "v2", "v3", "first", "second", "values", "matrix"]
        );
        assert_eq!(declaration.fields[0].signature, "b2SimplexVertex v1");
        assert_eq!(declaration.fields[1].signature, "b2SimplexVertex v2");
        assert_eq!(declaration.fields[2].signature, "b2SimplexVertex v3");
        assert_eq!(declaration.fields[3].signature, "const void * first");
        assert_eq!(declaration.fields[4].signature, "const void * second");
        assert_eq!(declaration.fields[5].signature, "float values [ 2 ]");
        assert_eq!(declaration.fields[6].signature, "float matrix [ 2 ] [ 3 ]");
    }

    #[test]
    fn parser_identifies_inline_function_pointer_fields_not_parameters() {
        let declaration = parse_struct_fixture(
            r#"
                #define B2_FIELD_ALIGN(value)
                typedef struct b2DebugDraw {
                    // The final parameter must not become the field name.
                    void (*DrawPolygonFcn)(const b2Vec2* vertices, int vertexCount,
                                           b2HexColor color, void* context);
                    void (*DrawStringFcn)(b2Vec2 p, const char* text,
                                          b2HexColor color, void* context);
                    B2_FIELD_ALIGN(16) float samples[2];
                    _Static_assert(sizeof(float) == 4, "float ABI");
                } b2DebugDraw;
            "#,
            "b2DebugDraw",
        );

        assert_eq!(
            declaration
                .fields
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>(),
            ["DrawPolygonFcn", "DrawStringFcn", "samples"]
        );
        assert!(
            declaration.fields[0]
                .signature
                .starts_with("void ( * DrawPolygonFcn ) (")
        );
        assert!(
            declaration.fields[0]
                .signature
                .ends_with("void * context )")
        );
        assert_eq!(
            declaration.fields[2].signature,
            "B2_FIELD_ALIGN ( 16 ) float samples [ 2 ]"
        );
    }

    #[test]
    fn parser_flattens_anonymous_unions_with_addressable_paths_and_overlay_groups() {
        let declaration = parse_struct_fixture(
            r#"
                typedef struct b2TreeNode {
                    b2AABB aabb;
                    uint64_t categoryBits;
                    union {
                        struct {
                            int32_t child1, child2;
                        } children;
                        uint64_t userData;
                    };
                    union {
                        int32_t parent;
                        int32_t next;
                    };
                    uint16_t height;
                    uint16_t flags;
                } b2TreeNode;
            "#,
            "b2TreeNode",
        );

        assert_eq!(
            declaration
                .fields
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>(),
            [
                "aabb",
                "categoryBits",
                "children.child1",
                "children.child2",
                "userData",
                "parent",
                "next",
                "height",
                "flags",
            ]
        );

        assert_eq!(declaration.fields[2].signature, "int32_t child1");
        assert_eq!(
            declaration.fields[2].overlays,
            [OverlayDecl {
                group: "b2TreeNode/union@2".to_owned(),
                alternative: "children".to_owned(),
                relative_path: vec!["child1".to_owned()],
            }]
        );
        assert_eq!(declaration.fields[3].overlays[0].relative_path, ["child2"]);
        assert_eq!(declaration.fields[4].overlays[0].alternative, "userData");
        assert!(declaration.fields[4].overlays[0].relative_path.is_empty());
        assert_eq!(declaration.fields[5].overlays[0].alternative, "parent");
        assert!(declaration.fields[5].overlays[0].relative_path.is_empty());
        assert_eq!(declaration.fields[6].overlays[0].alternative, "next");
        assert!(declaration.fields[6].overlays[0].relative_path.is_empty());
    }

    #[test]
    fn parser_recurses_through_named_inline_structs_and_unions() {
        let declaration = parse_struct_fixture(
            r#"
                typedef struct b2Nested {
                    struct {
                        union b2Value {
                            int32_t integral;
                            float real;
                        } value;
                        struct { uint16_t low, high; } words;
                    } nested;
                } b2Nested;
            "#,
            "b2Nested",
        );

        assert_eq!(
            declaration
                .fields
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>(),
            [
                "nested.value.integral",
                "nested.value.real",
                "nested.words.low",
                "nested.words.high",
            ]
        );
        assert_eq!(
            declaration.fields[0].overlays,
            [OverlayDecl {
                group: "b2Nested/struct:nested/union:value".to_owned(),
                alternative: "integral".to_owned(),
                relative_path: Vec::new(),
            }]
        );
        assert_eq!(declaration.fields[1].overlays[0].alternative, "real");
    }

    #[test]
    fn parser_fails_closed_for_non_addressable_inline_aggregate_declarators() {
        let source = r#"
            typedef struct b2Invalid {
                struct { int value; } *storage;
            } b2Invalid;
        "#;
        let error = parse_header(
            source,
            "fixture.h",
            &mut BTreeMap::new(),
            &mut BTreeMap::new(),
            &mut BTreeMap::new(),
        )
        .expect_err("pointer to inline aggregate should fail closed");
        assert!(
            error
                .to_string()
                .contains("only a plain named field is supported")
        );
    }

    #[test]
    fn field_signatures_and_fingerprints_ignore_formatting_and_comments() {
        let compact = parse_struct_fixture(
            "typedef struct b2Stable { int first, second; void (*visit)(int, void*); } b2Stable;",
            "b2Stable",
        );
        let formatted = parse_struct_fixture(
            r#"
                typedef struct b2Stable
                {
                    int first, /* does not affect the ABI */ second;
                    void
                    (
                        *visit
                    )
                    (
                        int,
                        void*
                    );
                } b2Stable;
            "#,
            "b2Stable",
        );

        assert_eq!(compact.fields, formatted.fields);
        assert_eq!(compact.fingerprint, formatted.fingerprint);
    }

    #[test]
    fn precision_inventory_selects_b2_pos_alias_or_struct() {
        let source = r#"
            typedef struct b2Vec2 { float x, y; } b2Vec2;

            #if defined(BOX2D_DOUBLE_PRECISION)
            typedef struct b2Pos { double x, y; } b2Pos;
            #else
            typedef b2Vec2 b2Pos;
            #endif
        "#;

        let single = parse_precision_header_fixture(source, CAbiPrecision::Single)
            .expect("single-precision fixture should parse");
        let double = parse_precision_header_fixture(source, CAbiPrecision::Double)
            .expect("double-precision fixture should parse");

        assert!(single.alias("b2Pos").is_some());
        assert!(single.structure("b2Pos").is_none());
        assert!(double.alias("b2Pos").is_none());
        assert!(double.structure("b2Pos").is_some());
        assert_eq!(
            single.type_shape("b2Pos"),
            single.type_shape("b2Vec2"),
            "the single-precision alias must resolve to the effective b2Vec2 layout"
        );
        assert_ne!(
            single.type_fingerprint("b2Pos"),
            double.type_fingerprint("b2Pos"),
            "float and double world positions must have different ABI fingerprints"
        );
    }

    #[test]
    fn precision_inventory_selects_world_cast_output_alias_or_struct() {
        let source = r#"
            typedef struct b2Vec2 { float x, y; } b2Vec2;
            typedef struct b2CastOutput {
                b2Vec2 normal;
                b2Vec2 point;
                float fraction;
                int iterations;
                bool hit;
            } b2CastOutput;

            #if defined(BOX2D_DOUBLE_PRECISION)
            typedef struct b2Pos { double x, y; } b2Pos;
            typedef struct b2WorldCastOutput {
                b2Vec2 normal;
                b2Pos point;
                float fraction;
                int iterations;
                bool hit;
            } b2WorldCastOutput;
            #else
            typedef b2Vec2 b2Pos;
            typedef b2CastOutput b2WorldCastOutput;
            #endif
        "#;

        let single = parse_precision_header_fixture(source, CAbiPrecision::Single)
            .expect("single-precision fixture should parse");
        let double = parse_precision_header_fixture(source, CAbiPrecision::Double)
            .expect("double-precision fixture should parse");

        assert!(single.alias("b2WorldCastOutput").is_some());
        assert!(double.structure("b2WorldCastOutput").is_some());
        assert_eq!(
            single.type_shape("b2WorldCastOutput"),
            single.type_shape("b2CastOutput")
        );
        assert_ne!(
            single.type_fingerprint("b2WorldCastOutput"),
            double.type_fingerprint("b2WorldCastOutput")
        );
    }

    #[test]
    fn precision_inventory_recursively_fingerprints_structs_callbacks_and_functions() {
        let source = r#"
            typedef struct b2Vec2 { float x, y; } b2Vec2;
            #if defined(BOX2D_DOUBLE_PRECISION)
            typedef struct b2Pos { double x, y; } b2Pos;
            #else
            typedef b2Vec2 b2Pos;
            #endif

            typedef struct b2QueryResult { b2Pos point; } b2QueryResult;
            typedef bool b2QueryFcn(const b2QueryResult* result, b2Pos origin, void* context);
            B2_API b2QueryResult b2QueryWorld(b2Pos origin, b2QueryFcn* callback, void* context);
        "#;

        let single = parse_precision_header_fixture(source, CAbiPrecision::Single)
            .expect("single-precision fixture should parse");
        let double = parse_precision_header_fixture(source, CAbiPrecision::Double)
            .expect("double-precision fixture should parse");

        assert_ne!(
            single.type_fingerprint("b2QueryResult"),
            double.type_fingerprint("b2QueryResult")
        );
        assert_ne!(
            single.callback("b2QueryFcn").map(|decl| &decl.fingerprint),
            double.callback("b2QueryFcn").map(|decl| &decl.fingerprint)
        );
        assert_ne!(
            single
                .function("b2QueryWorld")
                .map(|decl| &decl.fingerprint),
            double
                .function("b2QueryWorld")
                .map(|decl| &decl.fingerprint)
        );
    }

    #[test]
    fn precision_inventory_fails_closed_for_unknown_conditions_and_alias_cycles() {
        let unknown = r#"
            #if defined(BOX2D_DOUBLE_PRECISION) && defined(B2_EXPERIMENTAL)
            typedef struct b2Pos { double x, y; } b2Pos;
            #else
            typedef struct b2Pos { float x, y; } b2Pos;
            #endif
        "#;
        let error = parse_precision_header_fixture(unknown, CAbiPrecision::Double)
            .expect_err("compound precision conditions must fail closed");
        assert!(
            error
                .to_string()
                .contains("unsupported precision condition")
        );

        let cycle = r#"
            typedef b2Second b2First;
            typedef b2First b2Second;
        "#;
        let error = parse_precision_header_fixture(cycle, CAbiPrecision::Single)
            .expect_err("alias cycles must fail closed");
        assert!(error.to_string().contains("type alias cycle"));
    }

    #[test]
    fn pointer_fingerprint_normalizes_c_const_pointee_and_rust_const_pointer() {
        let c_shape = AbiTypeShape::Pointer {
            mutable: true,
            pointee: Box::new(AbiTypeShape::Qualified {
                is_const: true,
                is_volatile: false,
                inner: Box::new(AbiTypeShape::Named {
                    name: "b2Thing".to_owned(),
                }),
            }),
        };
        let rust_shape = AbiTypeShape::Pointer {
            mutable: false,
            pointee: Box::new(AbiTypeShape::Named {
                name: "b2Thing".to_owned(),
            }),
        };
        assert_eq!(c_shape.fingerprint(), rust_shape.fingerprint());
    }

    #[test]
    fn function_pointer_fingerprint_normalizes_bindgen_option_representation() {
        let function = AbiTypeShape::Function {
            result: Box::new(AbiTypeShape::Primitive {
                primitive: AbiPrimitive::Void,
            }),
            parameters: vec![AbiTypeShape::Primitive {
                primitive: AbiPrimitive::I32,
            }],
            variadic: false,
        };
        let c_pointer = AbiTypeShape::Pointer {
            mutable: true,
            pointee: Box::new(function.clone()),
        };
        assert_eq!(c_pointer.fingerprint(), function.fingerprint());
    }

    #[test]
    fn precision_inventory_resolves_integer_macro_array_lengths() {
        let inventory = parse_precision_header_fixture(
            r#"
                #define B2_POINT_COUNT 8
                typedef struct b2Points { float values[B2_POINT_COUNT]; } b2Points;
            "#,
            CAbiPrecision::Single,
        )
        .expect("integer macro fixture should parse");
        let AbiTypeShape::Aggregate { fields } = inventory
            .type_shape("b2Points")
            .expect("fixture aggregate should exist")
        else {
            panic!("fixture type should resolve to an aggregate");
        };
        let AbiTypeShape::Array { length, .. } = &fields[0].shape else {
            panic!("fixture field should resolve to an array");
        };
        assert_eq!(length, "8");
    }

    #[test]
    fn vendored_headers_build_precision_abi_inventories() {
        let include_dir = std::env::var_os("BOXDD_C_API_TEST_INCLUDE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../boxdd-sys/third-party/box2d/include/box2d")
            });
        for precision in [CAbiPrecision::Single, CAbiPrecision::Double] {
            let inventory = parse_headers_for_precision(&include_dir, precision)
                .unwrap_or_else(|error| panic!("{precision:?} inventory failed: {error}"));
            assert!(!inventory.structs.is_empty());
            assert!(!inventory.functions.is_empty());
            assert!(!inventory.callbacks.is_empty());
        }
    }

    #[test]
    fn vendored_single_function_abi_matches_generated_bindings() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let inventory = parse_headers_for_precision(
            &workspace.join("boxdd-sys/third-party/box2d/include/box2d"),
            CAbiPrecision::Single,
        )
        .expect("single C ABI inventory should parse");
        let rust = crate::sys_abi_index::index_bindings(
            &workspace.join("boxdd-sys/src/bindings_pregenerated.rs"),
        )
        .expect("pregenerated bindings should index");
        for function in &inventory.functions {
            let path = format!("boxdd_sys::ffi::{}", function.name);
            let rust_shape = rust
                .function_abi_shape(&path)
                .unwrap_or_else(|error| panic!("{} failed to index: {error}", function.name))
                .unwrap_or_else(|| panic!("{} is absent from bindings", function.name));
            assert_eq!(
                function.fingerprint,
                rust_shape.fingerprint(),
                "{} differs:\nC: {:#?}\nRust: {:#?}",
                function.name,
                function.shape,
                rust_shape
            );
        }
    }
}
