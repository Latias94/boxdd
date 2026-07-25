use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Debug)]
struct Token {
    kind: TokenKind,
    line: usize,
    start: usize,
    end: usize,
}

#[derive(Clone, Debug)]
enum TokenKind {
    Identifier(String),
    Literal,
    Punctuation(char),
}

impl Token {
    fn identifier(&self) -> Option<&str> {
        match &self.kind {
            TokenKind::Identifier(identifier) => Some(identifier),
            TokenKind::Literal | TokenKind::Punctuation(_) => None,
        }
    }

    fn is_punctuation(&self, expected: char) -> bool {
        matches!(self.kind, TokenKind::Punctuation(actual) if actual == expected)
    }
}

#[derive(Clone, Debug)]
struct Function {
    source: String,
    line: usize,
    body: Vec<Token>,
}

#[derive(Clone, Debug)]
struct MacroDefinition {
    source: String,
    line: usize,
    function_like: bool,
    replacement: Vec<Token>,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct AuditReport {
    pub(crate) reachable_functions: BTreeSet<String>,
    pub(crate) native_calls: BTreeSet<String>,
    pub(crate) library_calls: BTreeSet<String>,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct ConstantMacroReport {
    pub(crate) macros: BTreeSet<String>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum IncludeScope {
    File,
    FunctionBody,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum IncludeStyle {
    Quoted,
    System,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct IncludeDirective {
    source: String,
    scope: IncludeScope,
    style: IncludeStyle,
    target: String,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct IncludeReport {
    pub(crate) body_quoted: BTreeSet<(String, String)>,
}

pub(crate) fn audit_include_inventory(
    sources: &[(&str, &str)],
    expected_file_quoted: &[(&str, &str)],
    expected_file_system: &[(&str, &str)],
    expected_body_quoted: &[(&str, &str)],
) -> Result<IncludeReport, String> {
    let mut actual = BTreeSet::new();
    for (path, source) in sources {
        for include in parse_includes(path, source)? {
            if !actual.insert(include.clone()) {
                return Err(format!(
                    "duplicate include `{}` in {}",
                    include.target, include.source
                ));
            }
        }
    }

    let expected = expected_file_quoted
        .iter()
        .map(|(source, target)| include(source, IncludeScope::File, IncludeStyle::Quoted, target))
        .chain(expected_file_system.iter().map(|(source, target)| {
            include(source, IncludeScope::File, IncludeStyle::System, target)
        }))
        .chain(expected_body_quoted.iter().map(|(source, target)| {
            include(
                source,
                IncludeScope::FunctionBody,
                IncludeStyle::Quoted,
                target,
            )
        }))
        .collect::<BTreeSet<_>>();
    if actual != expected {
        let unexpected = actual.difference(&expected).cloned().collect::<Vec<_>>();
        let missing = expected.difference(&actual).cloned().collect::<Vec<_>>();
        return Err(format!(
            "C include inventory drifted; unexpected={unexpected:?}, missing={missing:?}"
        ));
    }

    let body_quoted = actual
        .into_iter()
        .filter(|include| {
            include.scope == IncludeScope::FunctionBody && include.style == IncludeStyle::Quoted
        })
        .map(|include| (include.source, include.target))
        .collect();
    Ok(IncludeReport { body_quoted })
}

fn include(
    source: &str,
    scope: IncludeScope,
    style: IncludeStyle,
    target: &str,
) -> IncludeDirective {
    IncludeDirective {
        source: source.to_owned(),
        scope,
        style,
        target: target.to_owned(),
    }
}

pub(crate) fn audit_pure_call_closure(
    sources: &[(&str, &str)],
    roots: &[&str],
    allowed_native_calls: &[&str],
    allowed_library_calls: &[&str],
    allowed_function_macros: &[&str],
) -> Result<AuditReport, String> {
    let mut functions = BTreeMap::new();
    let mut macros = BTreeMap::new();
    for (path, source) in sources {
        for (name, function) in parse_functions(path, source)? {
            if let Some(previous) = functions.insert(name.clone(), function) {
                return Err(format!(
                    "duplicate C function `{name}` in {} and {}",
                    previous.source, path
                ));
            }
        }
        for (name, definition) in parse_macros(path, source)? {
            macros.insert(((*path).to_owned(), name), definition);
        }
    }

    let allowed_native_calls = string_set(allowed_native_calls);
    let allowed_library_calls = string_set(allowed_library_calls);
    let allowed_function_macros = string_set(allowed_function_macros);
    for symbol in allowed_native_calls
        .iter()
        .chain(allowed_library_calls.iter())
    {
        if let Some(function) = functions.get(symbol) {
            return Err(format!(
                "audited external symbol `{symbol}` is shadowed by a definition at {}:{}",
                function.source, function.line
            ));
        }
    }
    for ((_, name), definition) in &macros {
        if allowed_native_calls.contains(name) || allowed_library_calls.contains(name) {
            return Err(format!(
                "audited external symbol `{name}` is shadowed by a macro at {}:{}",
                definition.source, definition.line
            ));
        }
        if definition.function_like && !allowed_function_macros.contains(name) {
            return Err(format!(
                "unapproved function-like macro `{name}` at {}:{}",
                definition.source, definition.line
            ));
        }
    }

    let mut queue = VecDeque::new();
    for root in roots {
        if !functions.contains_key(*root) {
            return Err(format!("audited C root `{root}` has no definition"));
        }
        queue.push_back((*root).to_owned());
    }

    let mut report = AuditReport {
        reachable_functions: BTreeSet::new(),
        native_calls: BTreeSet::new(),
        library_calls: BTreeSet::new(),
    };
    for ((_, name), definition) in &macros {
        audit_macro_replacement(
            name,
            definition,
            &functions,
            &allowed_native_calls,
            &allowed_library_calls,
            &mut queue,
            &mut report,
        )?;
    }
    while let Some(name) = queue.pop_front() {
        if !report.reachable_functions.insert(name.clone()) {
            continue;
        }
        let function = &functions[&name];
        for call in calls_in(function)? {
            if functions.contains_key(&call.name) {
                queue.push_back(call.name);
            } else if allowed_native_calls.contains(&call.name) {
                report.native_calls.insert(call.name);
            } else if allowed_library_calls.contains(&call.name) {
                report.library_calls.insert(call.name);
            } else {
                return Err(format!(
                    "unapproved external call `{}` is reachable from `{name}` at {}:{}",
                    call.name, function.source, call.line
                ));
            }
        }
    }

    Ok(report)
}

fn audit_macro_replacement(
    name: &str,
    definition: &MacroDefinition,
    functions: &BTreeMap<String, Function>,
    allowed_native_calls: &BTreeSet<String>,
    allowed_library_calls: &BTreeSet<String>,
    queue: &mut VecDeque<String>,
    report: &mut AuditReport,
) -> Result<(), String> {
    if contains_preprocessor_construction(&definition.replacement) {
        return Err(format!(
            "macro `{name}` uses stringize or token-paste at {}:{}",
            definition.source, definition.line
        ));
    }
    if let Some(alias) = unwrapped_single_identifier(&definition.replacement) {
        return Err(format!(
            "macro `{name}` is an unauditable identifier alias to `{alias}` at {}:{}",
            definition.source, definition.line
        ));
    }

    let calls = calls_in_tokens(&definition.source, &definition.replacement)?;
    if !definition.function_like && !calls.is_empty() {
        return Err(format!(
            "object-like macro `{name}` contains a call at {}:{}",
            definition.source, definition.line
        ));
    }
    for call in calls {
        if functions.contains_key(&call.name) {
            queue.push_back(call.name);
        } else if allowed_native_calls.contains(&call.name) {
            report.native_calls.insert(call.name);
        } else if allowed_library_calls.contains(&call.name) {
            report.library_calls.insert(call.name);
        } else {
            return Err(format!(
                "unapproved call `{}` occurs in macro `{name}` at {}:{}",
                call.name, definition.source, call.line
            ));
        }
    }

    for (index, token) in definition.replacement.iter().enumerate() {
        let Some(identifier) = token.identifier() else {
            continue;
        };
        let is_call = definition
            .replacement
            .get(index + 1)
            .is_some_and(|next| next.is_punctuation('('));
        if !is_call
            && (functions.contains_key(identifier)
                || allowed_native_calls.contains(identifier)
                || allowed_library_calls.contains(identifier)
                || identifier.starts_with("b2")
                || identifier.starts_with("boxdd"))
        {
            return Err(format!(
                "macro `{name}` contains a bare function-like symbol `{identifier}` at {}:{}",
                definition.source, token.line
            ));
        }
    }
    Ok(())
}

fn contains_preprocessor_construction(tokens: &[Token]) -> bool {
    tokens.iter().any(|token| token.is_punctuation('#'))
        || tokens.windows(2).any(|pair| {
            (pair[0].is_punctuation('%') && pair[1].is_punctuation(':'))
                || (pair[0].is_punctuation('?') && pair[1].is_punctuation('?'))
        })
}

pub(crate) fn audit_constant_macro_invocations(
    path: &str,
    source: &str,
    allowed_macros: &[&str],
    allowed_inner_operators: &[&str],
) -> Result<ConstantMacroReport, String> {
    let tokens = lex(path, source)?;
    let allowed_macros = string_set(allowed_macros);
    let allowed_inner_operators = string_set(allowed_inner_operators);
    let mut used_macros = BTreeSet::new();
    let mut index = 0;

    while index < tokens.len() {
        let Some(name) = tokens[index].identifier() else {
            return Err(format!(
                "expected a constant macro invocation at {path}:{}",
                tokens[index].line
            ));
        };
        if !allowed_macros.contains(name) {
            return Err(format!(
                "unapproved top-level macro `{name}` at {path}:{}",
                tokens[index].line
            ));
        }
        if !tokens
            .get(index + 1)
            .is_some_and(|token| token.is_punctuation('('))
        {
            return Err(format!(
                "constant macro `{name}` is not followed by `(` at {path}:{}",
                tokens[index].line
            ));
        }
        let close = matching_delimiter(&tokens, index + 1, '(', ')').ok_or_else(|| {
            format!(
                "unterminated constant macro `{name}` at {path}:{}",
                tokens[index].line
            )
        })?;
        for nested in index + 2..close {
            let Some(nested_name) = tokens[nested].identifier() else {
                continue;
            };
            if tokens
                .get(nested + 1)
                .is_some_and(|token| token.is_punctuation('('))
                && !allowed_inner_operators.contains(nested_name)
            {
                return Err(format!(
                    "unapproved call `{nested_name}` inside `{name}` at {path}:{}",
                    tokens[nested].line
                ));
            }
        }
        used_macros.insert(name.to_owned());
        index = close + 1;
    }

    Ok(ConstantMacroReport {
        macros: used_macros,
    })
}

#[derive(Debug)]
struct Call {
    name: String,
    line: usize,
}

fn calls_in(function: &Function) -> Result<Vec<Call>, String> {
    calls_in_tokens(&function.source, &function.body)
}

fn calls_in_tokens(source: &str, tokens: &[Token]) -> Result<Vec<Call>, String> {
    let mut calls = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if let Some(name) = token.identifier()
            && tokens
                .get(index + 1)
                .is_some_and(|next| next.is_punctuation('('))
        {
            if is_language_construct(name) || is_macro_definition_name(tokens, index) {
                continue;
            }
            calls.push(Call {
                name: name.to_owned(),
                line: token.line,
            });
            continue;
        }

        if (token.is_punctuation(')') || token.is_punctuation(']'))
            && tokens
                .get(index + 1)
                .is_some_and(|next| next.is_punctuation('('))
            && !(token.is_punctuation(')') && is_scalar_cast(tokens, index))
        {
            return Err(format!(
                "indirect C call is reachable from a pure adapter root at {source}:{}",
                token.line
            ));
        }
    }
    Ok(calls)
}

fn unwrapped_single_identifier(tokens: &[Token]) -> Option<&str> {
    let mut start = 0;
    let mut end = tokens.len();
    while end.saturating_sub(start) >= 3
        && tokens[start].is_punctuation('(')
        && matching_delimiter(tokens, start, '(', ')') == Some(end - 1)
    {
        start += 1;
        end -= 1;
    }
    (end == start + 1)
        .then(|| tokens[start].identifier())
        .flatten()
}

fn is_language_construct(name: &str) -> bool {
    matches!(
        name,
        "if" | "for"
            | "while"
            | "switch"
            | "sizeof"
            | "_Alignof"
            | "alignof"
            | "offsetof"
            | "_Generic"
            | "typeof"
            | "__typeof__"
            | "_Static_assert"
    )
}

fn is_macro_definition_name(tokens: &[Token], index: usize) -> bool {
    index >= 2
        && tokens[index - 2].is_punctuation('#')
        && tokens[index - 1].identifier() == Some("define")
}

fn is_scalar_cast(tokens: &[Token], close: usize) -> bool {
    let Some(open) = matching_open(tokens, close, '(', ')') else {
        return false;
    };
    let inner = &tokens[open + 1..close];
    inner.len() == 1
        && inner[0].identifier().is_some_and(|name| {
            matches!(
                name,
                "int8_t"
                    | "uint8_t"
                    | "int16_t"
                    | "uint16_t"
                    | "int32_t"
                    | "uint32_t"
                    | "int64_t"
                    | "uint64_t"
                    | "size_t"
                    | "float"
                    | "double"
                    | "bool"
            )
        })
}

fn parse_includes(path: &str, source: &str) -> Result<Vec<IncludeDirective>, String> {
    let tokens = lex(path, source)?;
    let continued_lines = continued_source_lines(source);
    let mut includes = Vec::new();
    let mut index = 0;
    let mut brace_depth = 0usize;

    while index < tokens.len() {
        if tokens[index].is_punctuation('{') {
            brace_depth += 1;
            index += 1;
            continue;
        }
        if tokens[index].is_punctuation('}') {
            brace_depth = brace_depth.checked_sub(1).ok_or_else(|| {
                format!(
                    "unmatched `}}` while auditing includes at {path}:{}",
                    tokens[index].line
                )
            })?;
            index += 1;
            continue;
        }
        if !tokens[index].is_punctuation('#')
            || !is_first_token_on_line(&tokens, index)
            || tokens.get(index + 1).and_then(Token::identifier) != Some("include")
        {
            index += 1;
            continue;
        }

        let start_line = tokens[index].line;
        let end_line = logical_directive_end_line(start_line, &continued_lines);
        if end_line != start_line {
            return Err(format!(
                "continued include directives are forbidden at {path}:{start_line}"
            ));
        }
        let directive_end = directive_end(&tokens, index, end_line);
        let (style, target) =
            parse_include_target(path, source, start_line, &tokens[index + 2..directive_end])?;
        includes.push(IncludeDirective {
            source: path.to_owned(),
            scope: if brace_depth == 0 {
                IncludeScope::File
            } else {
                IncludeScope::FunctionBody
            },
            style,
            target,
        });
        index = directive_end;
    }

    if brace_depth != 0 {
        return Err(format!("unclosed brace while auditing includes in {path}"));
    }
    Ok(includes)
}

fn parse_include_target(
    path: &str,
    source: &str,
    line: usize,
    payload: &[Token],
) -> Result<(IncludeStyle, String), String> {
    if payload.len() == 1 {
        let raw = &source[payload[0].start..payload[0].end];
        if raw.starts_with('"') && raw.ends_with('"') && raw.len() >= 2 {
            let target = &raw[1..raw.len() - 1];
            validate_include_target(path, line, target)?;
            return Ok((IncludeStyle::Quoted, target.to_owned()));
        }
    }
    if payload.len() >= 3
        && payload[0].is_punctuation('<')
        && payload[payload.len() - 1].is_punctuation('>')
    {
        let target = &source[payload[0].end..payload[payload.len() - 1].start];
        validate_include_target(path, line, target)?;
        return Ok((IncludeStyle::System, target.to_owned()));
    }
    Err(format!(
        "include target must be one literal header name at {path}:{line}; macro and concatenated includes are forbidden"
    ))
}

fn validate_include_target(path: &str, line: usize, target: &str) -> Result<(), String> {
    if target.is_empty()
        || !target.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'+' | b'.' | b'/')
        })
    {
        return Err(format!(
            "include target `{target}` contains unsupported syntax at {path}:{line}"
        ));
    }
    Ok(())
}

fn parse_macros(path: &str, source: &str) -> Result<BTreeMap<String, MacroDefinition>, String> {
    let tokens = lex(path, source)?;
    let continued_lines = continued_source_lines(source);
    let mut macros = BTreeMap::new();
    let mut index = 0;

    while index + 2 < tokens.len() {
        if !tokens[index].is_punctuation('#')
            || !is_first_token_on_line(&tokens, index)
            || tokens[index + 1].identifier() != Some("define")
        {
            index += 1;
            continue;
        }
        let Some(name) = tokens[index + 2].identifier() else {
            return Err(format!(
                "macro definition has no identifier at {path}:{}",
                tokens[index].line
            ));
        };
        let end_line = logical_directive_end_line(tokens[index].line, &continued_lines);
        let directive_end = directive_end(&tokens, index, end_line);

        let name_index = index + 2;
        let function_like = tokens.get(name_index + 1).is_some_and(|next| {
            next.is_punctuation('(')
                && tokens_are_adjacent_after_line_splicing(source, &tokens[name_index], next)
        });
        let replacement_start = if function_like {
            let parameters_end =
                matching_delimiter(&tokens, name_index + 1, '(', ')').ok_or_else(|| {
                    format!(
                        "unterminated macro parameter list for `{name}` at {path}:{}",
                        tokens[index].line
                    )
                })?;
            if parameters_end >= directive_end {
                return Err(format!(
                    "macro parameter list for `{name}` crosses its directive at {path}:{}",
                    tokens[index].line
                ));
            }
            parameters_end + 1
        } else {
            name_index + 1
        };
        let replacement = tokens[replacement_start..directive_end]
            .iter()
            .filter(|token| !(token.is_punctuation('\\') && continued_lines.contains(&token.line)))
            .cloned()
            .collect();
        let definition = MacroDefinition {
            source: path.to_owned(),
            line: tokens[index].line,
            function_like,
            replacement,
        };
        if let Some(previous) = macros.insert(name.to_owned(), definition) {
            return Err(format!(
                "duplicate macro `{name}` at {}:{} and {path}:{}",
                previous.source, previous.line, tokens[index].line
            ));
        }
        index = directive_end;
    }
    Ok(macros)
}

fn continued_source_lines(source: &str) -> BTreeSet<usize> {
    source
        .lines()
        .enumerate()
        .filter_map(|(index, line)| line.trim_end().ends_with('\\').then_some(index + 1))
        .collect()
}

fn logical_directive_end_line(start: usize, continued_lines: &BTreeSet<usize>) -> usize {
    let mut end = start;
    while continued_lines.contains(&end) {
        end += 1;
    }
    end
}

fn directive_end(tokens: &[Token], start: usize, end_line: usize) -> usize {
    tokens[start..]
        .iter()
        .position(|token| token.line > end_line)
        .map_or(tokens.len(), |offset| start + offset)
}

fn is_first_token_on_line(tokens: &[Token], index: usize) -> bool {
    index == 0 || tokens[index - 1].line < tokens[index].line
}

fn tokens_are_adjacent_after_line_splicing(source: &str, left: &Token, right: &Token) -> bool {
    let bytes = source.as_bytes();
    let mut index = left.end;
    while index < right.start {
        let Some(length) = line_splice_len(bytes, index) else {
            return false;
        };
        index += length;
    }
    index == right.start
}

fn parse_functions(path: &str, source: &str) -> Result<BTreeMap<String, Function>, String> {
    let tokens = lex(path, source)?;
    let mut functions = BTreeMap::new();
    let mut index = 0;
    let mut depth = 0usize;

    while index < tokens.len() {
        if tokens[index].is_punctuation('{') {
            depth += 1;
            index += 1;
            continue;
        }
        if tokens[index].is_punctuation('}') {
            depth = depth
                .checked_sub(1)
                .ok_or_else(|| format!("unmatched `}}` at {path}:{}", tokens[index].line))?;
            index += 1;
            continue;
        }
        if depth != 0 {
            index += 1;
            continue;
        }

        let Some(name) = tokens[index].identifier() else {
            index += 1;
            continue;
        };
        if !tokens
            .get(index + 1)
            .is_some_and(|token| token.is_punctuation('('))
        {
            index += 1;
            continue;
        }
        let Some(parameters_end) = matching_delimiter(&tokens, index + 1, '(', ')') else {
            return Err(format!(
                "unterminated parameter list after `{name}` at {path}:{}",
                tokens[index].line
            ));
        };
        if !tokens
            .get(parameters_end + 1)
            .is_some_and(|token| token.is_punctuation('{'))
        {
            index += 1;
            continue;
        }
        let body_start = parameters_end + 1;
        let Some(body_end) = matching_delimiter(&tokens, body_start, '{', '}') else {
            return Err(format!(
                "unterminated body for `{name}` at {path}:{}",
                tokens[index].line
            ));
        };
        let function = Function {
            source: path.to_owned(),
            line: tokens[index].line,
            body: tokens[body_start + 1..body_end].to_vec(),
        };
        if let Some(previous) = functions.insert(name.to_owned(), function) {
            return Err(format!(
                "duplicate C function `{name}` at {}:{} and {path}:{}",
                previous.source, previous.line, tokens[index].line
            ));
        }
        index = body_end + 1;
    }

    if depth != 0 {
        return Err(format!("unclosed global brace in {path}"));
    }
    Ok(functions)
}

fn matching_delimiter(
    tokens: &[Token],
    open: usize,
    open_punctuation: char,
    close_punctuation: char,
) -> Option<usize> {
    if !tokens.get(open)?.is_punctuation(open_punctuation) {
        return None;
    }
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(open) {
        if token.is_punctuation(open_punctuation) {
            depth += 1;
        } else if token.is_punctuation(close_punctuation) {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

fn matching_open(
    tokens: &[Token],
    close: usize,
    open_punctuation: char,
    close_punctuation: char,
) -> Option<usize> {
    if !tokens.get(close)?.is_punctuation(close_punctuation) {
        return None;
    }
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().take(close + 1).rev() {
        if token.is_punctuation(close_punctuation) {
            depth += 1;
        } else if token.is_punctuation(open_punctuation) {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

fn lex(path: &str, source: &str) -> Result<Vec<Token>, String> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    let mut line = 1;

    while index < bytes.len() {
        match bytes[index] {
            b'\\' if line_splice_len(bytes, index).is_some() => {
                index += line_splice_len(bytes, index).expect("matched line splice");
                line += 1;
            }
            b'\n' => {
                line += 1;
                index += 1;
            }
            byte if byte.is_ascii_whitespace() => index += 1,
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                index += 2;
                while index < bytes.len() && bytes[index] != b'\n' {
                    index += 1;
                }
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                let start_line = line;
                index += 2;
                let mut terminated = false;
                while index < bytes.len() {
                    if bytes[index] == b'\n' {
                        line += 1;
                    }
                    if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                        index += 2;
                        terminated = true;
                        break;
                    }
                    index += 1;
                }
                if !terminated {
                    return Err(format!("unterminated block comment at {path}:{start_line}"));
                }
            }
            quote @ (b'\'' | b'"') => {
                let start = index;
                let start_line = line;
                index += 1;
                let mut terminated = false;
                while index < bytes.len() {
                    if bytes[index] == b'\\' {
                        index += 1;
                        if index < bytes.len() && bytes[index] == b'\n' {
                            line += 1;
                        }
                        index += usize::from(index < bytes.len());
                        continue;
                    }
                    if bytes[index] == quote {
                        index += 1;
                        terminated = true;
                        break;
                    }
                    if bytes[index] == b'\n' {
                        line += 1;
                    }
                    index += 1;
                }
                if !terminated {
                    return Err(format!("unterminated literal at {path}:{start_line}"));
                }
                tokens.push(Token {
                    kind: TokenKind::Literal,
                    line: start_line,
                    start,
                    end: index,
                });
            }
            byte if is_identifier_start(byte) => {
                let start = index;
                let start_line = line;
                let mut identifier = String::new();
                index += 1;
                identifier.push(char::from(byte));
                while index < bytes.len() {
                    if is_identifier_continue(bytes[index]) {
                        identifier.push(char::from(bytes[index]));
                        index += 1;
                    } else if let Some(length) = line_splice_len(bytes, index) {
                        index += length;
                        line += 1;
                    } else {
                        break;
                    }
                }
                tokens.push(Token {
                    kind: TokenKind::Identifier(identifier),
                    line: start_line,
                    start,
                    end: index,
                });
            }
            byte if byte.is_ascii_digit() => {
                let start = index;
                index += 1;
                while index < bytes.len()
                    && (bytes[index].is_ascii_alphanumeric() || matches!(bytes[index], b'_' | b'.'))
                {
                    index += 1;
                }
                tokens.push(Token {
                    kind: TokenKind::Literal,
                    line,
                    start,
                    end: index,
                });
            }
            byte if byte.is_ascii() => {
                tokens.push(Token {
                    kind: TokenKind::Punctuation(char::from(byte)),
                    line,
                    start: index,
                    end: index + 1,
                });
                index += 1;
            }
            _ => return Err(format!("non-ASCII C source byte at {path}:{line}")),
        }
    }

    Ok(tokens)
}

fn is_identifier_start(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphabetic()
}

fn is_identifier_continue(byte: u8) -> bool {
    is_identifier_start(byte) || byte.is_ascii_digit()
}

fn line_splice_len(bytes: &[u8], index: usize) -> Option<usize> {
    if bytes.get(index) != Some(&b'\\') {
        return None;
    }
    if bytes.get(index + 1) == Some(&b'\n') {
        Some(2)
    } else if bytes.get(index + 1) == Some(&b'\r') && bytes.get(index + 2) == Some(&b'\n') {
        Some(3)
    } else {
        None
    }
}

fn string_set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOTS: &[&str] = &["root"];
    const NATIVE: &[&str] = &["b2IsDoublePrecision"];
    const LIBRARY: &[&str] = &["memcpy"];

    #[test]
    fn comments_and_literals_cannot_forge_call_edges() {
        let source = r#"
            void helper(void) { b2IsDoublePrecision(); }
            void root(void) {
                // b2World_Step();
                const char* text = "b2SetLengthUnitsPerMeter()";
                /* b2Alloc(); */
                helper();
                memcpy(0, 0, 0);
            }
        "#;
        let report =
            audit_pure_call_closure(&[("fixture.c", source)], ROOTS, NATIVE, LIBRARY, &[]).unwrap();
        assert_eq!(report.native_calls, string_set(NATIVE));
        assert_eq!(report.library_calls, string_set(LIBRARY));
    }

    #[test]
    fn transitive_native_mutation_fails_closed() {
        let source = r#"
            void mutate(void) { b2SetLengthUnitsPerMeter(2.0f); }
            void root(void) { mutate(); }
        "#;
        let error = audit_pure_call_closure(&[("fixture.c", source)], ROOTS, NATIVE, LIBRARY, &[])
            .unwrap_err();
        assert!(error.contains("b2SetLengthUnitsPerMeter"));
    }

    #[test]
    fn unreachable_native_mutation_is_outside_the_root_closure() {
        let source = r#"
            void mutate(void) { b2SetLengthUnitsPerMeter(2.0f); }
            void root(void) { b2IsDoublePrecision(); }
        "#;
        audit_pure_call_closure(&[("fixture.c", source)], ROOTS, NATIVE, LIBRARY, &[]).unwrap();
    }

    #[test]
    fn indirect_calls_fail_closed() {
        let source = "void root(void) { (*callback)(); }";
        let error = audit_pure_call_closure(&[("fixture.c", source)], ROOTS, NATIVE, LIBRARY, &[])
            .unwrap_err();
        assert!(error.contains("indirect C call"));
    }

    #[test]
    fn an_allowlisted_external_cannot_be_shadowed() {
        let source = r#"
            bool b2IsDoublePrecision(void) { return true; }
            void root(void) { b2IsDoublePrecision(); }
        "#;
        let error = audit_pure_call_closure(&[("fixture.c", source)], ROOTS, NATIVE, LIBRARY, &[])
            .unwrap_err();
        assert!(error.contains("shadowed"));
    }

    #[test]
    fn object_like_macros_cannot_hide_calls_or_function_aliases() {
        let call = r#"
            #define MUTATE b2World_Step(world, 1.0f, 4)
            void root(void) { MUTATE; }
        "#;
        let error = audit_pure_call_closure(&[("fixture.c", call)], ROOTS, NATIVE, LIBRARY, &[])
            .unwrap_err();
        assert!(error.contains("object-like macro `MUTATE` contains a call"));

        let alias = r#"
            #define MUTATE b2World_Step
            void root(void) { MUTATE(world, 1.0f, 4); }
        "#;
        let error = audit_pure_call_closure(&[("fixture.c", alias)], ROOTS, NATIVE, LIBRARY, &[])
            .unwrap_err();
        assert!(error.contains("identifier alias"));

        let spliced =
            "#define MUTATE b2World_\\\nStep(world, 1.0f, 4)\nvoid root(void) { MUTATE; }";
        let error = audit_pure_call_closure(&[("fixture.c", spliced)], ROOTS, NATIVE, LIBRARY, &[])
            .unwrap_err();
        assert!(error.contains("object-like macro `MUTATE` contains a call"));
    }

    #[test]
    fn approved_function_macros_still_audit_their_replacement() {
        let source = r#"
            #define PURE_VALUE(value) b2World_Step(world, value, 4)
            void root(void) { b2IsDoublePrecision(); }
        "#;
        let error = audit_pure_call_closure(
            &[("fixture.c", source)],
            ROOTS,
            NATIVE,
            LIBRARY,
            &["PURE_VALUE"],
        )
        .unwrap_err();
        assert!(error.contains("b2World_Step"));
    }

    #[test]
    fn token_construction_and_external_macro_shadowing_fail_closed() {
        let pasted = r#"
            #define JOIN(left, right) left ## right
            void root(void) { b2IsDoublePrecision(); }
        "#;
        let error =
            audit_pure_call_closure(&[("fixture.c", pasted)], ROOTS, NATIVE, LIBRARY, &["JOIN"])
                .unwrap_err();
        assert!(error.contains("token-paste"));

        let shadowed = r#"
            #define b2IsDoublePrecision() b2World_Step(world, 1.0f, 4)
            void root(void) { b2IsDoublePrecision(); }
        "#;
        let error = audit_pure_call_closure(
            &[("fixture.c", shadowed)],
            ROOTS,
            NATIVE,
            LIBRARY,
            &["b2IsDoublePrecision"],
        )
        .unwrap_err();
        assert!(error.contains("shadowed by a macro"));
    }

    #[test]
    fn numeric_string_and_flag_object_macros_remain_allowed() {
        let source = r#"
            #define BYTE_LIMIT 256u
            #define FLAG_MASK (0x1u | 0x2u)
            #define IDENTITY_TEXT "compile-time identity"
            void root(void) { b2IsDoublePrecision(); }
        "#;
        audit_pure_call_closure(&[("fixture.c", source)], ROOTS, NATIVE, LIBRARY, &[]).unwrap();
    }

    #[test]
    fn include_inventory_rejects_new_concatenated_macro_and_continued_targets() {
        let expected_file = &[("fixture.c", "safe.h")];
        let expected_body = &[("fixture.c", "safe.inl")];
        let baseline = r#"
            #include "safe.h"
            void root(void) {
            #include "safe.inl"
            }
        "#;
        let report = audit_include_inventory(
            &[("fixture.c", baseline)],
            expected_file,
            &[],
            expected_body,
        )
        .unwrap();
        assert_eq!(
            report.body_quoted,
            BTreeSet::from([("fixture.c".to_owned(), "safe.inl".to_owned())])
        );

        let added = r#"
            #include "safe.h"
            void root(void) {
            #include "safe.inl"
            #include "evil.inl"
            }
        "#;
        let error =
            audit_include_inventory(&[("fixture.c", added)], expected_file, &[], expected_body)
                .unwrap_err();
        assert!(error.contains("evil.inl"));

        for malformed in [
            r#"
                #include "safe.h"
                #define HEADER "evil.inl"
                void root(void) {
                #include HEADER
                }
            "#,
            r#"
                #include "safe.h"
                void root(void) {
                #include "evil" ".inl"
                }
            "#,
            "#include \"safe.h\"\nvoid root(void) {\n#include \\\n\"evil.inl\"\n}\n",
        ] {
            let error = audit_include_inventory(
                &[("fixture.c", malformed)],
                expected_file,
                &[],
                expected_body,
            )
            .unwrap_err();
            assert!(
                error.contains("forbidden")
                    || error.contains("one literal")
                    || error.contains("inventory drifted"),
                "unexpected include audit error: {error}"
            );
        }
    }

    #[test]
    fn constant_macro_inputs_reject_nested_calls() {
        audit_constant_macro_invocations(
            "fixture.inl",
            "VALUE(sizeof(Item)) VALUE(CONSTANT)",
            &["VALUE"],
            &["sizeof"],
        )
        .unwrap();
        let error = audit_constant_macro_invocations(
            "fixture.inl",
            "VALUE(b2World_Step(world))",
            &["VALUE"],
            &["sizeof"],
        )
        .unwrap_err();
        assert!(error.contains("b2World_Step"));
    }
}
