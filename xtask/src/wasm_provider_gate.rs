//! Structural qualification for the partitioned-memory browser provider route.

use std::collections::{BTreeMap, BTreeSet};

use wasmparser::{DataKind, ExternalKind, FuncType, Operator, Parser, Payload, TypeRef, ValType};

use crate::wasm_provider_memory::{
    CONSUMER_GLOBAL_BASE_BYTES, INITIAL_MEMORY_BYTES, INITIAL_MEMORY_PAGES, MAXIMUM_MEMORY_PAGES,
    PROVIDER_HEAP_LIMIT_BYTES, PROVIDER_STATIC_BASE_BYTES,
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct ObservedConsumerContract {
    memory_imports: u32,
    matching_memory_imports: u32,
    defined_memories: u32,
    provider_modules: BTreeSet<String>,
    data_end_exports: u32,
    data_end: Option<u64>,
    stack_low_exports: u32,
    stack_low: Option<u64>,
    stack_high_exports: u32,
    stack_high: Option<u64>,
    heap_base_exports: u32,
    heap_base: Option<u64>,
    heap_end_exports: u32,
    heap_end: Option<u64>,
}

impl ObservedConsumerContract {
    #[cfg(test)]
    fn closed(provider_module: &str) -> Self {
        Self {
            memory_imports: 1,
            matching_memory_imports: 1,
            defined_memories: 0,
            provider_modules: BTreeSet::from([provider_module.to_owned()]),
            data_end_exports: 1,
            data_end: Some(CONSUMER_GLOBAL_BASE_BYTES + 32 * 1024),
            stack_low_exports: 1,
            stack_low: Some(CONSUMER_GLOBAL_BASE_BYTES + 64 * 1024),
            stack_high_exports: 1,
            stack_high: Some(CONSUMER_GLOBAL_BASE_BYTES + 128 * 1024),
            heap_base_exports: 1,
            heap_base: Some(CONSUMER_GLOBAL_BASE_BYTES + 128 * 1024),
            heap_end_exports: 1,
            heap_end: Some(INITIAL_MEMORY_BYTES),
        }
    }

    fn validate(&self, provider_module: &str, memory_module: &str) -> Result<(), String> {
        if self.memory_imports != 1
            || self.matching_memory_imports != 1
            || self.defined_memories != 0
        {
            return Err(format!(
                "consumer must import exactly one {memory_module}.memory with min={INITIAL_MEMORY_PAGES}, max={MAXIMUM_MEMORY_PAGES}, found {self:?}"
            ));
        }
        if self.provider_modules != BTreeSet::from([provider_module.to_owned()]) {
            return Err(format!(
                "consumer provider module set must be exactly {provider_module:?}, found {:?}",
                self.provider_modules
            ));
        }
        let layout = (
            self.data_end,
            self.stack_low,
            self.stack_high,
            self.heap_base,
            self.heap_end,
        );
        if self.data_end_exports != 1
            || self.stack_low_exports != 1
            || self.stack_high_exports != 1
            || self.heap_base_exports != 1
            || self.heap_end_exports != 1
            || !matches!(
                layout,
                (
                    Some(data_end),
                    Some(stack_low),
                    Some(stack_high),
                    Some(heap_base),
                    Some(heap_end),
                )
                    if data_end >= CONSUMER_GLOBAL_BASE_BYTES
                        && data_end <= stack_low
                        && stack_low < stack_high
                        && stack_high <= heap_base
                        && heap_base < heap_end
                        && heap_end == INITIAL_MEMORY_BYTES
            )
        {
            return Err(format!(
                "consumer must export an ordered high partition __data_end <= __stack_low < __stack_high <= __heap_base < __heap_end in [{CONSUMER_GLOBAL_BASE_BYTES}, {INITIAL_MEMORY_BYTES}] with __heap_end equal to the initial memory size, found {layout:?}"
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
struct ObservedGlobal {
    immutable_i32_value: Option<u64>,
}

pub(crate) fn validate_consumer(
    bytes: &[u8],
    provider_module: &str,
    memory_module: &str,
) -> Result<(), String> {
    let mut observed = ObservedConsumerContract {
        memory_imports: 0,
        matching_memory_imports: 0,
        defined_memories: 0,
        provider_modules: BTreeSet::new(),
        data_end_exports: 0,
        data_end: None,
        stack_low_exports: 0,
        stack_low: None,
        stack_high_exports: 0,
        stack_high: None,
        heap_base_exports: 0,
        heap_base: None,
        heap_end_exports: 0,
        heap_end: None,
    };
    let mut globals = Vec::<ObservedGlobal>::new();
    let mut data_end_index = None;
    let mut stack_low_index = None;
    let mut stack_high_index = None;
    let mut heap_base_index = None;
    let mut heap_end_index = None;
    let mut data_ranges = Vec::new();

    for payload in Parser::new(0).parse_all(bytes) {
        match payload.map_err(|error| format!("failed to parse consumer Wasm: {error}"))? {
            Payload::ImportSection(section) => {
                for import in section.into_imports() {
                    let import = import
                        .map_err(|error| format!("failed to parse consumer import: {error}"))?;
                    match import.ty {
                        TypeRef::Memory(memory) => {
                            observed.memory_imports += 1;
                            if import.module == memory_module
                                && import.name == "memory"
                                && memory.initial == INITIAL_MEMORY_PAGES
                                && memory.maximum == Some(MAXIMUM_MEMORY_PAGES)
                                && !memory.memory64
                                && !memory.shared
                                && memory.page_size_log2.is_none()
                            {
                                observed.matching_memory_imports += 1;
                            }
                        }
                        TypeRef::Global(_) => globals.push(ObservedGlobal {
                            immutable_i32_value: None,
                        }),
                        TypeRef::Func(_) | TypeRef::FuncExact(_)
                            if import.module.starts_with("box2d-sys-") =>
                        {
                            observed.provider_modules.insert(import.module.to_owned());
                        }
                        _ => {}
                    }
                }
            }
            Payload::MemorySection(section) => {
                observed.defined_memories = observed
                    .defined_memories
                    .checked_add(section.count())
                    .ok_or_else(|| "consumer memory count overflow".to_owned())?;
            }
            Payload::GlobalSection(section) => {
                collect_globals(section, &mut globals, "consumer")?;
            }
            Payload::ExportSection(section) => {
                for export in section {
                    let export = export
                        .map_err(|error| format!("failed to parse consumer export: {error}"))?;
                    if export.kind == ExternalKind::Global {
                        match export.name {
                            "__data_end" => {
                                observed.data_end_exports += 1;
                                data_end_index = Some(export.index);
                            }
                            "__stack_low" => {
                                observed.stack_low_exports += 1;
                                stack_low_index = Some(export.index);
                            }
                            "__stack_high" => {
                                observed.stack_high_exports += 1;
                                stack_high_index = Some(export.index);
                            }
                            "__heap_base" => {
                                observed.heap_base_exports += 1;
                                heap_base_index = Some(export.index);
                            }
                            "__heap_end" => {
                                observed.heap_end_exports += 1;
                                heap_end_index = Some(export.index);
                            }
                            _ => {}
                        }
                    }
                }
            }
            Payload::DataSection(section) => {
                for (index, data) in section.into_iter().enumerate() {
                    let data =
                        data.map_err(|error| format!("failed to parse consumer data: {error}"))?;
                    let (offset, end) =
                        active_data_range(data.kind, data.data.len(), "consumer", index)?;
                    if offset < CONSUMER_GLOBAL_BASE_BYTES {
                        return Err(format!(
                            "consumer active data segment {index} starts at {offset}, below the Rust partition base {CONSUMER_GLOBAL_BASE_BYTES}"
                        ));
                    }
                    data_ranges.push((index, (offset, end)));
                }
            }
            _ => {}
        }
    }

    observed.data_end = exported_global_value(data_end_index, &globals);
    observed.stack_low = exported_global_value(stack_low_index, &globals);
    observed.stack_high = exported_global_value(stack_high_index, &globals);
    observed.heap_base = exported_global_value(heap_base_index, &globals);
    observed.heap_end = exported_global_value(heap_end_index, &globals);
    observed.validate(provider_module, memory_module)?;
    let data_end = observed.data_end.expect("validated consumer data end");
    for (index, (offset, end)) in data_ranges {
        if end > data_end {
            return Err(format!(
                "consumer active data segment {index} occupies [{offset}, {end}), beyond __data_end {data_end}"
            ));
        }
    }
    Ok(())
}

pub(crate) fn validate_provider(
    bytes: &[u8],
    expected_exports: &BTreeSet<String>,
) -> Result<(), String> {
    let mut types = Vec::<FuncType>::new();
    let mut function_type_indices = Vec::<u32>::new();
    let mut function_exports = BTreeMap::<String, Vec<u32>>::new();
    let mut globals = Vec::<ObservedGlobal>::new();
    let mut data_end_index = None;
    let mut stack_low_index = None;
    let mut stack_high_index = None;
    let mut heap_base_index = None;
    let mut data_end_exports = 0_u32;
    let mut stack_low_exports = 0_u32;
    let mut stack_high_exports = 0_u32;
    let mut heap_base_exports = 0_u32;
    let mut memory_imports = 0_u32;
    let mut matching_memory_imports = 0_u32;
    let mut defined_memories = 0_u32;
    let mut memory_grow_operators = 0_u64;
    let mut resize_heap_imports = Vec::new();
    let mut data_ranges = Vec::new();

    for payload in Parser::new(0).parse_all(bytes) {
        match payload.map_err(|error| format!("failed to parse provider Wasm: {error}"))? {
            Payload::TypeSection(section) => {
                types = section
                    .into_iter_err_on_gc_types()
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|error| format!("failed to parse provider function types: {error}"))?;
            }
            Payload::ImportSection(section) => {
                for import in section.into_imports() {
                    let import = import
                        .map_err(|error| format!("failed to parse provider import: {error}"))?;
                    match import.ty {
                        TypeRef::Func(type_index) | TypeRef::FuncExact(type_index) => {
                            function_type_indices.push(type_index);
                            if import.name.contains("resize_heap") {
                                resize_heap_imports
                                    .push(format!("{}.{}", import.module, import.name));
                            }
                        }
                        TypeRef::Memory(memory) => {
                            memory_imports += 1;
                            if import.module == "env"
                                && import.name == "memory"
                                && memory.initial == INITIAL_MEMORY_PAGES
                                && memory.maximum == Some(MAXIMUM_MEMORY_PAGES)
                                && !memory.memory64
                                && !memory.shared
                                && memory.page_size_log2.is_none()
                            {
                                matching_memory_imports += 1;
                            }
                        }
                        TypeRef::Global(_) => globals.push(ObservedGlobal {
                            immutable_i32_value: None,
                        }),
                        _ => {}
                    }
                }
            }
            Payload::FunctionSection(section) => {
                for type_index in section {
                    function_type_indices.push(
                        type_index.map_err(|error| {
                            format!("failed to parse provider function: {error}")
                        })?,
                    );
                }
            }
            Payload::MemorySection(section) => {
                defined_memories = defined_memories
                    .checked_add(section.count())
                    .ok_or_else(|| "provider memory count overflow".to_owned())?;
            }
            Payload::GlobalSection(section) => {
                collect_globals(section, &mut globals, "provider")?;
            }
            Payload::ExportSection(section) => {
                for export in section {
                    let export = export
                        .map_err(|error| format!("failed to parse provider export: {error}"))?;
                    if matches!(export.kind, ExternalKind::Func | ExternalKind::FuncExact) {
                        function_exports
                            .entry(export.name.to_owned())
                            .or_default()
                            .push(export.index);
                    } else if export.kind == ExternalKind::Global {
                        match export.name {
                            "__data_end" => {
                                data_end_exports += 1;
                                data_end_index = Some(export.index);
                            }
                            "__stack_low" => {
                                stack_low_exports += 1;
                                stack_low_index = Some(export.index);
                            }
                            "__stack_high" => {
                                stack_high_exports += 1;
                                stack_high_index = Some(export.index);
                            }
                            "__heap_base" => {
                                heap_base_exports += 1;
                                heap_base_index = Some(export.index);
                            }
                            _ => {}
                        }
                    }
                }
            }
            Payload::DataSection(section) => {
                for (index, data) in section.into_iter().enumerate() {
                    let data =
                        data.map_err(|error| format!("failed to parse provider data: {error}"))?;
                    let range = active_data_range(data.kind, data.data.len(), "provider", index)?;
                    data_ranges.push((index, range));
                }
            }
            Payload::CodeSectionEntry(body) => {
                let mut operators = body
                    .get_operators_reader()
                    .map_err(|error| format!("failed to parse provider function body: {error}"))?;
                while !operators.eof() {
                    if matches!(
                        operators.read().map_err(|error| format!(
                            "failed to parse provider operator: {error}"
                        ))?,
                        Operator::MemoryGrow { .. }
                    ) {
                        memory_grow_operators += 1;
                    }
                }
            }
            _ => {}
        }
    }

    if memory_imports != 1 || matching_memory_imports != 1 || defined_memories != 0 {
        return Err(format!(
            "provider must import exactly one env.memory with min={INITIAL_MEMORY_PAGES}, max={MAXIMUM_MEMORY_PAGES}"
        ));
    }
    if !resize_heap_imports.is_empty() || memory_grow_operators != 0 {
        return Err(format!(
            "provider must not grow shared memory: resize imports={resize_heap_imports:?}, memory.grow operators={memory_grow_operators}"
        ));
    }
    let layout = (
        exported_global_value(data_end_index, &globals),
        exported_global_value(stack_low_index, &globals),
        exported_global_value(stack_high_index, &globals),
        exported_global_value(heap_base_index, &globals),
    );
    if data_end_exports != 1
        || stack_low_exports != 1
        || stack_high_exports != 1
        || heap_base_exports != 1
        || !matches!(
            layout,
            (Some(data_end), Some(stack_low), Some(stack_high), Some(heap_base))
                if data_end >= PROVIDER_STATIC_BASE_BYTES
                    && data_end <= stack_low
                    && stack_low < stack_high
                    && stack_high <= heap_base
                    && heap_base < PROVIDER_HEAP_LIMIT_BYTES
        )
    {
        return Err(format!(
            "provider must export an ordered low partition __data_end <= __stack_low < __stack_high <= __heap_base in [{PROVIDER_STATIC_BASE_BYTES}, {PROVIDER_HEAP_LIMIT_BYTES}), found {layout:?}"
        ));
    }
    let data_end = layout.0.expect("validated provider data end");
    for (index, (offset, end)) in data_ranges {
        if offset < PROVIDER_STATIC_BASE_BYTES || end > data_end {
            return Err(format!(
                "provider active data segment {index} occupies [{offset}, {end}), outside the provider static range [{PROVIDER_STATIC_BASE_BYTES}, {data_end})"
            ));
        }
    }

    validate_provider_export_contract(
        &types,
        &function_type_indices,
        &function_exports,
        expected_exports,
    )
}

fn collect_globals(
    section: wasmparser::GlobalSectionReader<'_>,
    globals: &mut Vec<ObservedGlobal>,
    owner: &str,
) -> Result<(), String> {
    for global in section {
        let global = global.map_err(|error| format!("failed to parse {owner} global: {error}"))?;
        let mut operators = global.init_expr.get_operators_reader();
        let value = match operators
            .read()
            .map_err(|error| format!("failed to parse {owner} global initializer: {error}"))?
        {
            Operator::I32Const { value } if value >= 0 => Some(value as u64),
            _ => None,
        };
        let end = operators
            .read()
            .map_err(|error| format!("failed to finish {owner} global initializer: {error}"))?;
        let immutable_i32_value = (!global.ty.mutable
            && global.ty.content_type == ValType::I32
            && matches!(end, Operator::End)
            && operators.eof())
        .then_some(value)
        .flatten();
        globals.push(ObservedGlobal {
            immutable_i32_value,
        });
    }
    Ok(())
}

fn exported_global_value(index: Option<u32>, globals: &[ObservedGlobal]) -> Option<u64> {
    index
        .and_then(|index| usize::try_from(index).ok())
        .and_then(|index| globals.get(index))
        .and_then(|global| global.immutable_i32_value)
}

fn active_data_range(
    kind: DataKind<'_>,
    length: usize,
    owner: &str,
    index: usize,
) -> Result<(u64, u64), String> {
    let DataKind::Active {
        memory_index,
        offset_expr,
    } = kind
    else {
        return Err(format!(
            "{owner} data segment {index} must be active so its memory partition can be verified"
        ));
    };
    if memory_index != 0 {
        return Err(format!(
            "{owner} active data segment {index} targets memory {memory_index}, expected memory 0"
        ));
    }
    let mut operators = offset_expr.get_operators_reader();
    let offset = match operators.read().map_err(|error| {
        format!("failed to parse {owner} active data segment {index} offset: {error}")
    })? {
        Operator::I32Const { value } if value >= 0 => value as u64,
        _ => {
            return Err(format!(
                "{owner} active data segment {index} must use a non-negative constant i32 offset"
            ));
        }
    };
    let end_operator = operators.read().map_err(|error| {
        format!("failed to finish {owner} active data segment {index} offset: {error}")
    })?;
    if !matches!(end_operator, Operator::End) || !operators.eof() {
        return Err(format!(
            "{owner} active data segment {index} must use exactly one constant i32 offset"
        ));
    }
    let length = u64::try_from(length)
        .map_err(|_| format!("{owner} active data segment {index} length exceeds u64"))?;
    let end = offset.checked_add(length).ok_or_else(|| {
        format!("{owner} active data segment {index} offset plus length overflows u64")
    })?;
    Ok((offset, end))
}

/// Validates the concrete function-type seam between one compiled consumer and provider.
pub(crate) fn validate_consumer_provider_signatures(
    consumer_bytes: &[u8],
    provider_bytes: &[u8],
    provider_module: &str,
) -> Result<(), String> {
    let consumer_imports = consumer_provider_import_signatures(consumer_bytes, provider_module)?;
    let provider_exports = provider_export_signatures(provider_bytes)?;
    validate_function_signature_pair(&consumer_imports, &provider_exports)
}

fn consumer_provider_import_signatures(
    bytes: &[u8],
    provider_module: &str,
) -> Result<BTreeMap<String, Vec<FuncType>>, String> {
    let mut types = Vec::<FuncType>::new();
    let mut imports = BTreeMap::<String, Vec<FuncType>>::new();

    for payload in Parser::new(0).parse_all(bytes) {
        match payload.map_err(|error| format!("failed to parse consumer Wasm: {error}"))? {
            Payload::TypeSection(section) => {
                types = section
                    .into_iter_err_on_gc_types()
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|error| format!("failed to parse consumer function types: {error}"))?;
            }
            Payload::ImportSection(section) => {
                for import in section.into_imports() {
                    let import = import
                        .map_err(|error| format!("failed to parse consumer import: {error}"))?;
                    let type_index = match import.ty {
                        TypeRef::Func(type_index) | TypeRef::FuncExact(type_index)
                            if import.module == provider_module =>
                        {
                            type_index
                        }
                        _ => continue,
                    };
                    let signature = function_type(
                        &types,
                        type_index,
                        &format!("consumer import {provider_module}.{}", import.name),
                    )?;
                    imports
                        .entry(import.name.to_owned())
                        .or_default()
                        .push(signature.clone());
                }
            }
            _ => {}
        }
    }

    if imports.is_empty() {
        return Err(format!(
            "consumer does not import any functions from {provider_module}"
        ));
    }
    Ok(imports)
}

fn provider_export_signatures(bytes: &[u8]) -> Result<BTreeMap<String, Vec<FuncType>>, String> {
    let mut types = Vec::<FuncType>::new();
    let mut function_type_indices = Vec::<u32>::new();
    let mut function_exports = BTreeMap::<String, Vec<u32>>::new();

    for payload in Parser::new(0).parse_all(bytes) {
        match payload.map_err(|error| format!("failed to parse provider Wasm: {error}"))? {
            Payload::TypeSection(section) => {
                types = section
                    .into_iter_err_on_gc_types()
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|error| format!("failed to parse provider function types: {error}"))?;
            }
            Payload::ImportSection(section) => {
                for import in section.into_imports() {
                    let import = import
                        .map_err(|error| format!("failed to parse provider import: {error}"))?;
                    if let TypeRef::Func(type_index) | TypeRef::FuncExact(type_index) = import.ty {
                        function_type_indices.push(type_index);
                    }
                }
            }
            Payload::FunctionSection(section) => {
                for type_index in section {
                    function_type_indices.push(
                        type_index.map_err(|error| {
                            format!("failed to parse provider function: {error}")
                        })?,
                    );
                }
            }
            Payload::ExportSection(section) => {
                for export in section {
                    let export = export
                        .map_err(|error| format!("failed to parse provider export: {error}"))?;
                    if matches!(export.kind, ExternalKind::Func | ExternalKind::FuncExact) {
                        function_exports
                            .entry(export.name.to_owned())
                            .or_default()
                            .push(export.index);
                    }
                }
            }
            _ => {}
        }
    }

    function_exports
        .into_iter()
        .map(|(name, function_indices)| {
            let signatures = function_indices
                .into_iter()
                .map(|function_index| {
                    let type_index = usize::try_from(function_index)
                        .ok()
                        .and_then(|index| function_type_indices.get(index))
                        .copied()
                        .ok_or_else(|| {
                            format!("provider export {name} has an invalid function index")
                        })?;
                    function_type(&types, type_index, &format!("provider export {name}")).cloned()
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok((name, signatures))
        })
        .collect()
}

fn validate_function_signature_pair(
    consumer_imports: &BTreeMap<String, Vec<FuncType>>,
    provider_exports: &BTreeMap<String, Vec<FuncType>>,
) -> Result<(), String> {
    for (name, imported_signatures) in consumer_imports {
        let Some(imported) = imported_signatures.first() else {
            return Err(format!("consumer import {name} has no function signature"));
        };
        if imported_signatures
            .iter()
            .skip(1)
            .any(|signature| signature != imported)
        {
            return Err(format!(
                "consumer imports provider function {name} with conflicting Wasm signatures: {imported_signatures:?}"
            ));
        }

        let Some(exported_signatures) = provider_exports.get(name) else {
            return Err(format!("provider does not export consumer function {name}"));
        };
        let [exported] = exported_signatures.as_slice() else {
            return Err(format!(
                "provider must export consumer function {name} exactly once"
            ));
        };
        if imported != exported {
            return Err(format!(
                "Wasm function signature mismatch for {name}: consumer imports {imported}, provider exports {exported}"
            ));
        }
    }
    Ok(())
}

fn function_type<'a>(
    types: &'a [FuncType],
    type_index: u32,
    label: &str,
) -> Result<&'a FuncType, String> {
    usize::try_from(type_index)
        .ok()
        .and_then(|index| types.get(index))
        .ok_or_else(|| format!("{label} has an invalid function type index"))
}

/// Emscripten may add runtime helpers, tables, and data exports. Only the public `b2*` and
/// `boxdd*` function namespace is closed by the checked-in contract.
fn validate_provider_export_contract(
    types: &[FuncType],
    function_type_indices: &[u32],
    function_exports: &BTreeMap<String, Vec<u32>>,
    expected_exports: &BTreeSet<String>,
) -> Result<(), String> {
    let actual_exports = function_exports
        .keys()
        .filter(|name| {
            name.starts_with("b2")
                || name.starts_with("boxdd")
                || expected_exports.contains(name.as_str())
        })
        .cloned()
        .collect::<BTreeSet<_>>();
    let missing = expected_exports
        .difference(&actual_exports)
        .cloned()
        .collect::<Vec<_>>();
    let unexpected = actual_exports
        .difference(expected_exports)
        .cloned()
        .collect::<Vec<_>>();
    if !missing.is_empty() || !unexpected.is_empty() {
        return Err(format!(
            "provider export contract mismatch: missing={missing:?}, unexpected={unexpected:?}"
        ));
    }

    for name in expected_exports {
        let Some(indices) = function_exports.get(name) else {
            return Err(format!("provider export contract omitted function {name}"));
        };
        let [function_index] = indices.as_slice() else {
            return Err(format!(
                "provider export contract requires function {name} exactly once"
            ));
        };
        let type_index = usize::try_from(*function_index)
            .ok()
            .and_then(|index| function_type_indices.get(index))
            .copied()
            .ok_or_else(|| {
                format!("provider export contract function {name} has an invalid index")
            })?;
        if usize::try_from(type_index)
            .ok()
            .and_then(|index| types.get(index))
            .is_none()
        {
            return Err(format!(
                "provider export contract function {name} has an invalid type index"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PROVIDER_MODULE: &str = "box2d-sys-v2-single";

    fn append_test_section(module: &mut Vec<u8>, id: u8, payload: &[u8]) {
        module.push(id);
        push_u32_leb(module, payload.len() as u32);
        module.extend_from_slice(payload);
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
                break;
            }
        }
    }

    fn push_u64_leb(bytes: &mut Vec<u8>, mut value: u64) {
        loop {
            let mut byte = (value & 0x7f) as u8;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            bytes.push(byte);
            if value == 0 {
                break;
            }
        }
    }

    fn push_i32_leb(bytes: &mut Vec<u8>, mut value: i32) {
        loop {
            let mut byte = (value as u8) & 0x7f;
            value >>= 7;
            let sign_set = byte & 0x40 != 0;
            let finished = (value == 0 && !sign_set) || (value == -1 && sign_set);
            if !finished {
                byte |= 0x80;
            }
            bytes.push(byte);
            if finished {
                break;
            }
        }
    }

    fn push_name(bytes: &mut Vec<u8>, value: &str) {
        push_u32_leb(bytes, value.len() as u32);
        bytes.extend_from_slice(value.as_bytes());
    }

    #[derive(Clone)]
    struct ConsumerFixtureSpec {
        memory_initial: u64,
        memory_maximum: Option<u64>,
        defined_memory: bool,
        provider_modules: Vec<&'static str>,
        globals: [u64; 5],
        exports: Vec<(&'static str, u32)>,
        data: Option<(u64, usize)>,
    }

    impl ConsumerFixtureSpec {
        fn closed() -> Self {
            Self {
                memory_initial: INITIAL_MEMORY_PAGES,
                memory_maximum: Some(MAXIMUM_MEMORY_PAGES),
                defined_memory: false,
                provider_modules: vec![TEST_PROVIDER_MODULE],
                globals: [
                    CONSUMER_GLOBAL_BASE_BYTES + 32 * 1024,
                    CONSUMER_GLOBAL_BASE_BYTES + 64 * 1024,
                    CONSUMER_GLOBAL_BASE_BYTES + 128 * 1024,
                    CONSUMER_GLOBAL_BASE_BYTES + 128 * 1024,
                    INITIAL_MEMORY_BYTES,
                ],
                exports: vec![
                    ("__data_end", 0),
                    ("__stack_low", 1),
                    ("__stack_high", 2),
                    ("__heap_base", 3),
                    ("__heap_end", 4),
                ],
                data: Some((CONSUMER_GLOBAL_BASE_BYTES, 4)),
            }
        }
    }

    fn minimal_consumer_wasm(spec: &ConsumerFixtureSpec) -> Vec<u8> {
        let mut module = b"\0asm\x01\0\0\0".to_vec();
        append_test_section(&mut module, 1, &[1, 0x60, 0, 0]);

        let mut imports = Vec::new();
        push_u32_leb(&mut imports, 1 + spec.provider_modules.len() as u32);
        push_name(&mut imports, "env");
        push_name(&mut imports, "memory");
        imports.push(0x02);
        imports.push(u8::from(spec.memory_maximum.is_some()));
        push_u64_leb(&mut imports, spec.memory_initial);
        if let Some(maximum) = spec.memory_maximum {
            push_u64_leb(&mut imports, maximum);
        }
        for module_name in &spec.provider_modules {
            push_name(&mut imports, module_name);
            push_name(&mut imports, "b2Foo");
            imports.push(0x00);
            push_u32_leb(&mut imports, 0);
        }
        append_test_section(&mut module, 2, &imports);

        if spec.defined_memory {
            let mut memories = vec![1, 1];
            push_u64_leb(&mut memories, INITIAL_MEMORY_PAGES);
            push_u64_leb(&mut memories, MAXIMUM_MEMORY_PAGES);
            append_test_section(&mut module, 5, &memories);
        }

        let mut globals = Vec::new();
        push_u32_leb(&mut globals, spec.globals.len() as u32);
        for value in spec.globals {
            globals.extend_from_slice(&[0x7f, 0x00, 0x41]);
            push_i32_leb(&mut globals, i32::try_from(value).unwrap());
            globals.push(0x0b);
        }
        append_test_section(&mut module, 6, &globals);

        let mut exports = Vec::new();
        push_u32_leb(&mut exports, spec.exports.len() as u32);
        for (name, index) in &spec.exports {
            push_name(&mut exports, name);
            exports.push(0x03);
            push_u32_leb(&mut exports, *index);
        }
        append_test_section(&mut module, 7, &exports);

        if let Some((offset, length)) = spec.data {
            let mut data = vec![1, 0, 0x41];
            push_i32_leb(&mut data, i32::try_from(offset).unwrap());
            data.push(0x0b);
            push_u32_leb(&mut data, length as u32);
            data.resize(data.len() + length, 0xa5);
            append_test_section(&mut module, 11, &data);
        }
        module
    }

    fn single_function_consumer(parameter_type: u8) -> Vec<u8> {
        let mut module = b"\0asm\x01\0\0\0".to_vec();
        append_test_section(&mut module, 1, &[1, 0x60, 1, parameter_type, 1, 0x7f]);
        let mut imports = vec![1, TEST_PROVIDER_MODULE.len() as u8];
        imports.extend_from_slice(TEST_PROVIDER_MODULE.as_bytes());
        imports.extend_from_slice(&[5]);
        imports.extend_from_slice(b"b2Foo");
        imports.extend_from_slice(&[0, 0]);
        append_test_section(&mut module, 2, &imports);
        module
    }

    fn single_function_provider(parameter_type: u8) -> Vec<u8> {
        let mut module = b"\0asm\x01\0\0\0".to_vec();
        append_test_section(&mut module, 1, &[1, 0x60, 1, parameter_type, 1, 0x7f]);
        append_test_section(&mut module, 3, &[1, 0]);
        append_test_section(&mut module, 7, &[1, 5, b'b', b'2', b'F', b'o', b'o', 0, 0]);
        append_test_section(&mut module, 10, &[1, 2, 0, 0x0b]);
        module
    }

    #[test]
    fn closed_consumer_contract_is_valid() {
        ObservedConsumerContract::closed(TEST_PROVIDER_MODULE)
            .validate(TEST_PROVIDER_MODULE, "env")
            .unwrap();
    }

    #[test]
    fn raw_consumer_wasm_exercises_the_complete_parser_contract() {
        let closed = ConsumerFixtureSpec::closed();
        validate_consumer(&minimal_consumer_wasm(&closed), TEST_PROVIDER_MODULE, "env").unwrap();

        let mut mutations = Vec::new();
        let mut mutation = closed.clone();
        mutation.memory_initial -= 1;
        mutations.push(mutation);
        let mut mutation = closed.clone();
        mutation.memory_maximum = None;
        mutations.push(mutation);
        let mut mutation = closed.clone();
        mutation.memory_maximum = Some(MAXIMUM_MEMORY_PAGES - 1);
        mutations.push(mutation);
        let mut mutation = closed.clone();
        mutation.defined_memory = true;
        mutations.push(mutation);
        let mut mutation = closed.clone();
        mutation.provider_modules = vec!["box2d-sys-v1-single"];
        mutations.push(mutation);
        let mut mutation = closed.clone();
        mutation.provider_modules.push("box2d-sys-v2-double");
        mutations.push(mutation);
        for missing in 0..closed.exports.len() {
            let mut mutation = closed.clone();
            mutation.exports.remove(missing);
            mutations.push(mutation);
        }
        let mut mutation = closed.clone();
        mutation.exports.push(("__heap_end", 4));
        mutations.push(mutation);
        let mut mutation = closed.clone();
        mutation.globals[3] = CONSUMER_GLOBAL_BASE_BYTES - 1;
        mutations.push(mutation);
        let mut mutation = closed.clone();
        mutation.globals[3] = INITIAL_MEMORY_BYTES;
        mutations.push(mutation);
        let mut mutation = closed.clone();
        mutation.globals[4] = INITIAL_MEMORY_BYTES - crate::wasm_provider_memory::WASM_PAGE_BYTES;
        mutations.push(mutation);
        let mut mutation = closed.clone();
        mutation.data = Some((CONSUMER_GLOBAL_BASE_BYTES - 1, 1));
        mutations.push(mutation);
        let mut mutation = closed.clone();
        mutation.data = Some((closed.globals[0] - 1, 2));
        mutations.push(mutation);

        for mutation in mutations {
            assert!(
                validate_consumer(
                    &minimal_consumer_wasm(&mutation),
                    TEST_PROVIDER_MODULE,
                    "env"
                )
                .is_err(),
                "accepted raw consumer mutation"
            );
        }
    }

    #[test]
    fn consumer_contract_rejects_memory_and_partition_mutations() {
        let closed = ObservedConsumerContract::closed(TEST_PROVIDER_MODULE);
        let mut mutations = Vec::new();

        let mut mutation = closed.clone();
        mutation.matching_memory_imports = 0;
        mutations.push(mutation);
        let mut mutation = closed.clone();
        mutation.defined_memories = 1;
        mutations.push(mutation);
        let mut mutation = closed.clone();
        mutation
            .provider_modules
            .insert("box2d-sys-v1-single".to_owned());
        mutations.push(mutation);
        let mut mutation = closed.clone();
        mutation.heap_base = Some(CONSUMER_GLOBAL_BASE_BYTES - 1);
        mutations.push(mutation);
        let mut mutation = closed.clone();
        mutation.heap_base = Some(INITIAL_MEMORY_BYTES);
        mutations.push(mutation);
        let mut mutation = closed.clone();
        mutation.heap_end =
            Some(INITIAL_MEMORY_BYTES - crate::wasm_provider_memory::WASM_PAGE_BYTES);
        mutations.push(mutation);
        let mut mutation = closed.clone();
        mutation.heap_end_exports = 0;
        mutations.push(mutation);

        for mutation in mutations {
            assert_ne!(mutation, closed);
            assert!(
                mutation.validate(TEST_PROVIDER_MODULE, "env").is_err(),
                "accepted mutation: {mutation:?}"
            );
        }
    }

    #[test]
    fn compiled_consumer_and_provider_function_signatures_must_match() {
        let consumer = single_function_consumer(0x7f);
        let provider = single_function_provider(0x7f);
        validate_consumer_provider_signatures(&consumer, &provider, TEST_PROVIDER_MODULE).unwrap();

        let incompatible_provider = single_function_provider(0x7e);
        let error = validate_consumer_provider_signatures(
            &consumer,
            &incompatible_provider,
            TEST_PROVIDER_MODULE,
        )
        .expect_err("different numeric signatures must fail the compiled pair gate");
        assert!(error.contains("b2Foo"), "{error}");
        assert!(error.contains("signature mismatch"), "{error}");
    }

    #[test]
    fn provider_export_contract_rejects_missing_and_unexpected_public_functions() {
        let types = vec![FuncType::new([], [])];
        let function_type_indices = vec![0];
        let function_exports = BTreeMap::from([("b2Foo".to_owned(), vec![0])]);
        let expected = BTreeSet::from(["b2Foo".to_owned()]);
        validate_provider_export_contract(
            &types,
            &function_type_indices,
            &function_exports,
            &expected,
        )
        .unwrap();

        let missing = BTreeSet::from(["b2Bar".to_owned()]);
        assert!(
            validate_provider_export_contract(
                &types,
                &function_type_indices,
                &function_exports,
                &missing,
            )
            .is_err()
        );

        let mut unexpected = function_exports.clone();
        unexpected.insert("boxddUnexpected".to_owned(), vec![0]);
        assert!(
            validate_provider_export_contract(
                &types,
                &function_type_indices,
                &unexpected,
                &expected,
            )
            .is_err()
        );
    }

    #[test]
    fn passive_and_dynamic_data_segments_fail_closed() {
        let mut passive = b"\0asm\x01\0\0\0".to_vec();
        append_test_section(&mut passive, 11, &[1, 1, 1, 0]);
        let error = validate_consumer(&passive, TEST_PROVIDER_MODULE, "env")
            .expect_err("passive data must not bypass partition validation");
        assert!(error.contains("must be active"), "{error}");

        let mut dynamic = b"\0asm\x01\0\0\0".to_vec();
        append_test_section(&mut dynamic, 11, &[1, 0, 0x23, 0, 0x0b, 1, 0]);
        let error = validate_consumer(&dynamic, TEST_PROVIDER_MODULE, "env")
            .expect_err("dynamic data offsets must not bypass partition validation");
        assert!(error.contains("non-negative constant"), "{error}");
    }
}
