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
            "if" => conditions.push(classify_preprocessor_condition(argument)),
            "ifdef" => conditions.push(classify_ifdef_condition(argument, false)),
            "ifndef" => conditions.push(classify_ifdef_condition(argument, true)),
            "elif" => {
                if conditions.pop().is_none() {
                    return Err(Error::message(format!(
                        "{header}:{line_number}: unmatched #elif"
                    )));
                }
                conditions.push(classify_preprocessor_condition(argument));
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
}
