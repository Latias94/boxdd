use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use quote::ToTokens;
use syn::{
    Expr, Fields, FnArg, ForeignItem, GenericArgument, Item, Lit, PathArguments, ReturnType, Type,
    Visibility,
};

use crate::{
    Error, Result,
    c_api::{AbiFieldShape, AbiPrimitive, AbiTypeShape},
};

const FFI_PATH_PREFIX: &str = "boxdd_sys::ffi";

/// Exact Rust paths that are provably exported by a generated `boxdd-sys` binding file.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SysAbiIndex {
    type_paths: BTreeSet<String>,
    field_paths: BTreeSet<String>,
    function_paths: BTreeSet<String>,
    aggregate_fields: BTreeMap<String, BTreeMap<String, FieldType>>,
    aggregate_shapes: BTreeMap<String, AggregateShape>,
    aliases: BTreeMap<String, String>,
    alias_shapes: BTreeMap<String, AliasShape>,
    function_shapes: BTreeMap<String, FunctionShape>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FieldType {
    target_type: Option<String>,
    anonymous_wrapper: bool,
    abi_type: StoredType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StoredType {
    syntax: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AggregateKind {
    Struct,
    Union,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AggregateFieldShape {
    name: String,
    abi_type: StoredType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AggregateShape {
    kind: AggregateKind,
    reprs: Vec<String>,
    fields: Vec<AggregateFieldShape>,
    has_generics: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AliasShape {
    target: StoredType,
    has_generics: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FunctionShape {
    parameters: Vec<StoredType>,
    result: Option<StoredType>,
    variadic: bool,
    unsupported_parameter: bool,
    has_generics: bool,
}

/// The generated Rust representation form for one logical C type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SysAbiTypeDefinition {
    Alias { target: AbiTypeShape },
    Aggregate { shape: AbiTypeShape },
}

#[derive(Default)]
struct ShapeResolution {
    aliases: BTreeSet<String>,
    aggregates: BTreeSet<String>,
}

impl StoredType {
    fn from_syn(rust_type: &Type) -> Self {
        Self {
            syntax: rust_type.to_token_stream().to_string(),
        }
    }

    fn parse(&self, subject: &str) -> Result<Type> {
        syn::parse_str(&self.syntax).map_err(|error| {
            Error::message(format!(
                "stored generated Rust ABI type for `{subject}` is invalid: {error}"
            ))
        })
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SysAbiAccessStep {
    pub owner_type: String,
    pub field: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SysAbiAccessProjection {
    pub root_type: String,
    pub resolved_root_type: String,
    pub steps: Vec<SysAbiAccessStep>,
}

/// Parse generated bindings and index public aggregates, aliases, fields, and C functions.
pub fn index_bindings(path: &Path) -> Result<SysAbiIndex> {
    let source = fs::read_to_string(path).map_err(|source| Error::io(path, source))?;
    let syntax = syn::parse_file(&source).map_err(|error| {
        Error::message(format!(
            "{}: invalid generated Rust bindings: {error}",
            path.display()
        ))
    })?;

    Ok(index_syntax(&syntax))
}

impl SysAbiIndex {
    /// Return whether the complete qualified path names a public FFI struct or type alias.
    pub fn contains_type_path(&self, path: &str) -> bool {
        self.type_paths.contains(path)
    }

    /// Return whether the complete qualified path names a public field on a public FFI struct.
    pub fn contains_field_path(&self, path: &str) -> bool {
        self.field_paths.contains(path)
    }

    /// Return whether the complete qualified path names a public function in an unsafe C foreign block.
    pub fn contains_function_path(&self, path: &str) -> bool {
        self.function_paths.contains(path)
    }

    /// Return the exact generated FFI type path produced by a foreign function.
    ///
    /// Primitive, pointer, generic, and otherwise non-named results intentionally return `None`;
    /// callers use this only as a route-specific type hint for by-value Box2D aggregates.
    pub fn function_return_type_path(&self, function_path: &str) -> Result<Option<String>> {
        let Some(function) = self.function_shapes.get(function_path) else {
            return Ok(None);
        };
        if function.has_generics || function.unsupported_parameter {
            return Err(Error::message(format!(
                "foreign function `{function_path}` has unsupported generic or receiver parameters"
            )));
        }
        let Some(result) = &function.result else {
            return Ok(None);
        };
        let result = result.parse(&format!("{function_path} result"))?;
        let Some(path) = local_type_path(&result).filter(|path| self.type_paths.contains(path))
        else {
            return Ok(None);
        };
        Ok(matches!(
            self.type_abi_shape(&path)?,
            Some(AbiTypeShape::Aggregate { .. })
        )
        .then_some(path))
    }

    /// Return the direct generated-Rust form of one logical type.
    ///
    /// Alias targets and aggregate fields retain named references. The returned shape is useful
    /// for comparing a C declaration with the exact Rust declaration before recursive resolution.
    pub fn type_abi_definition(&self, type_path: &str) -> Result<Option<SysAbiTypeDefinition>> {
        if !self.type_paths.contains(type_path) {
            return Ok(None);
        }
        if let Some(alias) = self.alias_shapes.get(type_path) {
            if alias.has_generics {
                return Err(Error::message(format!(
                    "generated Rust type alias `{type_path}` has unsupported generic parameters"
                )));
            }
            return Ok(Some(SysAbiTypeDefinition::Alias {
                target: self.direct_type_shape(&alias.target.parse(type_path)?, type_path)?,
            }));
        }
        let aggregate = self.aggregate_shapes.get(type_path).ok_or_else(|| {
            Error::message(format!(
                "generated Rust type `{type_path}` has no indexed ABI definition"
            ))
        })?;
        if aggregate.has_generics {
            return Err(Error::message(format!(
                "generated Rust aggregate `{type_path}` has unsupported generic parameters"
            )));
        }
        self.require_repr_c(type_path, aggregate)?;
        let fields = aggregate
            .fields
            .iter()
            .map(|field| {
                Ok(AbiFieldShape {
                    name: field.name.clone(),
                    shape: self.direct_type_shape(
                        &field
                            .abi_type
                            .parse(&format!("{type_path}::{}", field.name))?,
                        &format!("{type_path}::{}", field.name),
                    )?,
                    overlays: Vec::new(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Some(SysAbiTypeDefinition::Aggregate {
            shape: AbiTypeShape::Aggregate { fields },
        }))
    }

    /// Return the recursively resolved ABI shape of one generated public type.
    pub fn type_abi_shape(&self, type_path: &str) -> Result<Option<AbiTypeShape>> {
        if !self.type_paths.contains(type_path) {
            return Ok(None);
        }
        Ok(Some(self.resolve_named_type(
            type_path,
            &mut ShapeResolution::default(),
        )?))
    }

    /// Return the recursively resolved ABI shape of a direct public field.
    pub fn field_abi_shape(&self, field_path: &str) -> Result<Option<AbiTypeShape>> {
        let Some((owner_path, field_name)) = field_path.rsplit_once("::") else {
            return Ok(None);
        };
        let Some(field) = self
            .aggregate_fields
            .get(owner_path)
            .and_then(|fields| fields.get(field_name))
        else {
            return Ok(None);
        };
        Ok(Some(self.resolve_stored_type(
            &field.abi_type,
            field_path,
            &mut ShapeResolution::default(),
        )?))
    }

    /// Return the ABI shape of a projected leaf field.
    pub fn field_access_abi_shape(
        &self,
        projection: &SysAbiAccessProjection,
    ) -> Result<Option<AbiTypeShape>> {
        if !self.contains_field_access(projection)? {
            return Ok(None);
        }
        let Some(step) = projection.steps.last() else {
            return Ok(None);
        };
        self.field_abi_shape(&format!("{}::{}", step.owner_type, step.field))
    }

    /// Return a stable recursive fingerprint for a generated public type.
    pub fn type_abi_fingerprint(&self, type_path: &str) -> Result<Option<String>> {
        Ok(self
            .type_abi_shape(type_path)?
            .map(|shape| shape.fingerprint()))
    }

    /// Return a stable recursive fingerprint for a direct public field.
    pub fn field_abi_fingerprint(&self, field_path: &str) -> Result<Option<String>> {
        Ok(self
            .field_abi_shape(field_path)?
            .map(|shape| shape.fingerprint()))
    }

    /// Return a stable recursive fingerprint for a projected leaf field.
    pub fn field_access_abi_fingerprint(
        &self,
        projection: &SysAbiAccessProjection,
    ) -> Result<Option<String>> {
        Ok(self
            .field_access_abi_shape(projection)?
            .map(|shape| shape.fingerprint()))
    }

    /// Return the ABI shape/fingerprint of a generated C callback typedef.
    pub fn callback_abi_shape(&self, callback_path: &str) -> Result<Option<AbiTypeShape>> {
        let Some(alias) = self.alias_shapes.get(callback_path) else {
            return Ok(None);
        };
        if alias.has_generics {
            return Err(Error::message(format!(
                "callback `{callback_path}` has unsupported generic parameters"
            )));
        }
        let shape = self.resolve_stored_type(
            &alias.target,
            callback_path,
            &mut ShapeResolution::default(),
        )?;
        if matches!(shape, AbiTypeShape::Function { .. }) {
            Ok(Some(shape))
        } else {
            Ok(None)
        }
    }

    pub fn callback_abi_fingerprint(&self, callback_path: &str) -> Result<Option<String>> {
        Ok(self
            .callback_abi_shape(callback_path)?
            .map(|shape| shape.fingerprint()))
    }

    /// Return the ABI shape of one generated C foreign function declaration.
    pub fn function_abi_shape(&self, function_path: &str) -> Result<Option<AbiTypeShape>> {
        let Some(function) = self.function_shapes.get(function_path) else {
            return Ok(None);
        };
        if function.has_generics || function.unsupported_parameter {
            return Err(Error::message(format!(
                "foreign function `{function_path}` has unsupported generic or receiver parameters"
            )));
        }
        let parameters = function
            .parameters
            .iter()
            .enumerate()
            .map(|(index, parameter)| {
                self.resolve_stored_type(
                    parameter,
                    &format!("{function_path} parameter {index}"),
                    &mut ShapeResolution::default(),
                )
            })
            .collect::<Result<Vec<_>>>()?;
        let result = function
            .result
            .as_ref()
            .map(|result| {
                self.resolve_stored_type(
                    result,
                    &format!("{function_path} result"),
                    &mut ShapeResolution::default(),
                )
            })
            .transpose()?
            .unwrap_or(AbiTypeShape::Primitive {
                primitive: AbiPrimitive::Void,
            });
        Ok(Some(AbiTypeShape::Function {
            result: Box::new(result),
            parameters,
            variadic: function.variadic,
        }))
    }

    pub fn function_abi_fingerprint(&self, function_path: &str) -> Result<Option<String>> {
        Ok(self
            .function_abi_shape(function_path)?
            .map(|shape| shape.fingerprint()))
    }

    /// Resolve a generated public type alias chain, retaining non-aggregate terminal aliases.
    pub fn resolved_type_path(&self, type_path: &str) -> Result<Option<String>> {
        if !self.type_paths.contains(type_path) {
            return Ok(None);
        }
        let mut current = type_path.to_owned();
        let mut visited = BTreeSet::new();
        loop {
            if !visited.insert(current.clone()) {
                return Err(Error::message(format!(
                    "generated Rust type alias cycle includes `{current}`"
                )));
            }
            let Some(target) = self.aliases.get(&current) else {
                return Ok(Some(current));
            };
            if !self.type_paths.contains(target) {
                return Ok(Some(current));
            }
            current.clone_from(target);
        }
    }

    /// Resolve C member segments to one exact, structured generated Rust access chain.
    pub fn project_field_access(
        &self,
        root_type_path: &str,
        segments: &[&str],
    ) -> Result<Option<SysAbiAccessProjection>> {
        if segments.is_empty() || !self.type_paths.contains(root_type_path) {
            return Ok(None);
        }
        let Some(resolved_root_type) = self.resolve_aggregate_type(root_type_path)? else {
            return Ok(None);
        };
        let mut projections = BTreeSet::new();
        self.collect_access_projections(
            &resolved_root_type,
            segments,
            0,
            &mut BTreeSet::new(),
            &mut Vec::new(),
            &mut projections,
        )?;
        match projections.len() {
            0 => Ok(None),
            1 => Ok(projections
                .into_iter()
                .next()
                .map(|steps| SysAbiAccessProjection {
                    root_type: root_type_path.to_owned(),
                    resolved_root_type,
                    steps,
                })),
            _ => {
                let candidates = projections
                    .iter()
                    .map(|steps| {
                        steps
                            .iter()
                            .map(|step| step.field.as_str())
                            .collect::<Vec<_>>()
                            .join("::")
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                Err(Error::message(format!(
                    "generated Rust field projection from `{root_type_path}` through `{}` is ambiguous: {candidates}",
                    segments.join(".")
                )))
            }
        }
    }

    /// Prove that every owner/field edge in a structured access chain exists in the binding AST.
    pub fn contains_field_access(&self, projection: &SysAbiAccessProjection) -> Result<bool> {
        if self.resolved_type_path(&projection.root_type)?
            != Some(projection.resolved_root_type.clone())
            || projection.steps.is_empty()
        {
            return Ok(false);
        }
        let mut owner = projection.resolved_root_type.clone();
        for (index, step) in projection.steps.iter().enumerate() {
            if step.owner_type != owner {
                return Ok(false);
            }
            let Some(field) = self
                .aggregate_fields
                .get(&owner)
                .and_then(|fields| fields.get(&step.field))
            else {
                return Ok(false);
            };
            if index + 1 < projection.steps.len() {
                let Some(target) = &field.target_type else {
                    return Ok(false);
                };
                let Some(target) = self.resolve_aggregate_type(target)? else {
                    return Ok(false);
                };
                owner = target;
            }
        }
        Ok(true)
    }

    /// Resolve C member segments to one exact generated Rust field path.
    ///
    /// Bindgen's anonymous aggregate fields are traversed without consuming a segment. Missing
    /// fields return `Ok(None)`; aliases, anonymous-wrapper cycles, and multiple exact projections
    /// fail closed.
    pub fn project_field_path(
        &self,
        root_type_path: &str,
        segments: &[&str],
    ) -> Result<Option<String>> {
        if segments.is_empty() || !self.type_paths.contains(root_type_path) {
            return Ok(None);
        }

        Ok(self
            .project_field_access(root_type_path, segments)?
            .map(|projection| {
                let suffix = projection
                    .steps
                    .iter()
                    .map(|step| step.field.as_str())
                    .collect::<Vec<_>>()
                    .join("::");
                format!("{root_type_path}::{suffix}")
            }))
    }

    fn collect_access_projections(
        &self,
        owner_type: &str,
        segments: &[&str],
        segment_index: usize,
        active_anonymous_edges: &mut BTreeSet<(String, usize)>,
        steps: &mut Vec<SysAbiAccessStep>,
        projections: &mut BTreeSet<Vec<SysAbiAccessStep>>,
    ) -> Result<()> {
        let Some(owner_type) = self.resolve_aggregate_type(owner_type)? else {
            return Ok(());
        };
        let Some(fields) = self.aggregate_fields.get(&owner_type) else {
            return Ok(());
        };

        if let Some(field) = fields.get(segments[segment_index]) {
            steps.push(SysAbiAccessStep {
                owner_type: owner_type.clone(),
                field: segments[segment_index].to_owned(),
            });
            if segment_index + 1 == segments.len() {
                projections.insert(steps.clone());
            } else if let Some(target_type) = &field.target_type {
                self.collect_access_projections(
                    target_type,
                    segments,
                    segment_index + 1,
                    &mut BTreeSet::new(),
                    steps,
                    projections,
                )?;
            }
            steps.pop();
        }

        let edge = (owner_type.clone(), segment_index);
        if !active_anonymous_edges.insert(edge.clone()) {
            return Err(Error::message(format!(
                "anonymous bindgen wrapper cycle while projecting `{}` from `{owner_type}`",
                segments.join(".")
            )));
        }
        for (field_name, field) in fields {
            if !field.anonymous_wrapper {
                continue;
            }
            let Some(target_type) = &field.target_type else {
                continue;
            };
            steps.push(SysAbiAccessStep {
                owner_type: owner_type.clone(),
                field: field_name.clone(),
            });
            self.collect_access_projections(
                target_type,
                segments,
                segment_index,
                active_anonymous_edges,
                steps,
                projections,
            )?;
            steps.pop();
        }
        active_anonymous_edges.remove(&edge);
        Ok(())
    }

    fn resolve_aggregate_type(&self, type_path: &str) -> Result<Option<String>> {
        let mut current = type_path.to_owned();
        let mut visited = BTreeSet::new();
        loop {
            if self.aggregate_fields.contains_key(&current) {
                return Ok(Some(current));
            }
            if !visited.insert(current.clone()) {
                return Err(Error::message(format!(
                    "generated Rust type alias cycle includes `{current}`"
                )));
            }
            let Some(target) = self.aliases.get(&current) else {
                return Ok(None);
            };
            current.clone_from(target);
        }
    }

    fn resolve_stored_type(
        &self,
        stored: &StoredType,
        subject: &str,
        resolution: &mut ShapeResolution,
    ) -> Result<AbiTypeShape> {
        self.type_shape_from_syn(&stored.parse(subject)?, subject, resolution, true)
    }

    fn direct_type_shape(&self, rust_type: &Type, subject: &str) -> Result<AbiTypeShape> {
        self.type_shape_from_syn(rust_type, subject, &mut ShapeResolution::default(), false)
    }

    fn type_shape_from_syn(
        &self,
        rust_type: &Type,
        subject: &str,
        resolution: &mut ShapeResolution,
        resolve_named: bool,
    ) -> Result<AbiTypeShape> {
        if let Some(primitive) = primitive_type(rust_type) {
            return Ok(AbiTypeShape::Primitive { primitive });
        }
        match rust_type {
            Type::Array(array) => Ok(AbiTypeShape::Array {
                element: Box::new(self.type_shape_from_syn(
                    &array.elem,
                    subject,
                    resolution,
                    resolve_named,
                )?),
                length: array_length(&array.len, subject)?,
            }),
            Type::BareFn(function) => {
                let abi_is_c = function.abi.as_ref().is_some_and(|abi| {
                    abi.name
                        .as_ref()
                        .is_none_or(|name| name.value().as_str() == "C")
                });
                if !abi_is_c {
                    return Err(Error::message(format!(
                        "generated Rust ABI type `{subject}` uses a non-C function pointer"
                    )));
                }
                let parameters = function
                    .inputs
                    .iter()
                    .map(|argument| {
                        self.type_shape_from_syn(&argument.ty, subject, resolution, resolve_named)
                    })
                    .collect::<Result<Vec<_>>>()?;
                let result = match &function.output {
                    ReturnType::Default => AbiTypeShape::Primitive {
                        primitive: AbiPrimitive::Void,
                    },
                    ReturnType::Type(_, result) => {
                        self.type_shape_from_syn(result, subject, resolution, resolve_named)?
                    }
                };
                Ok(AbiTypeShape::Function {
                    result: Box::new(result),
                    parameters,
                    variadic: function.variadic.is_some(),
                })
            }
            Type::Group(group) => {
                self.type_shape_from_syn(&group.elem, subject, resolution, resolve_named)
            }
            Type::Paren(paren) => {
                self.type_shape_from_syn(&paren.elem, subject, resolution, resolve_named)
            }
            Type::Path(path) => {
                if let Some(inner) = option_inner_type(path)? {
                    let shape =
                        self.type_shape_from_syn(inner, subject, resolution, resolve_named)?;
                    if matches!(shape, AbiTypeShape::Function { .. }) {
                        return Ok(shape);
                    }
                    return Err(Error::message(format!(
                        "generated Rust ABI type `{subject}` uses Option around a non-function type"
                    )));
                }
                let local_path = local_type_path(rust_type).ok_or_else(|| {
                    Error::message(format!(
                        "generated Rust ABI type `{subject}` has unsupported path `{}`",
                        rust_type.to_token_stream()
                    ))
                })?;
                if !self.type_paths.contains(&local_path) {
                    return Err(Error::message(format!(
                        "generated Rust ABI type `{subject}` references unknown local type `{local_path}`"
                    )));
                }
                if resolve_named {
                    self.resolve_named_type(&local_path, resolution)
                } else {
                    Ok(AbiTypeShape::Named {
                        name: logical_type_name(&local_path)?.to_owned(),
                    })
                }
            }
            Type::Ptr(pointer) => Ok(AbiTypeShape::Pointer {
                mutable: pointer.mutability.is_some(),
                // A pointee definition does not affect the pointer calling ABI. Preserve named
                // references here and let validators prove the referenced type independently.
                pointee: Box::new(self.type_shape_from_syn(
                    &pointer.elem,
                    subject,
                    resolution,
                    false,
                )?),
            }),
            Type::Tuple(tuple) if tuple.elems.is_empty() => Ok(AbiTypeShape::Primitive {
                primitive: AbiPrimitive::Void,
            }),
            _ => Err(Error::message(format!(
                "generated Rust ABI type `{subject}` has unsupported shape `{}`",
                rust_type.to_token_stream()
            ))),
        }
    }

    fn resolve_named_type(
        &self,
        type_path: &str,
        resolution: &mut ShapeResolution,
    ) -> Result<AbiTypeShape> {
        if let Some(alias) = self.alias_shapes.get(type_path) {
            if alias.has_generics {
                return Err(Error::message(format!(
                    "generated Rust type alias `{type_path}` has unsupported generic parameters"
                )));
            }
            if !resolution.aliases.insert(type_path.to_owned()) {
                return Err(Error::message(format!(
                    "generated Rust type alias cycle includes `{type_path}`"
                )));
            }
            let result = self.resolve_stored_type(&alias.target, type_path, resolution);
            resolution.aliases.remove(type_path);
            return result;
        }

        let aggregate = self.aggregate_shapes.get(type_path).ok_or_else(|| {
            Error::message(format!(
                "generated Rust type `{type_path}` has no indexed ABI definition"
            ))
        })?;
        if aggregate.has_generics {
            return Err(Error::message(format!(
                "generated Rust aggregate `{type_path}` has unsupported generic parameters"
            )));
        }
        self.require_repr_c(type_path, aggregate)?;
        let logical_name = logical_type_name(type_path)?;
        if !resolution.aggregates.insert(type_path.to_owned()) {
            return Ok(AbiTypeShape::RecursiveRef {
                name: logical_name.to_owned(),
            });
        }
        let result = aggregate
            .fields
            .iter()
            .map(|field| {
                Ok(AbiFieldShape {
                    name: field.name.clone(),
                    shape: self.resolve_stored_type(
                        &field.abi_type,
                        &format!("{type_path}::{}", field.name),
                        resolution,
                    )?,
                    overlays: Vec::new(),
                })
            })
            .collect::<Result<Vec<_>>>()
            .map(|fields| AbiTypeShape::Aggregate { fields });
        resolution.aggregates.remove(type_path);
        result
    }

    fn require_repr_c(&self, type_path: &str, aggregate: &AggregateShape) -> Result<()> {
        let kind = match aggregate.kind {
            AggregateKind::Struct => "struct",
            AggregateKind::Union => "union",
        };
        let reprs = aggregate
            .reprs
            .iter()
            .map(|repr| {
                repr.chars()
                    .filter(|character| !character.is_whitespace())
                    .collect()
            })
            .collect::<Vec<String>>();
        if reprs.as_slice() == ["repr(C)"] {
            return Ok(());
        }
        Err(Error::message(format!(
            "generated Rust {kind} `{type_path}` must have exactly `#[repr(C)]` for ABI comparison"
        )))
    }
}

fn index_syntax(syntax: &syn::File) -> SysAbiIndex {
    let mut index = SysAbiIndex::default();

    for item in &syntax.items {
        match item {
            Item::Struct(item) if is_public(&item.vis) => {
                let type_path = qualified_path(&item.ident.to_string());
                index.type_paths.insert(type_path.clone());
                index_aggregate(
                    &mut index,
                    &type_path,
                    AggregateKind::Struct,
                    &item.attrs,
                    item.generics.params.is_empty(),
                    &item.fields,
                );
            }
            Item::Union(item) if is_public(&item.vis) => {
                let type_path = qualified_path(&item.ident.to_string());
                index.type_paths.insert(type_path.clone());
                index_aggregate(
                    &mut index,
                    &type_path,
                    AggregateKind::Union,
                    &item.attrs,
                    item.generics.params.is_empty(),
                    &Fields::Named(syn::FieldsNamed {
                        brace_token: item.fields.brace_token,
                        named: item.fields.named.clone(),
                    }),
                );
            }
            Item::Type(item) if is_public(&item.vis) => {
                let type_path = qualified_path(&item.ident.to_string());
                index.type_paths.insert(type_path.clone());
                if let Some(target) = local_type_path(&item.ty) {
                    index.aliases.insert(type_path.clone(), target);
                }
                index.alias_shapes.insert(
                    type_path,
                    AliasShape {
                        target: StoredType::from_syn(&item.ty),
                        has_generics: !item.generics.params.is_empty(),
                    },
                );
            }
            Item::ForeignMod(item) if is_unsafe_c_foreign_module(item) => {
                for foreign_item in &item.items {
                    let ForeignItem::Fn(function) = foreign_item else {
                        continue;
                    };
                    if is_public(&function.vis) {
                        let function_path = qualified_path(&function.sig.ident.to_string());
                        index.function_paths.insert(function_path.clone());
                        let mut parameters = Vec::new();
                        let mut unsupported_parameter = false;
                        for argument in &function.sig.inputs {
                            match argument {
                                FnArg::Typed(argument) => {
                                    parameters.push(StoredType::from_syn(&argument.ty));
                                }
                                FnArg::Receiver(_) => unsupported_parameter = true,
                            }
                        }
                        index.function_shapes.insert(
                            function_path,
                            FunctionShape {
                                parameters,
                                result: match &function.sig.output {
                                    ReturnType::Default => None,
                                    ReturnType::Type(_, return_type) => {
                                        Some(StoredType::from_syn(return_type))
                                    }
                                },
                                variadic: function.sig.variadic.is_some(),
                                unsupported_parameter,
                                has_generics: !function.sig.generics.params.is_empty(),
                            },
                        );
                    }
                }
            }
            _ => {}
        }
    }

    index
}

fn index_aggregate(
    index: &mut SysAbiIndex,
    type_path: &str,
    kind: AggregateKind,
    attrs: &[syn::Attribute],
    no_generics: bool,
    fields: &Fields,
) {
    let mut field_types = BTreeMap::new();
    let mut shape_fields = Vec::new();
    let fields = match fields {
        Fields::Named(fields) => fields
            .named
            .iter()
            .map(|field| (field.ident.as_ref().map(ToString::to_string), field))
            .collect::<Vec<_>>(),
        Fields::Unnamed(fields) => fields
            .unnamed
            .iter()
            .enumerate()
            .map(|(index, field)| (Some(format!("_{index}")), field))
            .collect::<Vec<_>>(),
        Fields::Unit => Vec::new(),
    };
    for (field_index, (field_name, field)) in fields.into_iter().enumerate() {
        let shape_name = field_name
            .clone()
            .unwrap_or_else(|| format!("_{field_index}"));
        shape_fields.push(AggregateFieldShape {
            name: canonical_abi_field_name(&shape_name),
            abi_type: StoredType::from_syn(&field.ty),
        });
        if !is_public(&field.vis) {
            continue;
        }
        let Some(field_name) = field_name else {
            continue;
        };
        index
            .field_paths
            .insert(format!("{type_path}::{field_name}"));
        field_types.insert(
            field_name.clone(),
            FieldType {
                target_type: local_type_path(&field.ty),
                anonymous_wrapper: is_bindgen_anonymous_field(&field_name),
                abi_type: StoredType::from_syn(&field.ty),
            },
        );
    }
    index
        .aggregate_fields
        .insert(type_path.to_owned(), field_types);
    let mut reprs = attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident("repr"))
        .map(|attribute| attribute.meta.to_token_stream().to_string())
        .collect::<Vec<_>>();
    reprs.sort();
    index.aggregate_shapes.insert(
        type_path.to_owned(),
        AggregateShape {
            kind,
            reprs,
            fields: shape_fields,
            has_generics: !no_generics,
        },
    );
}

fn canonical_abi_field_name(name: &str) -> String {
    let Some(base) = name.strip_suffix('_') else {
        return name.to_owned();
    };
    if matches!(
        base,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
    ) {
        base.to_owned()
    } else {
        name.to_owned()
    }
}

fn primitive_type(rust_type: &Type) -> Option<AbiPrimitive> {
    let Type::Path(path) = rust_type else {
        return None;
    };
    if path.qself.is_some()
        || path
            .path
            .segments
            .iter()
            .any(|segment| !matches!(segment.arguments, PathArguments::None))
    {
        return None;
    }
    let names = path
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>();
    let name = names.last()?.as_str();
    let primitive = match name {
        "bool" => AbiPrimitive::Bool,
        "i8" => AbiPrimitive::I8,
        "u8" => AbiPrimitive::U8,
        "i16" => AbiPrimitive::I16,
        "u16" => AbiPrimitive::U16,
        "i32" => AbiPrimitive::I32,
        "u32" => AbiPrimitive::U32,
        "i64" => AbiPrimitive::I64,
        "u64" => AbiPrimitive::U64,
        "isize" => AbiPrimitive::Isize,
        "usize" => AbiPrimitive::Usize,
        "f32" => AbiPrimitive::F32,
        "f64" => AbiPrimitive::F64,
        "c_void" => AbiPrimitive::Void,
        "c_schar" => AbiPrimitive::I8,
        "c_uchar" => AbiPrimitive::U8,
        "c_short" => AbiPrimitive::I16,
        "c_ushort" => AbiPrimitive::U16,
        "c_int" => AbiPrimitive::I32,
        "c_uint" => AbiPrimitive::U32,
        "c_long" => AbiPrimitive::I64,
        "c_ulong" => AbiPrimitive::U64,
        "c_longlong" => AbiPrimitive::I64,
        "c_ulonglong" => AbiPrimitive::U64,
        "c_float" => AbiPrimitive::F32,
        "c_double" => AbiPrimitive::F64,
        "c_char" => AbiPrimitive::I8,
        _ => return None,
    };
    Some(primitive)
}

fn array_length(expression: &Expr, subject: &str) -> Result<String> {
    match expression {
        Expr::Lit(literal) => match &literal.lit {
            Lit::Int(integer) => Ok(integer.base10_digits().to_owned()),
            _ => Err(Error::message(format!(
                "generated Rust ABI array `{subject}` has a non-integer length"
            ))),
        },
        Expr::Paren(parenthesized) => array_length(&parenthesized.expr, subject),
        _ => Err(Error::message(format!(
            "generated Rust ABI array `{subject}` has an unsupported length expression `{}`",
            expression.to_token_stream()
        ))),
    }
}

fn option_inner_type(path: &syn::TypePath) -> Result<Option<&Type>> {
    let segments = &path.path.segments;
    if segments.is_empty()
        || segments.iter().enumerate().any(|(index, segment)| {
            index + 1 != segments.len() && !matches!(segment.arguments, PathArguments::None)
        })
    {
        return Ok(None);
    }
    let last = segments.last().expect("checked non-empty segments");
    if last.ident != "Option"
        || !segments
            .iter()
            .take(segments.len().saturating_sub(1))
            .map(|segment| segment.ident.to_string())
            .all(|segment| matches!(segment.as_str(), "std" | "core" | "option"))
    {
        return Ok(None);
    }
    let PathArguments::AngleBracketed(arguments) = &last.arguments else {
        return Ok(None);
    };
    if arguments.args.len() != 1 {
        return Err(Error::message(
            "generated Rust Option ABI type must have exactly one argument",
        ));
    }
    let Some(GenericArgument::Type(inner)) = arguments.args.first() else {
        return Err(Error::message(
            "generated Rust Option ABI type argument is not a type",
        ));
    };
    Ok(Some(inner))
}

fn logical_type_name(type_path: &str) -> Result<&str> {
    type_path
        .strip_prefix(FFI_PATH_PREFIX)
        .and_then(|suffix| suffix.strip_prefix("::").filter(|name| !name.is_empty()))
        .ok_or_else(|| {
            Error::message(format!(
                "generated Rust ABI path `{type_path}` is not under `{FFI_PATH_PREFIX}`"
            ))
        })
}

fn local_type_path(rust_type: &Type) -> Option<String> {
    let Type::Path(rust_type) = rust_type else {
        return None;
    };
    if rust_type.qself.is_some() {
        return None;
    }
    if rust_type
        .path
        .segments
        .iter()
        .any(|segment| !matches!(segment.arguments, PathArguments::None))
    {
        return None;
    }
    let segments = rust_type
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>();
    match segments.as_slice() {
        [ident] => Some(qualified_path(ident)),
        [prefix, ident] if prefix == "self" => Some(qualified_path(ident)),
        [crate_name, module, ident] if crate_name == "boxdd_sys" && module == "ffi" => {
            Some(qualified_path(ident))
        }
        [crate_name, module, ident] if crate_name == "crate" && module == "ffi" => {
            Some(qualified_path(ident))
        }
        _ => None,
    }
}

fn is_bindgen_anonymous_field(name: &str) -> bool {
    name.strip_prefix("__bindgen_anon_").is_some_and(|suffix| {
        !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
    })
}

fn is_unsafe_c_foreign_module(module: &syn::ItemForeignMod) -> bool {
    module.unsafety.is_some()
        && module
            .abi
            .name
            .as_ref()
            .is_some_and(|name| name.value() == "C")
}

fn qualified_path(ident: &str) -> String {
    format!("{FFI_PATH_PREFIX}::{ident}")
}

fn is_public(visibility: &Visibility) -> bool {
    matches!(visibility, Visibility::Public(_))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PREGENERATED_BINDINGS: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../boxdd-sys/src/bindings_pregenerated.rs"
    );

    fn index(source: &str) -> SysAbiIndex {
        let syntax = syn::parse_file(source).expect("test bindings must parse");
        index_syntax(&syntax)
    }

    #[test]
    fn indexes_the_checked_in_pregenerated_bindings() {
        let bindings = Path::new(PREGENERATED_BINDINGS);
        let index = index_bindings(bindings).expect("pregenerated bindings must be indexable");

        assert!(index.contains_type_path("boxdd_sys::ffi::b2Vec2"));
        assert!(index.contains_field_path("boxdd_sys::ffi::b2Vec2::x"));
        assert!(index.contains_type_path("boxdd_sys::ffi::b2TaskCallback"));
        assert!(index.contains_function_path("boxdd_sys::ffi::b2SetAllocator"));
        assert!(index.contains_function_path("boxdd_sys::ffi::b2World_Step"));
        assert!(!index.contains_field_path("boxdd_sys::ffi::b2Vec2::missing"));
        assert!(!index.contains_type_path("boxdd_sys::ffi::MissingCallback"));
        assert!(!index.contains_function_path("boxdd_sys::ffi::b2MissingFunction"));
        assert!(
            index
                .type_abi_fingerprint("boxdd_sys::ffi::b2Vec2")
                .expect("pregenerated aggregate shape should resolve")
                .is_some()
        );
        assert!(
            index
                .callback_abi_fingerprint("boxdd_sys::ffi::b2TaskCallback")
                .expect("pregenerated callback shape should resolve")
                .is_some()
        );
        assert!(
            index
                .function_abi_fingerprint("boxdd_sys::ffi::b2World_Step")
                .expect("pregenerated function shape should resolve")
                .is_some()
        );
    }

    #[test]
    fn function_return_type_path_only_reports_indexed_by_value_ffi_types() {
        let index = index(
            r#"
                #[repr(C)]
                pub struct b2Result { pub value: i32 }
                pub type b2ResultAlias = b2Result;
                pub type b2ScalarAlias = u32;
                pub type b2CallbackAlias = Option<unsafe extern "C" fn(value: i32)>;
                unsafe extern "C" {
                    pub fn b2MakeResult() -> b2Result;
                    pub fn b2MakeResultAlias() -> b2ResultAlias;
                    pub fn b2Primitive() -> i32;
                    pub fn b2ScalarAliasResult() -> b2ScalarAlias;
                    pub fn b2CallbackAliasResult() -> b2CallbackAlias;
                    pub fn b2Pointer() -> *const b2Result;
                }
            "#,
        );

        assert_eq!(
            index
                .function_return_type_path("boxdd_sys::ffi::b2MakeResult")
                .expect("by-value result query"),
            Some("boxdd_sys::ffi::b2Result".to_owned())
        );
        assert_eq!(
            index
                .function_return_type_path("boxdd_sys::ffi::b2MakeResultAlias")
                .expect("aggregate alias result query"),
            Some("boxdd_sys::ffi::b2ResultAlias".to_owned())
        );
        assert_eq!(
            index
                .function_return_type_path("boxdd_sys::ffi::b2Primitive")
                .expect("primitive result query"),
            None
        );
        assert_eq!(
            index
                .function_return_type_path("boxdd_sys::ffi::b2ScalarAliasResult")
                .expect("primitive alias result query"),
            None
        );
        assert_eq!(
            index
                .function_return_type_path("boxdd_sys::ffi::b2CallbackAliasResult")
                .expect("callback alias result query"),
            None
        );
        assert_eq!(
            index
                .function_return_type_path("boxdd_sys::ffi::b2Pointer")
                .expect("pointer result query"),
            None
        );
        assert_eq!(
            index
                .function_return_type_path("boxdd_sys::ffi::b2Missing")
                .expect("missing result query"),
            None
        );
    }

    #[test]
    fn indexes_public_struct_fields_and_callback_aliases() {
        let index = index(
            r#"
                pub struct b2Vec2 {
                    pub x: f32,
                    pub y: f32,
                }

                pub type b2TaskCallback = Option<unsafe extern "C" fn(context: *mut ())>;
            "#,
        );

        assert!(index.contains_type_path("boxdd_sys::ffi::b2Vec2"));
        assert!(index.contains_field_path("boxdd_sys::ffi::b2Vec2::x"));
        assert!(index.contains_field_path("boxdd_sys::ffi::b2Vec2::y"));
        assert!(index.contains_type_path("boxdd_sys::ffi::b2TaskCallback"));
    }

    #[test]
    fn rejects_forged_aliases_fields_and_partial_paths() {
        let index = index(
            r#"
                pub struct b2Vec2 {
                    pub x: f32,
                }

                pub type b2TaskCallback = Option<unsafe extern "C" fn()>;
            "#,
        );

        assert!(!index.contains_type_path("boxdd_sys::ffi::b2MissingCallback"));
        assert!(!index.contains_type_path("b2TaskCallback"));
        assert!(!index.contains_field_path("boxdd_sys::ffi::b2Vec2::z"));
        assert!(!index.contains_field_path("b2Vec2::x"));
        assert!(!index.contains_field_path("prefix::boxdd_sys::ffi::b2Vec2::x"));
    }

    #[test]
    fn keeps_same_named_fields_scoped_to_their_declaring_struct() {
        let index = index(
            r#"
                pub struct First {
                    pub shared: u32,
                    pub first_only: u32,
                }

                pub struct Second {
                    pub shared: u32,
                    pub second_only: u32,
                }
            "#,
        );

        assert!(index.contains_field_path("boxdd_sys::ffi::First::shared"));
        assert!(index.contains_field_path("boxdd_sys::ffi::Second::shared"));
        assert!(!index.contains_field_path("boxdd_sys::ffi::First::second_only"));
        assert!(!index.contains_field_path("boxdd_sys::ffi::Second::first_only"));
    }

    #[test]
    fn preserves_bindgen_raw_identifiers() {
        let index = index(
            r#"
                pub struct KeywordFields {
                    pub r#type: u32,
                }

                pub type r#match = u32;
            "#,
        );

        assert!(index.contains_type_path("boxdd_sys::ffi::r#match"));
        assert!(index.contains_field_path("boxdd_sys::ffi::KeywordFields::r#type"));
        assert!(!index.contains_type_path("boxdd_sys::ffi::match"));
        assert!(!index.contains_field_path("boxdd_sys::ffi::KeywordFields::type"));
    }

    #[test]
    fn does_not_infer_nested_or_private_paths() {
        let index = index(
            r#"
                pub struct Outer {
                    pub anonymous: Outer__bindgen_ty_1,
                    private_field: u32,
                }

                pub struct Outer__bindgen_ty_1 {
                    pub nested: u32,
                }

                struct Private {
                    pub exposed_looking: u32,
                }

                pub(crate) type CrateOnly = u32;
            "#,
        );

        assert!(index.contains_field_path("boxdd_sys::ffi::Outer::anonymous"));
        assert!(index.contains_field_path("boxdd_sys::ffi::Outer__bindgen_ty_1::nested"));
        assert!(!index.contains_field_path("boxdd_sys::ffi::Outer::anonymous::nested"));
        assert!(!index.contains_field_path("boxdd_sys::ffi::Outer::private_field"));
        assert!(!index.contains_type_path("boxdd_sys::ffi::Private"));
        assert!(!index.contains_field_path("boxdd_sys::ffi::Private::exposed_looking"));
        assert!(!index.contains_type_path("boxdd_sys::ffi::CrateOnly"));
    }

    #[test]
    fn indexes_only_public_functions_from_unsafe_c_foreign_blocks() {
        let index = index(
            r#"
                unsafe extern "C" {
                    pub fn b2Present(value: u32);
                    fn b2Private();
                }

                pub unsafe extern "C" fn b2PresentOnlyAsLocal(value: u32) {
                    let _ = value;
                }

                pub fn b2ForgedLocal() {}

                unsafe extern "system" {
                    pub fn b2WrongAbi();
                }

                extern "C" {
                    pub fn b2MissingUnsafeBlock();
                }
            "#,
        );

        assert!(index.contains_function_path("boxdd_sys::ffi::b2Present"));
        assert!(!index.contains_function_path("boxdd_sys::ffi::b2Private"));
        assert!(!index.contains_function_path("boxdd_sys::ffi::b2PresentOnlyAsLocal"));
        assert!(!index.contains_function_path("boxdd_sys::ffi::b2ForgedLocal"));
        assert!(!index.contains_function_path("boxdd_sys::ffi::b2WrongAbi"));
        assert!(!index.contains_function_path("boxdd_sys::ffi::b2MissingUnsafeBlock"));
        assert!(!index.contains_function_path("b2Present"));
    }

    #[test]
    fn projects_named_field_segments_through_type_aliases() {
        let index = index(
            r#"
                pub type RootAlias = Root;
                pub type ChildAlias = self::Child;

                pub struct Root {
                    pub child: ChildAlias,
                }

                pub struct Child {
                    pub leaf: u32,
                }
            "#,
        );

        assert_eq!(
            index
                .project_field_path("boxdd_sys::ffi::RootAlias", &["child", "leaf"],)
                .expect("alias graph should resolve"),
            Some("boxdd_sys::ffi::RootAlias::child::leaf".to_owned())
        );
        assert_eq!(
            index
                .project_field_path("boxdd_sys::ffi::Root", &["missing"])
                .expect("missing field is not an index error"),
            None
        );
        let projection = index
            .project_field_access("boxdd_sys::ffi::RootAlias", &["child", "leaf"])
            .expect("structured alias graph should resolve")
            .expect("field access should exist");
        assert_eq!(projection.root_type, "boxdd_sys::ffi::RootAlias");
        assert_eq!(projection.resolved_root_type, "boxdd_sys::ffi::Root");
        assert_eq!(
            projection
                .steps
                .iter()
                .map(|step| (step.owner_type.as_str(), step.field.as_str()))
                .collect::<Vec<_>>(),
            [
                ("boxdd_sys::ffi::Root", "child"),
                ("boxdd_sys::ffi::Child", "leaf"),
            ]
        );
        assert!(
            index
                .contains_field_access(&projection)
                .expect("structured access should validate")
        );
    }

    #[test]
    fn projects_anonymous_bindgen_wrappers_without_consuming_c_segments() {
        let index = index(
            r#"
                pub struct b2TreeNode {
                    pub __bindgen_anon_1: b2TreeNode__bindgen_ty_1,
                }

                pub union b2TreeNode__bindgen_ty_1 {
                    pub children: b2TreeNode__bindgen_ty_1__bindgen_ty_1,
                    pub userData: u64,
                }

                pub struct b2TreeNode__bindgen_ty_1__bindgen_ty_1 {
                    pub child1: i32,
                    pub child2: i32,
                }
            "#,
        );

        assert_eq!(
            index
                .project_field_path("boxdd_sys::ffi::b2TreeNode", &["children", "child1"],)
                .expect("anonymous wrapper should project"),
            Some("boxdd_sys::ffi::b2TreeNode::__bindgen_anon_1::children::child1".to_owned())
        );
        let projection = index
            .project_field_access("boxdd_sys::ffi::b2TreeNode", &["children", "child1"])
            .expect("structured projection should resolve")
            .expect("structured projection should exist");
        assert_eq!(projection.root_type, "boxdd_sys::ffi::b2TreeNode");
        assert_eq!(projection.resolved_root_type, "boxdd_sys::ffi::b2TreeNode");
        assert_eq!(
            projection.steps,
            [
                SysAbiAccessStep {
                    owner_type: "boxdd_sys::ffi::b2TreeNode".to_owned(),
                    field: "__bindgen_anon_1".to_owned(),
                },
                SysAbiAccessStep {
                    owner_type: "boxdd_sys::ffi::b2TreeNode__bindgen_ty_1".to_owned(),
                    field: "children".to_owned(),
                },
                SysAbiAccessStep {
                    owner_type: "boxdd_sys::ffi::b2TreeNode__bindgen_ty_1__bindgen_ty_1".to_owned(),
                    field: "child1".to_owned(),
                },
            ]
        );
        assert!(
            index
                .contains_field_access(&projection)
                .expect("generated projection should validate")
        );
        let mut forged = projection.clone();
        forged.steps[1].owner_type = "boxdd_sys::ffi::Forged".to_owned();
        assert!(
            !index
                .contains_field_access(&forged)
                .expect("forged projection should be rejected")
        );
        assert_eq!(
            index
                .project_field_path("boxdd_sys::ffi::b2TreeNode", &["userData"])
                .expect("anonymous union leaf should project"),
            Some("boxdd_sys::ffi::b2TreeNode::__bindgen_anon_1::userData".to_owned())
        );
        assert!(index.contains_type_path("boxdd_sys::ffi::b2TreeNode__bindgen_ty_1"));
        assert!(index.contains_field_path("boxdd_sys::ffi::b2TreeNode__bindgen_ty_1::children"));
        let mut projection = index
            .project_field_access("boxdd_sys::ffi::b2TreeNode", &["children", "child1"])
            .expect("structured anonymous projection should resolve")
            .expect("anonymous projection should exist");
        assert_eq!(
            projection
                .steps
                .iter()
                .map(|step| step.field.as_str())
                .collect::<Vec<_>>(),
            ["__bindgen_anon_1", "children", "child1"]
        );
        assert!(
            index
                .contains_field_access(&projection)
                .expect("anonymous projection should validate")
        );
        projection.steps[1].owner_type = "boxdd_sys::ffi::b2TreeNode".to_owned();
        assert!(
            !index
                .contains_field_access(&projection)
                .expect("forged owner should be rejected")
        );
    }

    #[test]
    fn anonymous_projection_requires_exact_wrapper_names_and_unique_paths() {
        let not_anonymous = index(
            r#"
                pub struct Root {
                    pub prefix__bindgen_anon_1: Wrapper,
                }
                pub struct Wrapper {
                    pub value: u32,
                }
            "#,
        );
        assert_eq!(
            not_anonymous
                .project_field_path("boxdd_sys::ffi::Root", &["value"])
                .expect("lookalike field is simply not projected"),
            None
        );

        let ambiguous = index(
            r#"
                pub struct Root {
                    pub __bindgen_anon_1: First,
                    pub __bindgen_anon_2: Second,
                }
                pub struct First { pub value: u32 }
                pub struct Second { pub value: u32 }
            "#,
        );
        let error = ambiguous
            .project_field_path("boxdd_sys::ffi::Root", &["value"])
            .expect_err("multiple structural projections must fail closed");
        assert!(error.to_string().contains("is ambiguous"));

        let generic_wrapper = index(
            r#"
                pub struct Root {
                    pub __bindgen_anon_1: Wrapper<u32>,
                }
                pub struct Wrapper<T> {
                    pub value: T,
                }
            "#,
        );
        assert_eq!(
            generic_wrapper
                .project_field_path("boxdd_sys::ffi::Root", &["value"])
                .expect("generic field types must not become inferred aggregate edges"),
            None
        );
    }

    #[test]
    fn alias_cycles_fail_closed_during_projection() {
        let index = index(
            r#"
                pub type First = Second;
                pub type Second = First;
            "#,
        );
        let error = index
            .project_field_path("boxdd_sys::ffi::First", &["field"])
            .expect_err("alias cycle must not be treated as a missing field");
        assert!(error.to_string().contains("type alias cycle"));
    }

    #[test]
    fn abi_fingerprint_distinguishes_same_named_f32_and_f64_fields() {
        let single = index(
            r#"
                #[repr(C)]
                pub struct Sample { pub value: f32 }
            "#,
        );
        let double = index(
            r#"
                #[repr(C)]
                pub struct Sample { pub value: f64 }
            "#,
        );
        assert_ne!(
            single
                .field_abi_fingerprint("boxdd_sys::ffi::Sample::value")
                .expect("single field shape should resolve"),
            double
                .field_abi_fingerprint("boxdd_sys::ffi::Sample::value")
                .expect("double field shape should resolve"),
        );
    }

    #[test]
    fn abi_fingerprint_detects_callback_parameter_drift() {
        let one_argument = index(
            r#"
                pub type Callback = Option<unsafe extern "C" fn(value: f32)>;
            "#,
        );
        let two_arguments = index(
            r#"
                pub type Callback = Option<unsafe extern "C" fn(value: f32, count: u32)>;
            "#,
        );
        assert_ne!(
            one_argument
                .callback_abi_fingerprint("boxdd_sys::ffi::Callback")
                .expect("one-argument callback should resolve"),
            two_arguments
                .callback_abi_fingerprint("boxdd_sys::ffi::Callback")
                .expect("two-argument callback should resolve"),
        );
    }

    #[test]
    fn abi_fingerprint_propagates_through_recursive_aliases() {
        let aliased = index(
            r#"
                pub type Scalar = f32;
                pub type NestedScalar = Scalar;
                #[repr(C)]
                pub struct Sample { pub value: NestedScalar }
            "#,
        );
        let direct = index(
            r#"
                #[repr(C)]
                pub struct Sample { pub value: f32 }
            "#,
        );
        assert_eq!(
            aliased
                .field_abi_fingerprint("boxdd_sys::ffi::Sample::value")
                .expect("aliased field should resolve"),
            direct
                .field_abi_fingerprint("boxdd_sys::ffi::Sample::value")
                .expect("direct field should resolve"),
        );
    }

    #[test]
    fn abi_shape_rejects_alias_cycles_and_unknown_local_types() {
        let cycle = index(
            r#"
                pub type First = Second;
                pub type Second = First;
            "#,
        );
        let cycle_error = cycle
            .type_abi_fingerprint("boxdd_sys::ffi::First")
            .expect_err("alias cycle must fail closed");
        assert!(cycle_error.to_string().contains("type alias cycle"));

        let unknown = index(
            r#"
                pub type Missing = Unknown;
            "#,
        );
        let unknown_error = unknown
            .type_abi_fingerprint("boxdd_sys::ffi::Missing")
            .expect_err("unknown local type must fail closed");
        assert!(unknown_error.to_string().contains("unknown local type"));
    }

    #[test]
    fn abi_fingerprint_detects_foreign_function_signature_drift() {
        let one_argument = index(
            r#"
                unsafe extern "C" { pub fn call(value: f32); }
            "#,
        );
        let two_arguments = index(
            r#"
                unsafe extern "C" { pub fn call(value: f32, count: u32); }
            "#,
        );
        assert_ne!(
            one_argument
                .function_abi_fingerprint("boxdd_sys::ffi::call")
                .expect("one-argument function should resolve"),
            two_arguments
                .function_abi_fingerprint("boxdd_sys::ffi::call")
                .expect("two-argument function should resolve"),
        );
    }
}
