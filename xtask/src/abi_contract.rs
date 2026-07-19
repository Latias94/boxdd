use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use serde::{Deserialize, Serialize};
use syn::{
    Attribute, Expr, Fields, FnArg, ForeignItem, Item, Lit, Meta, ReturnType, Type, TypeBareFn,
    Visibility,
    parse::Parser,
    punctuated::Punctuated,
    token::Comma,
    visit::{self, Visit},
};

use crate::{
    Error, Result,
    c_api::{
        AbiFieldShape, AbiTypeShape, CApiInventory, OverlayDecl, PrecisionCApiInventory, StructDecl,
    },
    commands::api_coverage::Classification,
    commands::upstream_sync::{ArtifactProvider, ArtifactTarget, Precision, RustTarget},
    rust_index::RustIndex,
    sys_abi_index::{SysAbiAccessProjection, SysAbiAccessStep, SysAbiIndex, index_bindings},
};

const ABI_AVAILABILITY: &[&str] = &["always"];

pub const ABI_POLICY_ID: &str = "raw-ffi-abi";
pub const ABI_HEADER_EVIDENCE_ID: &str = "abi-header-parser";
pub const ABI_BINDING_EVIDENCE_ID: &str = "abi-binding-index";
pub const ABI_VALIDATOR_EVIDENCE_ID: &str = "abi-contract-validator";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AbiCapabilityPolicy {
    pub id: String,
    pub classification: Classification,
    pub rationale: String,
    pub modes: Vec<String>,
    pub providers: Vec<String>,
    pub availability: Vec<String>,
    pub evidence: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AbiTypeMapping {
    pub mode: String,
    pub provider: String,
    pub path: String,
    pub resolved_path: String,
    #[serde(default)]
    pub abi_fingerprint: String,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AbiAccessStep {
    pub owner_type: String,
    pub field: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AbiSafeWitnessKind {
    PublicType,
    StructAdapter,
    PublicField,
    Accessor,
    Builder,
    CallbackAdapter,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AbiSafeWitness {
    pub path: String,
    pub kind: AbiSafeWitnessKind,
    pub raw_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_field: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub native_symbols: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AbiFieldMapping {
    pub mode: String,
    pub provider: String,
    pub root_path: String,
    pub resolved_root_path: String,
    pub steps: Vec<AbiAccessStep>,
    #[serde(default)]
    pub abi_fingerprint: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AbiBindingIndex {
    pub artifact: String,
    pub precision: Precision,
    pub target: ArtifactTarget,
    pub provider: ArtifactProvider,
    pub index: SysAbiIndex,
    surface: AbiBindingSurface,
}

impl AbiBindingIndex {
    pub fn from_path(
        artifact: impl Into<String>,
        precision: Precision,
        target: ArtifactTarget,
        provider: ArtifactProvider,
        path: &Path,
    ) -> Result<Self> {
        let surface = AbiBindingSurface::from_path(path)?;
        surface.require_wasm_import_modules(precision, false)?;
        Ok(Self {
            artifact: artifact.into(),
            precision,
            target,
            provider,
            index: index_bindings(path)?,
            surface,
        })
    }

    pub fn refresh_from_path(&mut self, path: &Path) -> Result<()> {
        let index = index_bindings(path)?;
        let surface = AbiBindingSurface::from_path(path)?;
        surface.require_wasm_import_modules(self.precision, false)?;
        self.index = index;
        self.surface = surface;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AbiAggregateKind {
    Struct,
    Union,
}

impl AbiAggregateKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Struct => "struct",
            Self::Union => "union",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AbiAggregateSurface {
    kind: AbiAggregateKind,
    fields: Vec<String>,
    bindgen_opaque: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct AbiExternBlockSurface {
    functions: BTreeSet<String>,
    wasm_import_module: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct AbiBindingSurface {
    functions: BTreeSet<String>,
    aggregates: BTreeMap<String, AbiAggregateSurface>,
    type_aliases: BTreeSet<String>,
    extern_blocks: Vec<AbiExternBlockSurface>,
}

impl AbiBindingSurface {
    fn from_path(path: &Path) -> Result<Self> {
        let source = fs::read_to_string(path).map_err(|source| Error::io(path, source))?;
        let syntax = syn::parse_file(&source).map_err(|error| {
            Error::message(format!(
                "{}: invalid generated Rust bindings: {error}",
                path.display()
            ))
        })?;
        reject_abi_changing_attributes("binding file", &syntax.attrs)?;
        let mut surface = Self::default();
        for item in syntax.items {
            match item {
                Item::Struct(item) if is_public_visibility(&item.vis) => {
                    let path = type_path(&item.ident.to_string());
                    reject_abi_changing_attributes(&format!("aggregate `{path}`"), &item.attrs)?;
                    for field in item.fields.iter() {
                        let field_name = field
                            .ident
                            .as_ref()
                            .map(ToString::to_string)
                            .unwrap_or_else(|| "<unnamed>".to_owned());
                        validate_abi_type(&format!("field `{path}::{field_name}`"), &field.ty)?;
                    }
                    let aggregate = AbiAggregateSurface {
                        kind: AbiAggregateKind::Struct,
                        fields: binding_field_names(&path, &item.fields)?,
                        bindgen_opaque: is_bindgen_opaque_struct(&item),
                    };
                    if surface.aggregates.insert(path.clone(), aggregate).is_some() {
                        return Err(Error::message(format!(
                            "generated Rust ABI repeats public aggregate `{path}`"
                        )));
                    }
                }
                Item::Union(item) if is_public_visibility(&item.vis) => {
                    let path = type_path(&item.ident.to_string());
                    reject_abi_changing_attributes(&format!("aggregate `{path}`"), &item.attrs)?;
                    let mut fields = Vec::new();
                    for field in &item.fields.named {
                        let field_name = field
                            .ident
                            .as_ref()
                            .map(ToString::to_string)
                            .unwrap_or_else(|| "<unnamed>".to_owned());
                        reject_abi_changing_attributes(
                            &format!("field `{path}::{field_name}`"),
                            &field.attrs,
                        )?;
                        validate_abi_type(&format!("field `{path}::{field_name}`"), &field.ty)?;
                        if field_name != "<unnamed>" {
                            fields.push(field_name);
                        }
                    }
                    let aggregate = AbiAggregateSurface {
                        kind: AbiAggregateKind::Union,
                        fields,
                        bindgen_opaque: false,
                    };
                    if surface.aggregates.insert(path.clone(), aggregate).is_some() {
                        return Err(Error::message(format!(
                            "generated Rust ABI repeats public aggregate `{path}`"
                        )));
                    }
                }
                Item::Type(item) if is_public_visibility(&item.vis) => {
                    let path = type_path(&item.ident.to_string());
                    reject_abi_changing_attributes(&format!("type alias `{path}`"), &item.attrs)?;
                    validate_abi_type(&format!("type alias `{path}`"), &item.ty)?;
                    if !surface.type_aliases.insert(path.clone()) {
                        return Err(Error::message(format!(
                            "generated Rust ABI repeats public type alias `{path}`"
                        )));
                    }
                }
                Item::ForeignMod(item) => surface.index_foreign_block(item)?,
                Item::Macro(item) => {
                    let name = item
                        .mac
                        .path
                        .segments
                        .last()
                        .map(|segment| segment.ident.to_string())
                        .unwrap_or_else(|| "<macro>".to_owned());
                    return Err(unindexable_public_abi_error(&format!(
                        "macro invocation or definition `{name}`"
                    )));
                }
                Item::Mod(item) => {
                    return Err(unindexable_public_abi_error(&format!(
                        "module `{}`",
                        item.ident
                    )));
                }
                Item::Use(item) if is_public_visibility(&item.vis) => {
                    return Err(unindexable_public_abi_error("public re-export"));
                }
                Item::ExternCrate(item) if is_public_visibility(&item.vis) => {
                    return Err(unindexable_public_abi_error(
                        "public extern-crate re-export",
                    ));
                }
                Item::Enum(item) if is_public_visibility(&item.vis) => {
                    return Err(unindexable_public_abi_error(&format!(
                        "public enum `{}`",
                        item.ident
                    )));
                }
                Item::Fn(item) if is_public_visibility(&item.vis) => {
                    return Err(unindexable_public_abi_error(&format!(
                        "public Rust function `{}`",
                        item.sig.ident
                    )));
                }
                Item::Static(item) if is_public_visibility(&item.vis) => {
                    return Err(unindexable_public_abi_error(&format!(
                        "public static `{}`",
                        item.ident
                    )));
                }
                Item::Trait(item) if is_public_visibility(&item.vis) => {
                    return Err(unindexable_public_abi_error(&format!(
                        "public trait `{}`",
                        item.ident
                    )));
                }
                Item::TraitAlias(item) if is_public_visibility(&item.vis) => {
                    return Err(unindexable_public_abi_error(&format!(
                        "public trait alias `{}`",
                        item.ident
                    )));
                }
                Item::Impl(_) => {
                    return Err(unindexable_public_abi_error("implementation block"));
                }
                Item::Verbatim(_) => {
                    return Err(unindexable_public_abi_error("unparsed Rust tokens"));
                }
                _ => {}
            }
        }
        Ok(surface)
    }

    fn index_foreign_block(&mut self, item: syn::ItemForeignMod) -> Result<()> {
        reject_abi_changing_attributes("extern block", &item.attrs)?;
        let abi = item.abi.name.as_ref().map(syn::LitStr::value);
        let wasm_import_module = parse_wasm_import_module(&item.attrs)?;
        let mut block_functions = BTreeSet::new();
        for item in item.items {
            match item {
                ForeignItem::Fn(function) => {
                    let name = function.sig.ident.to_string();
                    if !is_public_visibility(&function.vis) {
                        continue;
                    }
                    reject_abi_changing_attributes(
                        &format!("foreign function `{name}`"),
                        &function.attrs,
                    )?;
                    if !name.starts_with("b2") {
                        return Err(unindexable_public_abi_error(&format!(
                            "public foreign function `{name}`"
                        )));
                    }
                    if abi.as_deref() != Some("C") {
                        return Err(Error::message(format!(
                            "generated Rust ABI foreign function `{name}` must be declared in an explicit extern-C block"
                        )));
                    }
                    for (index, argument) in function.sig.inputs.iter().enumerate() {
                        match argument {
                            FnArg::Receiver(receiver) => {
                                reject_abi_changing_attributes(
                                    &format!("foreign function `{name}` argument {index}"),
                                    &receiver.attrs,
                                )?;
                                return Err(Error::message(format!(
                                    "generated Rust ABI foreign function `{name}` contains a receiver parameter"
                                )));
                            }
                            FnArg::Typed(argument) => {
                                let subject = format!("foreign function `{name}` argument {index}");
                                reject_abi_changing_attributes(&subject, &argument.attrs)?;
                                validate_abi_type(&subject, &argument.ty)?;
                            }
                        }
                    }
                    if let Some(variadic) = &function.sig.variadic {
                        reject_abi_changing_attributes(
                            &format!("foreign function `{name}` variadic parameter"),
                            &variadic.attrs,
                        )?;
                    }
                    if let ReturnType::Type(_, result) = &function.sig.output {
                        validate_abi_type(&format!("foreign function `{name}` result"), result)?;
                    }
                    if !self.functions.insert(name.clone()) {
                        return Err(Error::message(format!(
                            "generated Rust ABI repeats public foreign function `{name}`"
                        )));
                    }
                    block_functions.insert(name);
                }
                ForeignItem::Macro(_) | ForeignItem::Verbatim(_) => {
                    return Err(unindexable_public_abi_error(
                        "macro or unparsed tokens inside an extern block",
                    ));
                }
                ForeignItem::Static(item) if is_public_visibility(&item.vis) => {
                    return Err(unindexable_public_abi_error(&format!(
                        "public foreign static `{}`",
                        item.ident
                    )));
                }
                ForeignItem::Type(item) if is_public_visibility(&item.vis) => {
                    return Err(unindexable_public_abi_error(&format!(
                        "public foreign type `{}`",
                        item.ident
                    )));
                }
                _ => {}
            }
        }
        if !block_functions.is_empty() {
            self.extern_blocks.push(AbiExternBlockSurface {
                functions: block_functions,
                wasm_import_module,
            });
        }
        Ok(())
    }

    fn require_wasm_import_modules(&self, precision: Precision, required: bool) -> Result<()> {
        let expected = expected_wasm_import_module(precision);
        let require_every_block = required
            || self
                .extern_blocks
                .iter()
                .any(|block| block.wasm_import_module.is_some());
        for block in &self.extern_blocks {
            match block.wasm_import_module.as_deref() {
                Some(actual) if actual != expected => {
                    return Err(Error::message(format!(
                        "generated Rust ABI extern block for {:?} imports WASM module `{actual}`, expected `{expected}` for {} precision",
                        block.functions,
                        precision.as_str()
                    )));
                }
                None if require_every_block => {
                    return Err(Error::message(format!(
                        "generated Rust ABI extern block for {:?} is missing `#[link(wasm_import_module = \"{expected}\")]`",
                        block.functions
                    )));
                }
                Some(_) | None => {}
            }
        }
        Ok(())
    }
}

fn binding_field_names(owner: &str, fields: &Fields) -> Result<Vec<String>> {
    match fields {
        Fields::Named(fields) => fields
            .named
            .iter()
            .filter_map(|field| {
                field
                    .ident
                    .as_ref()
                    .map(ToString::to_string)
                    .map(|name| (name, &field.attrs))
            })
            .map(|(name, attrs)| {
                reject_abi_changing_attributes(&format!("field `{owner}::{name}`"), attrs)?;
                Ok(name)
            })
            .collect(),
        Fields::Unnamed(fields) => fields
            .unnamed
            .iter()
            .enumerate()
            .map(|(index, field)| {
                let name = format!("_{index}");
                reject_abi_changing_attributes(&format!("field `{owner}::{name}`"), &field.attrs)?;
                Ok(name)
            })
            .collect(),
        Fields::Unit => Ok(Vec::new()),
    }
}

fn reject_abi_changing_attributes(subject: &str, attrs: &[Attribute]) -> Result<()> {
    for attribute in attrs {
        let disallowed = ["cfg", "cfg_attr", "link_name", "link_ordinal"]
            .into_iter()
            .find(|name| attribute.path().is_ident(name));
        if let Some(attribute_name) = disallowed {
            return Err(Error::message(format!(
                "generated Rust ABI {subject} uses unsupported `#[{attribute_name}]`; indexed ABI declarations must be unconditional and preserve their physical symbol names"
            )));
        }
        if attribute.path().is_ident("derive") {
            validate_abi_derives(subject, attribute)?;
            continue;
        }
        if !["doc", "repr", "link"]
            .into_iter()
            .any(|name| attribute.path().is_ident(name))
        {
            return Err(Error::message(format!(
                "generated Rust ABI {subject} uses an unsupported attribute macro; only bindgen's built-in doc, repr, derive, and link attributes can be indexed as a closed-world public ABI surface"
            )));
        }
    }
    Ok(())
}

fn validate_abi_derives(subject: &str, attribute: &Attribute) -> Result<()> {
    let Meta::List(list) = &attribute.meta else {
        return Err(Error::message(format!(
            "generated Rust ABI {subject} has malformed derive metadata"
        )));
    };
    let derives = Punctuated::<syn::Path, Comma>::parse_terminated
        .parse2(list.tokens.clone())
        .map_err(|error| {
            Error::message(format!(
                "generated Rust ABI {subject} has malformed derive metadata: {error}"
            ))
        })?;
    if derives.iter().all(|derive| {
        ["Debug", "Copy", "Clone"]
            .into_iter()
            .any(|name| derive.is_ident(name))
    }) {
        return Ok(());
    }
    Err(Error::message(format!(
        "generated Rust ABI {subject} uses a custom derive macro that cannot be indexed as a closed-world public ABI surface"
    )))
}

struct AbiTypeValidator<'a> {
    subject: &'a str,
    error: Option<String>,
}

impl<'ast> Visit<'ast> for AbiTypeValidator<'_> {
    fn visit_attribute(&mut self, attribute: &'ast Attribute) {
        if self.error.is_none()
            && let Err(error) =
                reject_abi_changing_attributes(self.subject, std::slice::from_ref(attribute))
        {
            self.error = Some(error.to_string());
        }
        visit::visit_attribute(self, attribute);
    }

    fn visit_type_bare_fn(&mut self, bare_fn: &'ast TypeBareFn) {
        let is_c = bare_fn
            .abi
            .as_ref()
            .and_then(|abi| abi.name.as_ref())
            .is_some_and(|name| name.value() == "C");
        if self.error.is_none() && (bare_fn.unsafety.is_none() || !is_c) {
            self.error = Some(format!(
                "generated Rust ABI {} contains a bare function pointer that is not `unsafe extern \"C\" fn`",
                self.subject
            ));
        }
        visit::visit_type_bare_fn(self, bare_fn);
    }

    fn visit_macro(&mut self, _macro: &'ast syn::Macro) {
        if self.error.is_none() {
            self.error = Some(format!(
                "generated Rust ABI {} contains macro-expanded type syntax that cannot be indexed as a closed-world public ABI surface",
                self.subject
            ));
        }
    }
}

fn validate_abi_type(subject: &str, rust_type: &Type) -> Result<()> {
    let mut validator = AbiTypeValidator {
        subject,
        error: None,
    };
    validator.visit_type(rust_type);
    validator
        .error
        .map_or(Ok(()), |error| Err(Error::message(error)))
}

fn parse_wasm_import_module(attrs: &[Attribute]) -> Result<Option<String>> {
    let parser = Punctuated::<Meta, Comma>::parse_terminated;
    let mut module = None;
    for attribute in attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident("link"))
    {
        let Meta::List(list) = &attribute.meta else {
            return Err(Error::message(
                "generated Rust ABI extern block has malformed `#[link]` metadata",
            ));
        };
        let nested = parser.parse2(list.tokens.clone()).map_err(|error| {
            Error::message(format!(
                "generated Rust ABI extern block has malformed `#[link]` metadata: {error}"
            ))
        })?;
        for meta in nested {
            if !meta.path().is_ident("wasm_import_module") {
                return Err(Error::message(
                    "generated Rust ABI extern block contains unsupported `#[link]` metadata; only `wasm_import_module` is allowed",
                ));
            }
            let Meta::NameValue(name_value) = meta else {
                return Err(Error::message(
                    "generated Rust ABI `wasm_import_module` must be one string literal",
                ));
            };
            let Expr::Lit(value) = name_value.value else {
                return Err(Error::message(
                    "generated Rust ABI `wasm_import_module` must be one string literal",
                ));
            };
            let Lit::Str(value) = value.lit else {
                return Err(Error::message(
                    "generated Rust ABI `wasm_import_module` must be one string literal",
                ));
            };
            if module.replace(value.value()).is_some() {
                return Err(Error::message(
                    "generated Rust ABI extern block repeats `wasm_import_module`",
                ));
            }
        }
    }
    Ok(module)
}

fn expected_wasm_import_module(precision: Precision) -> &'static str {
    match precision {
        Precision::Single => "box2d-sys-v0-single",
        Precision::Double => "box2d-sys-v0-double",
    }
}

fn is_bindgen_opaque_struct(item: &syn::ItemStruct) -> bool {
    let Fields::Named(fields) = &item.fields else {
        return false;
    };
    let Some(field) = fields.named.first().filter(|_| fields.named.len() == 1) else {
        return false;
    };
    let is_private_unused = matches!(field.vis, Visibility::Inherited)
        && field.ident.as_ref().is_some_and(|ident| ident == "_unused");
    let Type::Array(array) = &field.ty else {
        return false;
    };
    let is_u8 = matches!(array.elem.as_ref(), Type::Path(path) if path.qself.is_none() && path.path.is_ident("u8"));
    let is_zero = matches!(&array.len, Expr::Lit(value) if matches!(&value.lit, Lit::Int(value) if value.base10_parse::<u64>().is_ok_and(|value| value == 0)));
    is_private_unused && is_u8 && is_zero && has_repr_c(&item.attrs)
}

fn has_repr_c(attrs: &[Attribute]) -> bool {
    let parser = Punctuated::<Meta, Comma>::parse_terminated;
    attrs.iter().any(|attribute| {
        let Meta::List(list) = &attribute.meta else {
            return false;
        };
        attribute.path().is_ident("repr")
            && parser
                .parse2(list.tokens.clone())
                .is_ok_and(|metas| metas.iter().any(|meta| meta.path().is_ident("C")))
    })
}

fn unindexable_public_abi_error(subject: &str) -> Error {
    Error::message(format!(
        "generated Rust ABI contains {subject}, which cannot be indexed as a closed-world public ABI surface"
    ))
}

fn is_public_visibility(visibility: &Visibility) -> bool {
    matches!(visibility, Visibility::Public(_))
}

pub type AbiBindingIndexes = BTreeMap<String, AbiBindingIndex>;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct AbiBindingRoute {
    pub mode: String,
    pub provider: String,
    pub artifact: String,
    pub rust_target: RustTarget,
    pub rust_features: Vec<String>,
}

pub type AbiBindingRoutes = BTreeMap<(String, String), AbiBindingRoute>;
pub type AbiFunctionSymbols = BTreeMap<(String, String), BTreeMap<String, String>>;
pub type AbiRustIndexes = BTreeMap<(String, String), RustIndex>;
pub type AbiPrecisionInventories = BTreeMap<String, PrecisionCApiInventory>;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AbiFieldContract {
    pub name: String,
    pub signature: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub overlays: Vec<OverlayDecl>,
    #[serde(default)]
    pub rationale: String,
    #[serde(default)]
    pub policy: String,
    #[serde(default)]
    pub safe_paths: Vec<String>,
    #[serde(default)]
    pub safe_witnesses: Vec<AbiSafeWitness>,
    #[serde(default, alias = "rust_mappings")]
    pub raw_mappings: Vec<AbiFieldMapping>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AbiStructContract {
    pub name: String,
    pub fingerprint: String,
    pub header: String,
    #[serde(default)]
    pub rationale: String,
    #[serde(default)]
    pub policy: String,
    #[serde(default)]
    pub safe_paths: Vec<String>,
    #[serde(default)]
    pub safe_witnesses: Vec<AbiSafeWitness>,
    #[serde(default, alias = "rust_mappings")]
    pub raw_mappings: Vec<AbiTypeMapping>,
    pub fields: Vec<AbiFieldContract>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AbiCallbackContract {
    pub name: String,
    pub signature: String,
    pub fingerprint: String,
    pub header: String,
    #[serde(default)]
    pub rationale: String,
    #[serde(default)]
    pub policy: String,
    #[serde(default)]
    pub safe_paths: Vec<String>,
    #[serde(default)]
    pub safe_witnesses: Vec<AbiSafeWitness>,
    #[serde(default, alias = "rust_mappings")]
    pub raw_mappings: Vec<AbiTypeMapping>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AbiContract {
    #[serde(default)]
    pub policies: Vec<AbiCapabilityPolicy>,
    pub structs: Vec<AbiStructContract>,
    pub callbacks: Vec<AbiCallbackContract>,
}

pub fn default_policy(binding_routes: &AbiBindingRoutes) -> AbiCapabilityPolicy {
    let modes = binding_routes
        .keys()
        .map(|(mode, _)| mode.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let providers = binding_routes
        .keys()
        .map(|(_, provider)| provider.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    AbiCapabilityPolicy {
        id: ABI_POLICY_ID.to_owned(),
        classification: Classification::Raw,
        rationale: "The exact native ABI remains available through boxdd_sys::ffi, where callers must uphold all pointer, lifetime, layout, and callback contracts."
            .to_owned(),
        modes,
        providers,
        availability: vec!["always".to_owned()],
        evidence: vec![
            ABI_HEADER_EVIDENCE_ID.to_owned(),
            ABI_BINDING_EVIDENCE_ID.to_owned(),
            ABI_VALIDATOR_EVIDENCE_ID.to_owned(),
        ],
    }
}

/// Build the current executable ABI mapping from exact declarations and generated bindings.
pub fn map_inventory(
    inventory: &CApiInventory,
    binding_routes: &AbiBindingRoutes,
    binding_indexes: &AbiBindingIndexes,
) -> Result<AbiContract> {
    map_inventory_impl(inventory, None, binding_routes, binding_indexes)
}

/// Build a precision-aware executable ABI mapping.
///
/// Unlike [`map_inventory`], this entry point proves every mapped C type, field, and callback
/// against the generated Rust declaration selected by the same executable precision route.
pub fn map_precision_inventory(
    inventory: &CApiInventory,
    precision_inventories: &AbiPrecisionInventories,
    binding_routes: &AbiBindingRoutes,
    binding_indexes: &AbiBindingIndexes,
) -> Result<AbiContract> {
    require_precision_inventory_modes(precision_inventories, binding_routes)?;
    map_inventory_impl(
        inventory,
        Some(precision_inventories),
        binding_routes,
        binding_indexes,
    )
}

/// Seed a schema-v4 review with newly verified precision fingerprints.
///
/// This is intentionally stricter than normal review inheritance: every pre-existing mapping
/// coordinate and path must match the regenerated proof exactly, with only the formerly absent
/// fingerprint allowed to differ. Callers additionally gate this migration on an unchanged
/// upstream revision and schema transition.
pub fn bootstrap_legacy_precision_proofs(previous: &mut AbiContract, generated: &AbiContract) {
    let generated_structs = generated
        .structs
        .iter()
        .map(|structure| (structure.name.as_str(), structure))
        .collect::<BTreeMap<_, _>>();
    for structure in &mut previous.structs {
        let Some(generated_structure) = generated_structs.get(structure.name.as_str()) else {
            continue;
        };
        seed_type_mapping_fingerprints(
            &mut structure.raw_mappings,
            &generated_structure.raw_mappings,
        );
        let generated_fields = generated_structure
            .fields
            .iter()
            .map(|field| (field.name.as_str(), field))
            .collect::<BTreeMap<_, _>>();
        for field in &mut structure.fields {
            if let Some(generated_field) = generated_fields.get(field.name.as_str()) {
                seed_field_mapping_fingerprints(
                    &mut field.raw_mappings,
                    &generated_field.raw_mappings,
                );
            }
        }
    }

    let generated_callbacks = generated
        .callbacks
        .iter()
        .map(|callback| (callback.name.as_str(), callback))
        .collect::<BTreeMap<_, _>>();
    for callback in &mut previous.callbacks {
        if let Some(generated_callback) = generated_callbacks.get(callback.name.as_str()) {
            seed_type_mapping_fingerprints(
                &mut callback.raw_mappings,
                &generated_callback.raw_mappings,
            );
        }
    }
}

fn seed_type_mapping_fingerprints(
    previous: &mut Vec<AbiTypeMapping>,
    generated: &[AbiTypeMapping],
) {
    if previous.len() != generated.len() {
        return;
    }
    let generated = generated
        .iter()
        .map(|mapping| ((mapping.mode.as_str(), mapping.provider.as_str()), mapping))
        .collect::<BTreeMap<_, _>>();
    let mut seeded = previous.clone();
    for mapping in &mut seeded {
        let Some(expected) = generated.get(&(mapping.mode.as_str(), mapping.provider.as_str()))
        else {
            return;
        };
        if !mapping.abi_fingerprint.is_empty() {
            return;
        }
        mapping
            .abi_fingerprint
            .clone_from(&expected.abi_fingerprint);
        if mapping != *expected {
            return;
        }
    }
    *previous = seeded;
}

fn seed_field_mapping_fingerprints(
    previous: &mut Vec<AbiFieldMapping>,
    generated: &[AbiFieldMapping],
) {
    if previous.len() != generated.len() {
        return;
    }
    let generated = generated
        .iter()
        .map(|mapping| ((mapping.mode.as_str(), mapping.provider.as_str()), mapping))
        .collect::<BTreeMap<_, _>>();
    let mut seeded = previous.clone();
    for mapping in &mut seeded {
        let Some(expected) = generated.get(&(mapping.mode.as_str(), mapping.provider.as_str()))
        else {
            return;
        };
        if !mapping.abi_fingerprint.is_empty() {
            return;
        }
        mapping
            .abi_fingerprint
            .clone_from(&expected.abi_fingerprint);
        if mapping != *expected {
            return;
        }
    }
    *previous = seeded;
}

fn map_inventory_impl(
    inventory: &CApiInventory,
    precision_inventories: Option<&AbiPrecisionInventories>,
    binding_routes: &AbiBindingRoutes,
    binding_indexes: &AbiBindingIndexes,
) -> Result<AbiContract> {
    if precision_inventories.is_some() {
        require_exact_routed_binding_type_surfaces(
            inventory,
            precision_inventories,
            binding_routes,
            binding_indexes,
        )?;
    }
    let policy = default_policy(binding_routes);
    let coordinates = coordinates(&policy);
    let mut structs = Vec::with_capacity(inventory.structs.len());
    for declaration in &inventory.structs {
        let path = type_path(&declaration.name);
        let raw_mappings = coordinates
            .iter()
            .map(|(mode, provider)| {
                let binding =
                    require_route_binding(mode, provider, binding_routes, binding_indexes)?;
                let resolved_path =
                    require_resolved_type(&binding.index, &path, &declaration.name)?;
                let abi_fingerprint = precision_inventories.map_or_else(
                    || {
                        require_exact_struct_projection(declaration, binding, &path)?;
                        Ok(String::new())
                    },
                    |inventories| {
                        require_struct_abi_fingerprint(
                            declaration,
                            require_precision_inventory(mode, inventories)?,
                            binding,
                            &path,
                        )
                    },
                )?;
                Ok(AbiTypeMapping {
                    mode: mode.clone(),
                    provider: provider.clone(),
                    path: path.clone(),
                    resolved_path,
                    abi_fingerprint,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let mut fields = Vec::with_capacity(declaration.fields.len());
        for field in &declaration.fields {
            let raw_mappings = coordinates
                .iter()
                .map(|(mode, provider)| {
                    let binding =
                        require_route_binding(mode, provider, binding_routes, binding_indexes)?;
                    let projection = require_field_projection(&binding.index, &path, &field.name)?;
                    let abi_fingerprint = precision_inventories.map_or_else(
                        || Ok(String::new()),
                        |inventories| {
                            require_field_abi_fingerprint(
                                &declaration.name,
                                &field.name,
                                require_precision_inventory(mode, inventories)?,
                                &binding.index,
                                &projection,
                            )
                        },
                    )?;
                    Ok(field_mapping(mode, provider, &projection, abi_fingerprint))
                })
                .collect::<Result<Vec<_>>>()?;
            fields.push(AbiFieldContract {
                name: field.name.clone(),
                signature: field.signature.clone(),
                overlays: field.overlays.clone(),
                rationale: format!(
                    "The exact native field `{}::{}` remains available through the reviewed raw ABI mapping.",
                    declaration.name, field.name
                ),
                policy: ABI_POLICY_ID.to_owned(),
                safe_paths: Vec::new(),
                safe_witnesses: Vec::new(),
                raw_mappings,
            });
        }
        structs.push(AbiStructContract {
            name: declaration.name.clone(),
            fingerprint: declaration.fingerprint.clone(),
            header: declaration.header.clone(),
            rationale: format!(
                "The exact native structure `{}` remains available through the reviewed raw ABI mapping.",
                declaration.name
            ),
            policy: ABI_POLICY_ID.to_owned(),
            safe_paths: Vec::new(),
            safe_witnesses: Vec::new(),
            raw_mappings,
            fields,
        });
    }

    let mut callbacks = Vec::with_capacity(inventory.callbacks.len());
    for declaration in &inventory.callbacks {
        let path = type_path(&declaration.name);
        callbacks.push(AbiCallbackContract {
            name: declaration.name.clone(),
            signature: declaration.signature.clone(),
            fingerprint: declaration.fingerprint.clone(),
            header: declaration.header.clone(),
            rationale: format!(
                "The exact native callback `{}` remains available through the reviewed raw ABI mapping.",
                declaration.name
            ),
            policy: ABI_POLICY_ID.to_owned(),
            safe_paths: Vec::new(),
            safe_witnesses: Vec::new(),
            raw_mappings: coordinates
                .iter()
                .map(|(mode, provider)| {
                    let binding = require_route_binding(
                        mode,
                        provider,
                        binding_routes,
                        binding_indexes,
                    )?;
                    let resolved_path =
                        require_resolved_type(&binding.index, &path, &declaration.name)?;
                    let abi_fingerprint = precision_inventories.map_or_else(
                        || Ok(String::new()),
                        |inventories| {
                            require_callback_abi_fingerprint(
                                &declaration.name,
                                require_precision_inventory(mode, inventories)?,
                                &binding.index,
                                &path,
                            )
                        },
                    )?;
                    Ok(AbiTypeMapping {
                        mode: mode.clone(),
                        provider: provider.clone(),
                        path: path.clone(),
                        resolved_path,
                        abi_fingerprint,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        });
    }

    Ok(AbiContract {
        policies: vec![policy],
        structs,
        callbacks,
    })
}

/// Merge regenerated raw ABI proof with the previously reviewed exposure decisions.
///
/// Exact declarations inherit their reviewed exposure. Added and drifted declarations retain
/// the generated conservative Raw policy, while removed declarations disappear from the active
/// inventory contract.
pub fn preserve_reviewed_exposure(previous: &AbiContract, generated: &mut AbiContract) {
    let current_route_policy = generated
        .policies
        .iter()
        .find(|policy| policy.id == ABI_POLICY_ID)
        .cloned();
    let mut policies = generated
        .policies
        .drain(..)
        .map(|policy| (policy.id.clone(), policy))
        .collect::<BTreeMap<_, _>>();
    for policy in &previous.policies {
        let policy = current_route_policy.as_ref().map_or_else(
            || policy.clone(),
            |current| inherit_policy_route_matrix(policy, current),
        );
        policies.insert(policy.id.clone(), policy);
    }
    generated.policies = policies.into_values().collect();

    let mut previous_structs = previous
        .structs
        .iter()
        .cloned()
        .map(|structure| (structure.name.clone(), structure))
        .collect::<BTreeMap<_, _>>();
    for structure in &mut generated.structs {
        let Some(previous_structure) = previous_structs.remove(&structure.name) else {
            continue;
        };
        if structure.fingerprint == previous_structure.fingerprint
            && structure.header == previous_structure.header
            && mapping_proof_can_be_inherited(
                &previous_structure.raw_mappings,
                &structure.raw_mappings,
            )
        {
            copy_struct_exposure(structure, &previous_structure);
        } else {
            structure.rationale = format!(
                "The declaration identity or precision-specific raw ABI proof for `{}` changed, so the previous Safe review was not inherited and this refreshed capability is conservatively raw.",
                structure.name
            );
        }
        let mut previous_fields = previous_structure
            .fields
            .into_iter()
            .map(|field| (field.name.clone(), field))
            .collect::<BTreeMap<_, _>>();
        for field in &mut structure.fields {
            if let Some(previous_field) = previous_fields.remove(&field.name) {
                if field.signature == previous_field.signature
                    && field.overlays == previous_field.overlays
                    && mapping_proof_can_be_inherited(
                        &previous_field.raw_mappings,
                        &field.raw_mappings,
                    )
                {
                    copy_field_exposure(field, &previous_field);
                } else {
                    field.rationale = format!(
                        "The declaration, overlay contract, or precision-specific raw ABI proof for `{}::{}` changed, so the previous Safe review was not inherited and this refreshed field is conservatively raw.",
                        structure.name, field.name
                    );
                }
            }
        }
    }

    let mut previous_callbacks = previous
        .callbacks
        .iter()
        .cloned()
        .map(|callback| (callback.name.clone(), callback))
        .collect::<BTreeMap<_, _>>();
    for callback in &mut generated.callbacks {
        if let Some(previous_callback) = previous_callbacks.remove(&callback.name) {
            if callback.signature == previous_callback.signature
                && callback.fingerprint == previous_callback.fingerprint
                && callback.header == previous_callback.header
                && mapping_proof_can_be_inherited(
                    &previous_callback.raw_mappings,
                    &callback.raw_mappings,
                )
            {
                callback.rationale = previous_callback.rationale;
                callback.policy = previous_callback.policy;
                callback.safe_paths = previous_callback.safe_paths;
                callback.safe_witnesses = previous_callback.safe_witnesses;
            } else {
                callback.rationale = format!(
                    "The declaration identity or precision-specific raw ABI proof for `{}` changed, so the previous Safe review was not inherited and this refreshed callback is conservatively raw.",
                    callback.name
                );
            }
        }
    }
    let used_policies = generated
        .structs
        .iter()
        .flat_map(|structure| {
            std::iter::once(structure.policy.as_str())
                .chain(structure.fields.iter().map(|field| field.policy.as_str()))
        })
        .chain(
            generated
                .callbacks
                .iter()
                .map(|callback| callback.policy.as_str()),
        )
        .collect::<BTreeSet<_>>();
    generated
        .policies
        .retain(|policy| used_policies.contains(policy.id.as_str()));
}

fn inherit_policy_route_matrix(
    reviewed: &AbiCapabilityPolicy,
    current: &AbiCapabilityPolicy,
) -> AbiCapabilityPolicy {
    let mut inherited = reviewed.clone();
    inherited.modes.clone_from(&current.modes);
    inherited.providers.clone_from(&current.providers);
    inherited.availability.clone_from(&current.availability);
    inherited
}

/// Drop inherited Safe exposure when its structural Rust proof no longer matches the refreshed
/// native inventory.
///
/// A callback or field declaration can remain byte-for-byte identical while the function that
/// installs it changes its argument order or ownership shape. Declaration identity is therefore
/// necessary, but not sufficient, for carrying a Safe review across an upstream refresh.
pub fn discard_unproven_reviewed_exposure(
    contract: &mut AbiContract,
    inventory: &CApiInventory,
    binding_routes: &AbiBindingRoutes,
    rust_indexes: &AbiRustIndexes,
) {
    let policies = contract
        .policies
        .iter()
        .cloned()
        .map(|policy| (policy.id.clone(), policy))
        .collect::<BTreeMap<_, _>>();
    let empty_binding_indexes = AbiBindingIndexes::new();
    let empty_function_symbols = AbiFunctionSymbols::new();
    let empty_evidence_ids = BTreeSet::new();
    let context = AbiValidationContext::new(
        inventory,
        binding_routes,
        &empty_binding_indexes,
        &empty_function_symbols,
        rust_indexes,
        &empty_evidence_ids,
    );

    for structure in &mut contract.structs {
        if exposure_proof_has_drifted(
            &format!("ABI struct `{}`", structure.name),
            &structure.safe_paths,
            &structure.safe_witnesses,
            SafeAbiCapability::Struct(&structure.name),
            policies.get(&structure.policy),
            &context,
        ) {
            downgrade_to_raw(
                &mut structure.policy,
                &mut structure.rationale,
                &mut structure.safe_paths,
                &mut structure.safe_witnesses,
            );
        }
        for field in &mut structure.fields {
            if exposure_proof_has_drifted(
                &format!("ABI field `{}::{}`", structure.name, field.name),
                &field.safe_paths,
                &field.safe_witnesses,
                SafeAbiCapability::Field {
                    struct_name: &structure.name,
                    field_name: &field.name,
                },
                policies.get(&field.policy),
                &context,
            ) {
                downgrade_to_raw(
                    &mut field.policy,
                    &mut field.rationale,
                    &mut field.safe_paths,
                    &mut field.safe_witnesses,
                );
            }
        }
    }
    for callback in &mut contract.callbacks {
        if exposure_proof_has_drifted(
            &format!("ABI callback `{}`", callback.name),
            &callback.safe_paths,
            &callback.safe_witnesses,
            SafeAbiCapability::Callback(&callback.name),
            policies.get(&callback.policy),
            &context,
        ) {
            downgrade_to_raw(
                &mut callback.policy,
                &mut callback.rationale,
                &mut callback.safe_paths,
                &mut callback.safe_witnesses,
            );
        }
    }

    if !contract
        .policies
        .iter()
        .any(|policy| policy.id == ABI_POLICY_ID)
    {
        contract.policies.push(default_policy(binding_routes));
    }
    let used_policies = contract
        .structs
        .iter()
        .flat_map(|structure| {
            std::iter::once(structure.policy.as_str())
                .chain(structure.fields.iter().map(|field| field.policy.as_str()))
        })
        .chain(
            contract
                .callbacks
                .iter()
                .map(|callback| callback.policy.as_str()),
        )
        .collect::<BTreeSet<_>>();
    contract
        .policies
        .retain(|policy| used_policies.contains(policy.id.as_str()));
}

fn exposure_proof_has_drifted(
    subject: &str,
    safe_paths: &[String],
    safe_witnesses: &[AbiSafeWitness],
    capability: SafeAbiCapability<'_>,
    policy: Option<&AbiCapabilityPolicy>,
    context: &AbiValidationContext<'_>,
) -> bool {
    if safe_paths.is_empty() && safe_witnesses.is_empty() {
        return false;
    }
    let mut errors = Vec::new();
    validate_exposure(
        subject,
        safe_paths,
        safe_witnesses,
        capability,
        policy,
        context,
        &mut errors,
    );
    !errors.is_empty()
}

fn downgrade_to_raw(
    policy: &mut String,
    rationale: &mut String,
    safe_paths: &mut Vec<String>,
    safe_witnesses: &mut Vec<AbiSafeWitness>,
) {
    *policy = ABI_POLICY_ID.to_owned();
    *rationale = "The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw."
        .to_owned();
    safe_paths.clear();
    safe_witnesses.clear();
}

fn copy_struct_exposure(target: &mut AbiStructContract, source: &AbiStructContract) {
    target.rationale.clone_from(&source.rationale);
    target.policy.clone_from(&source.policy);
    target.safe_paths.clone_from(&source.safe_paths);
    target.safe_witnesses.clone_from(&source.safe_witnesses);
}

fn copy_field_exposure(target: &mut AbiFieldContract, source: &AbiFieldContract) {
    target.rationale.clone_from(&source.rationale);
    target.policy.clone_from(&source.policy);
    target.safe_paths.clone_from(&source.safe_paths);
    target.safe_witnesses.clone_from(&source.safe_witnesses);
}

pub struct AbiValidationContext<'a> {
    inventory: &'a CApiInventory,
    precision_inventories: Option<&'a AbiPrecisionInventories>,
    expected_function_count: Option<usize>,
    binding_routes: &'a AbiBindingRoutes,
    binding_indexes: &'a AbiBindingIndexes,
    function_symbols: &'a AbiFunctionSymbols,
    rust_indexes: &'a AbiRustIndexes,
    evidence_ids: &'a BTreeSet<String>,
}

impl<'a> AbiValidationContext<'a> {
    pub fn new(
        inventory: &'a CApiInventory,
        binding_routes: &'a AbiBindingRoutes,
        binding_indexes: &'a AbiBindingIndexes,
        function_symbols: &'a AbiFunctionSymbols,
        rust_indexes: &'a AbiRustIndexes,
        evidence_ids: &'a BTreeSet<String>,
    ) -> Self {
        Self {
            inventory,
            precision_inventories: None,
            expected_function_count: None,
            binding_routes,
            binding_indexes,
            function_symbols,
            rust_indexes,
            evidence_ids,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_precision(
        inventory: &'a CApiInventory,
        precision_inventories: &'a AbiPrecisionInventories,
        binding_routes: &'a AbiBindingRoutes,
        binding_indexes: &'a AbiBindingIndexes,
        function_symbols: &'a AbiFunctionSymbols,
        rust_indexes: &'a AbiRustIndexes,
        evidence_ids: &'a BTreeSet<String>,
    ) -> Self {
        Self {
            inventory,
            precision_inventories: Some(precision_inventories),
            expected_function_count: None,
            binding_routes,
            binding_indexes,
            function_symbols,
            rust_indexes,
            evidence_ids,
        }
    }

    pub fn with_expected_function_count(mut self, expected: Option<usize>) -> Self {
        self.expected_function_count = expected;
        self
    }
}

pub fn validate(
    contract: &AbiContract,
    context: &AbiValidationContext<'_>,
    errors: &mut Vec<String>,
) {
    validate_binding_routes(context.binding_routes, context.binding_indexes, errors);
    if let Some(precision_inventories) = context.precision_inventories
        && let Err(error) =
            require_precision_inventory_modes(precision_inventories, context.binding_routes)
    {
        errors.push(error.to_string());
    }
    if context.rust_indexes.keys().collect::<BTreeSet<_>>()
        != context.binding_routes.keys().collect::<BTreeSet<_>>()
    {
        errors.push(
            "Safe Rust indexes must cover exactly the executable manifest binding routes"
                .to_owned(),
        );
    }
    let policies = validate_policies(
        &contract.policies,
        context.binding_routes,
        context.evidence_ids,
        errors,
    );
    let mut used_policies = BTreeSet::new();
    validate_referenced_binding_types(
        context.inventory,
        context.precision_inventories,
        context.binding_routes,
        context.binding_indexes,
        errors,
    );
    validate_structs(contract, context, &policies, &mut used_policies, errors);
    validate_callbacks(contract, context, &policies, &mut used_policies, errors);
    for policy in policies.keys() {
        if !used_policies.contains(*policy) {
            errors.push(format!("ABI policy `{policy}` is unused"));
        }
    }
    validate_referenced_binding_functions(
        context.inventory,
        context.expected_function_count,
        context.binding_routes,
        context.binding_indexes,
        context.function_symbols,
        errors,
    );
}

fn validate_policies<'a>(
    contract: &'a [AbiCapabilityPolicy],
    binding_routes: &AbiBindingRoutes,
    evidence_ids: &BTreeSet<String>,
    errors: &mut Vec<String>,
) -> BTreeMap<&'a str, &'a AbiCapabilityPolicy> {
    let allowed_modes = binding_routes
        .keys()
        .map(|(mode, _)| mode.as_str())
        .collect::<BTreeSet<_>>();
    let allowed_providers = binding_routes
        .keys()
        .map(|(_, provider)| provider.as_str())
        .collect::<BTreeSet<_>>();
    let expected_coordinates = binding_routes.keys().cloned().collect::<BTreeSet<_>>();
    let mut policies = BTreeMap::new();
    for policy in contract {
        if !is_policy_id(&policy.id) {
            errors.push(format!(
                "ABI policy id `{}` must be non-empty kebab-case ASCII",
                policy.id
            ));
        }
        if policies.insert(policy.id.as_str(), policy).is_some() {
            errors.push(format!("duplicate ABI policy `{}`", policy.id));
        }
        if !has_rationale(&policy.rationale) {
            errors.push(format!(
                "ABI policy `{}` needs a specific rationale",
                policy.id
            ));
        }
        validate_registry_values(
            &format!("ABI policy `{}`", policy.id),
            "mode",
            &policy.modes,
            &allowed_modes,
            errors,
        );
        validate_registry_values(
            &format!("ABI policy `{}`", policy.id),
            "provider",
            &policy.providers,
            &allowed_providers,
            errors,
        );
        validate_registry_values(
            &format!("ABI policy `{}`", policy.id),
            "availability",
            &policy.availability,
            &ABI_AVAILABILITY.iter().copied().collect(),
            errors,
        );
        if coordinates(policy).into_iter().collect::<BTreeSet<_>>() != expected_coordinates
            || value_set(&policy.availability)
                != ABI_AVAILABILITY.iter().copied().collect::<BTreeSet<_>>()
        {
            errors.push(format!(
                "ABI policy `{}` must cover exactly the current executable mode/provider/availability matrix",
                policy.id
            ));
        }
        if policy.evidence.is_empty() {
            errors.push(format!("ABI policy `{}` has no test evidence", policy.id));
        }
        let mut policy_evidence = BTreeSet::new();
        for evidence in &policy.evidence {
            if !policy_evidence.insert(evidence.as_str()) {
                errors.push(format!(
                    "ABI policy `{}` repeats evidence `{evidence}`",
                    policy.id
                ));
            }
            if !evidence_ids.contains(evidence) {
                errors.push(format!(
                    "ABI policy `{}` references unknown evidence `{evidence}`",
                    policy.id
                ));
            }
        }
    }
    policies
}

fn validate_structs(
    contract: &AbiContract,
    context: &AbiValidationContext<'_>,
    policies: &BTreeMap<&str, &AbiCapabilityPolicy>,
    used_policies: &mut BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    let expected_structs = context
        .inventory
        .structs
        .iter()
        .map(|declaration| (declaration.name.as_str(), declaration))
        .collect::<BTreeMap<_, _>>();
    let mut seen_structs = BTreeSet::new();
    for structure in &contract.structs {
        if !seen_structs.insert(structure.name.as_str()) {
            errors.push(format!("duplicate ABI struct `{}`", structure.name));
        }
        let Some(declaration) = expected_structs.get(structure.name.as_str()) else {
            errors.push(format!(
                "ABI struct `{}` is absent from active headers",
                structure.name
            ));
            continue;
        };
        if structure.fingerprint != declaration.fingerprint
            || structure.header != declaration.header
        {
            errors.push(format!(
                "ABI struct declaration drifted for `{}`",
                structure.name
            ));
        }
        validate_capability_rationale(
            &format!("ABI struct `{}`", structure.name),
            &structure.rationale,
            errors,
        );
        let policy = policy_reference(
            &format!("ABI struct `{}`", structure.name),
            &structure.policy,
            policies,
            used_policies,
            errors,
        );
        validate_exposure(
            &format!("ABI struct `{}`", structure.name),
            &structure.safe_paths,
            &structure.safe_witnesses,
            SafeAbiCapability::Struct(&structure.name),
            policy,
            context,
            errors,
        );
        validate_struct_mappings(
            &format!("ABI struct `{}`", structure.name),
            declaration,
            &structure.raw_mappings,
            policy,
            context,
            errors,
        );
        validate_fields(
            structure,
            declaration,
            context,
            policies,
            used_policies,
            errors,
        );
    }
    for declaration in &context.inventory.structs {
        if !seen_structs.contains(declaration.name.as_str()) {
            errors.push(format!(
                "active header ABI struct `{}` has no capability mapping",
                declaration.name
            ));
        }
    }
}

fn validate_fields(
    structure: &AbiStructContract,
    declaration: &crate::c_api::StructDecl,
    context: &AbiValidationContext<'_>,
    policies: &BTreeMap<&str, &AbiCapabilityPolicy>,
    used_policies: &mut BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    let expected_fields = declaration
        .fields
        .iter()
        .map(|field| (field.name.as_str(), field))
        .collect::<BTreeMap<_, _>>();
    let mut seen_fields = BTreeSet::new();
    for field in &structure.fields {
        if !seen_fields.insert(field.name.as_str()) {
            errors.push(format!(
                "duplicate ABI field `{}::{}`",
                structure.name, field.name
            ));
        }
        let Some(expected) = expected_fields.get(field.name.as_str()) else {
            errors.push(format!(
                "ABI field `{}::{}` is absent from active headers",
                structure.name, field.name
            ));
            continue;
        };
        if field.signature != expected.signature || field.overlays != expected.overlays {
            errors.push(format!(
                "ABI field declaration drifted for `{}::{}`",
                structure.name, field.name
            ));
        }
        let subject = format!("ABI field `{}::{}`", structure.name, field.name);
        validate_capability_rationale(&subject, &field.rationale, errors);
        let policy = policy_reference(&subject, &field.policy, policies, used_policies, errors);
        validate_exposure(
            &subject,
            &field.safe_paths,
            &field.safe_witnesses,
            SafeAbiCapability::Field {
                struct_name: &structure.name,
                field_name: &field.name,
            },
            policy,
            context,
            errors,
        );
        validate_field_mappings(
            &subject,
            &structure.name,
            &field.name,
            &field.raw_mappings,
            policy,
            context,
            errors,
        );
    }
    for expected in &declaration.fields {
        if !seen_fields.contains(expected.name.as_str()) {
            errors.push(format!(
                "active header ABI field `{}::{}` has no explicit capability mapping",
                structure.name, expected.name
            ));
        }
    }
}

fn validate_callbacks(
    contract: &AbiContract,
    context: &AbiValidationContext<'_>,
    policies: &BTreeMap<&str, &AbiCapabilityPolicy>,
    used_policies: &mut BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    let expected_callbacks = context
        .inventory
        .callbacks
        .iter()
        .map(|declaration| (declaration.name.as_str(), declaration))
        .collect::<BTreeMap<_, _>>();
    let mut seen_callbacks = BTreeSet::new();
    for callback in &contract.callbacks {
        if !seen_callbacks.insert(callback.name.as_str()) {
            errors.push(format!("duplicate ABI callback `{}`", callback.name));
        }
        let Some(declaration) = expected_callbacks.get(callback.name.as_str()) else {
            errors.push(format!(
                "ABI callback `{}` is absent from active headers",
                callback.name
            ));
            continue;
        };
        if callback.signature != declaration.signature
            || callback.fingerprint != declaration.fingerprint
            || callback.header != declaration.header
        {
            errors.push(format!(
                "ABI callback declaration drifted for `{}`",
                callback.name
            ));
        }
        validate_capability_rationale(
            &format!("ABI callback `{}`", callback.name),
            &callback.rationale,
            errors,
        );
        let subject = format!("ABI callback `{}`", callback.name);
        let policy = policy_reference(&subject, &callback.policy, policies, used_policies, errors);
        validate_exposure(
            &subject,
            &callback.safe_paths,
            &callback.safe_witnesses,
            SafeAbiCapability::Callback(&callback.name),
            policy,
            context,
            errors,
        );
        validate_callback_mappings(
            &subject,
            &callback.name,
            &callback.raw_mappings,
            policy,
            context,
            errors,
        );
    }
    for declaration in &context.inventory.callbacks {
        if !seen_callbacks.contains(declaration.name.as_str()) {
            errors.push(format!(
                "active header ABI callback `{}` has no capability mapping",
                declaration.name
            ));
        }
    }
}

fn validate_struct_mappings(
    subject: &str,
    declaration: &StructDecl,
    mappings: &[AbiTypeMapping],
    policy: Option<&AbiCapabilityPolicy>,
    context: &AbiValidationContext<'_>,
    errors: &mut Vec<String>,
) {
    validate_mapping_coordinates(subject, mappings, policy, errors);
    let expected_path = type_path(&declaration.name);
    for mapping in mappings {
        let Some(binding) = mapping_binding(
            subject,
            &mapping.mode,
            &mapping.provider,
            context.binding_routes,
            context.binding_indexes,
            errors,
        ) else {
            continue;
        };
        let index = &binding.index;
        let expected_resolved = match index.resolved_type_path(&expected_path) {
            Ok(Some(path)) => Some(path),
            Ok(None) => {
                errors.push(format!(
                    "{subject} expected generated type path `{expected_path}` is absent from binding artifact `{}`",
                    route_artifact(&mapping.mode, &mapping.provider, context.binding_routes)
                ));
                None
            }
            Err(error) => {
                errors.push(format!(
                    "{subject} in binding artifact `{}`: {error}",
                    route_artifact(&mapping.mode, &mapping.provider, context.binding_routes)
                ));
                None
            }
        };
        if mapping.path != expected_path {
            errors.push(format!(
                "{subject} maps to `{}`, expected canonical generated path `{expected_path}`",
                mapping.path
            ));
        }
        if expected_resolved.as_ref() != Some(&mapping.resolved_path) {
            errors.push(format!(
                "{subject} resolves `{}` to `{}`, expected `{:?}`",
                mapping.path, mapping.resolved_path, expected_resolved
            ));
        }
        if let Some(precision_inventories) = context.precision_inventories {
            validate_struct_mapping_fingerprint(
                subject,
                declaration,
                mapping,
                precision_inventories,
                binding,
                errors,
            );
        } else if let Err(error) =
            require_exact_struct_projection(declaration, binding, &expected_path)
        {
            errors.push(format!(
                "{subject} at `{}/{}`: {error}",
                mapping.mode, mapping.provider
            ));
        }
    }
}

fn validate_callback_mappings(
    subject: &str,
    callback_name: &str,
    mappings: &[AbiTypeMapping],
    policy: Option<&AbiCapabilityPolicy>,
    context: &AbiValidationContext<'_>,
    errors: &mut Vec<String>,
) {
    validate_mapping_coordinates(subject, mappings, policy, errors);
    let expected_path = type_path(callback_name);
    for mapping in mappings {
        let Some(index) = mapping_binding_index(
            subject,
            &mapping.mode,
            &mapping.provider,
            context.binding_routes,
            context.binding_indexes,
            errors,
        ) else {
            continue;
        };
        let expected_resolved = match index.resolved_type_path(&expected_path) {
            Ok(Some(path)) => Some(path),
            Ok(None) => {
                errors.push(format!(
                    "{subject} expected generated callback path `{expected_path}` is absent from binding artifact `{}`",
                    route_artifact(&mapping.mode, &mapping.provider, context.binding_routes)
                ));
                None
            }
            Err(error) => {
                errors.push(format!(
                    "{subject} in binding artifact `{}`: {error}",
                    route_artifact(&mapping.mode, &mapping.provider, context.binding_routes)
                ));
                None
            }
        };
        if mapping.path != expected_path {
            errors.push(format!(
                "{subject} maps to `{}`, expected canonical generated callback path `{expected_path}`",
                mapping.path
            ));
        }
        if expected_resolved.as_ref() != Some(&mapping.resolved_path) {
            errors.push(format!(
                "{subject} resolves `{}` to `{}`, expected `{:?}`",
                mapping.path, mapping.resolved_path, expected_resolved
            ));
        }
        if let Some(precision_inventories) = context.precision_inventories {
            validate_callback_mapping_fingerprint(
                subject,
                callback_name,
                mapping,
                precision_inventories,
                index,
                errors,
            );
        }
    }
}

fn validate_field_mappings(
    subject: &str,
    struct_name: &str,
    c_field: &str,
    mappings: &[AbiFieldMapping],
    policy: Option<&AbiCapabilityPolicy>,
    context: &AbiValidationContext<'_>,
    errors: &mut Vec<String>,
) {
    validate_mapping_coordinates(subject, mappings, policy, errors);
    let root_path = type_path(struct_name);
    for mapping in mappings {
        let Some(binding) = mapping_binding(
            subject,
            &mapping.mode,
            &mapping.provider,
            context.binding_routes,
            context.binding_indexes,
            errors,
        ) else {
            continue;
        };
        let index = &binding.index;
        let expected = match require_field_projection(index, &root_path, c_field) {
            Ok(projection) => Some(field_mapping("", "", &projection, String::new())),
            Err(error) => {
                errors.push(format!(
                    "{subject} in binding artifact `{}`: {error}",
                    binding.artifact
                ));
                None
            }
        };
        if let Some(expected) = &expected
            && (mapping.root_path != expected.root_path
                || mapping.resolved_root_path != expected.resolved_root_path
                || mapping.steps != expected.steps)
        {
            errors.push(format!(
                "{subject} has a forged or stale generated Rust access chain"
            ));
        }
        let projection = sys_projection(mapping);
        match index.contains_field_access(&projection) {
            Ok(true) => {}
            Ok(false) => errors.push(format!(
                "{subject} access chain is absent from the Rust binding AST"
            )),
            Err(error) => errors.push(format!("{subject}: {error}")),
        }
        if let Some(precision_inventories) = context.precision_inventories {
            validate_field_mapping_fingerprint(
                subject,
                struct_name,
                c_field,
                mapping,
                precision_inventories,
                index,
                &projection,
                errors,
            );
        }
    }
}

#[derive(Clone, Copy)]
enum SafeAbiCapability<'a> {
    Struct(&'a str),
    Field {
        struct_name: &'a str,
        field_name: &'a str,
    },
    Callback(&'a str),
}

fn validate_exposure(
    subject: &str,
    safe_paths: &[String],
    safe_witnesses: &[AbiSafeWitness],
    capability: SafeAbiCapability<'_>,
    policy: Option<&AbiCapabilityPolicy>,
    context: &AbiValidationContext<'_>,
    errors: &mut Vec<String>,
) {
    let mut unique_paths = BTreeSet::new();
    for path in safe_paths {
        if !unique_paths.insert(path.as_str()) {
            errors.push(format!("{subject} repeats Safe Rust path `{path}`"));
        }
    }
    let mut witness_paths = BTreeSet::new();
    for witness in safe_witnesses {
        if !witness_paths.insert(witness.path.as_str()) {
            errors.push(format!(
                "{subject} repeats Safe Rust witness path `{}`",
                witness.path
            ));
        }
    }
    if unique_paths != witness_paths {
        errors.push(format!(
            "{subject} must provide exactly one capability-specific witness for every Safe Rust path"
        ));
    }
    let Some(policy) = policy else {
        validate_exposure_for_coordinates(
            subject,
            safe_paths,
            safe_witnesses,
            capability,
            context.rust_indexes.keys().cloned(),
            context,
            errors,
        );
        return;
    };
    validate_exposure_for_coordinates(
        subject,
        safe_paths,
        safe_witnesses,
        capability,
        coordinates(policy),
        context,
        errors,
    );
    match policy.classification {
        Classification::Safe if safe_paths.is_empty() || safe_witnesses.is_empty() => {
            errors.push(format!(
                "{subject} is classified safe but has no witnessed canonical Safe Rust path"
            ));
        }
        Classification::Safe => {}
        Classification::Raw | Classification::Omitted | Classification::Deferred
            if !safe_paths.is_empty() || !safe_witnesses.is_empty() =>
        {
            errors.push(format!(
                "{subject} is classified {} and cannot claim Safe Rust paths",
                policy.classification.as_str()
            ));
        }
        Classification::Raw | Classification::Omitted | Classification::Deferred => {}
    }
}

fn validate_exposure_for_coordinates(
    subject: &str,
    safe_paths: &[String],
    safe_witnesses: &[AbiSafeWitness],
    capability: SafeAbiCapability<'_>,
    coordinates: impl IntoIterator<Item = (String, String)>,
    context: &AbiValidationContext<'_>,
    errors: &mut Vec<String>,
) {
    if safe_paths.is_empty() && safe_witnesses.is_empty() {
        return;
    }
    for coordinate in coordinates {
        let Some(index) = context.rust_indexes.get(&coordinate) else {
            errors.push(format!(
                "{subject} has no Safe Rust index for route `{}/{}`",
                coordinate.0, coordinate.1
            ));
            continue;
        };
        let coordinate_subject = format!(
            "{subject} at Safe Rust route `{}/{}`",
            coordinate.0, coordinate.1
        );
        for path in safe_paths {
            if !index.contains_public_path(path) {
                errors.push(format!(
                    "{coordinate_subject} references nonexistent canonical Safe Rust path `{path}`"
                ));
            }
        }
        for witness in safe_witnesses {
            validate_safe_witness(
                &coordinate_subject,
                witness,
                capability,
                index,
                context.inventory,
                errors,
            );
        }
    }
}

fn validate_safe_witness(
    subject: &str,
    witness: &AbiSafeWitness,
    capability: SafeAbiCapability<'_>,
    index: &RustIndex,
    inventory: &CApiInventory,
    errors: &mut Vec<String>,
) {
    let (expected_raw_type, expected_raw_field) = match capability {
        SafeAbiCapability::Struct(name) | SafeAbiCapability::Callback(name) => {
            (type_path(name), None)
        }
        SafeAbiCapability::Field {
            struct_name,
            field_name,
        } => (
            type_path(struct_name),
            Some(
                field_name
                    .split('.')
                    .map(rust_binding_field_identifier)
                    .collect::<Vec<_>>()
                    .join("::"),
            ),
        ),
    };
    if witness.raw_type != expected_raw_type {
        errors.push(format!(
            "{subject} witness `{}` names raw type `{}`, expected `{expected_raw_type}`",
            witness.path, witness.raw_type
        ));
    }
    if witness.raw_field != expected_raw_field {
        errors.push(format!(
            "{subject} witness `{}` names raw field `{:?}`, expected `{:?}`",
            witness.path, witness.raw_field, expected_raw_field
        ));
    }

    match (capability, witness.kind) {
        (
            SafeAbiCapability::Struct(_),
            kind @ (AbiSafeWitnessKind::PublicType | AbiSafeWitnessKind::StructAdapter),
        ) => {
            if !witness.native_symbols.is_empty() {
                errors.push(format!(
                    "{subject} public-type witness `{}` cannot name native function symbols",
                    witness.path
                ));
            }
            let right_path_kind = match kind {
                AbiSafeWitnessKind::PublicType => index.contains_public_type_path(&witness.path),
                AbiSafeWitnessKind::StructAdapter => {
                    index.contains_public_safe_callable_path(&witness.path)
                }
                _ => unreachable!("match restricts witness kind"),
            };
            if !right_path_kind {
                errors.push(format!(
                    "{subject} witness `{}` has the wrong public path kind for {:?}",
                    witness.path, witness.kind
                ));
            }
            if !index.path_has_safe_ffi_type_witness(&witness.path, &witness.raw_type) {
                errors.push(format!(
                    "{subject} Safe Rust path `{}` has no exact witness for raw type `{}`",
                    witness.path, witness.raw_type
                ));
            }
        }
        (
            SafeAbiCapability::Field { .. },
            kind @ (AbiSafeWitnessKind::PublicField
            | AbiSafeWitnessKind::Accessor
            | AbiSafeWitnessKind::Builder),
        ) => {
            if !witness.native_symbols.is_empty() {
                errors.push(format!(
                    "{subject} field witness `{}` cannot substitute native symbols for an exact field relation",
                    witness.path
                ));
            }
            if kind == AbiSafeWitnessKind::PublicField
                && !index.contains_public_field_path(&witness.path)
            {
                errors.push(format!(
                    "{subject} witness `{}` is not an exact public field path",
                    witness.path
                ));
            }
            if matches!(
                kind,
                AbiSafeWitnessKind::Accessor | AbiSafeWitnessKind::Builder
            ) && !index.contains_public_safe_callable_path(&witness.path)
            {
                errors.push(format!(
                    "{subject} witness `{}` is not an exact public callable path for {:?}",
                    witness.path, witness.kind
                ));
            }
            let raw_field = witness.raw_field.as_deref().unwrap_or_default();
            if !index.path_has_safe_ffi_field_witness(&witness.path, &witness.raw_type, raw_field) {
                errors.push(format!(
                    "{subject} Safe Rust path `{}` has no exact witness for raw field `{}::{raw_field}`",
                    witness.path, witness.raw_type
                ));
            }
        }
        (
            SafeAbiCapability::Field {
                struct_name,
                field_name,
            },
            AbiSafeWitnessKind::CallbackAdapter,
        ) => {
            validate_field_callback_witness(
                subject,
                struct_name,
                field_name,
                witness,
                index,
                inventory,
                errors,
            );
        }
        (SafeAbiCapability::Callback(callback), AbiSafeWitnessKind::CallbackAdapter) => {
            validate_callback_witness(subject, callback, witness, index, inventory, errors);
        }
        _ => errors.push(format!(
            "{subject} witness `{}` has incompatible kind {:?}",
            witness.path, witness.kind
        )),
    }
}

fn rust_binding_field_identifier(field: &str) -> String {
    if matches!(
        field,
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
            | "try"
    ) {
        format!("{field}_")
    } else {
        field.to_owned()
    }
}

#[allow(clippy::too_many_arguments)]
fn validate_field_callback_witness(
    subject: &str,
    struct_name: &str,
    field_name: &str,
    witness: &AbiSafeWitness,
    index: &RustIndex,
    inventory: &CApiInventory,
    errors: &mut Vec<String>,
) {
    if !index.contains_public_safe_callable_path(&witness.path) {
        errors.push(format!(
            "{subject} callback-adapter witness `{}` is not an exact public callable path",
            witness.path
        ));
    }
    let raw_field = field_name.replace('.', "::");
    if !index.path_has_ffi_field_witness(&witness.path, &witness.raw_type, &raw_field) {
        errors.push(format!(
            "{subject} callback-adapter path `{}` has no exact witness for raw field `{}::{raw_field}`",
            witness.path, witness.raw_type
        ));
    }
    if witness.native_symbols.is_empty() {
        errors.push(format!(
            "{subject} callback-adapter witness `{}` has no native installation symbol",
            witness.path
        ));
        return;
    }

    let mut seen_symbols = BTreeSet::new();
    for symbol in &witness.native_symbols {
        if !seen_symbols.insert(symbol.as_str()) {
            errors.push(format!(
                "{subject} callback-adapter witness `{}` repeats native symbol `{symbol}`",
                witness.path
            ));
        }
        let related = inventory.functions.iter().any(|function| {
            function_matches_symbol(function, symbol)
                && function
                    .parameters
                    .iter()
                    .enumerate()
                    .any(|(argument_index, parameter)| {
                        declaration_mentions_identifier(parameter, struct_name)
                            && index.path_reaches_symbol_with_callable_field_argument(
                                &witness.path,
                                symbol,
                                argument_index,
                                &raw_field,
                            )
                    })
        });
        if !related {
            errors.push(format!(
                "{subject} callback-adapter witness `{}` does not pass the same `{struct_name}` value with callable field `{raw_field}` to native symbol `{symbol}`",
                witness.path
            ));
        }
        if !index.path_reaches_symbol(&witness.path, symbol) {
            errors.push(format!(
                "{subject} callback-adapter witness `{}` does not reach native symbol `{symbol}`",
                witness.path
            ));
        }
    }
}

fn validate_callback_witness(
    subject: &str,
    callback: &str,
    witness: &AbiSafeWitness,
    index: &RustIndex,
    inventory: &CApiInventory,
    errors: &mut Vec<String>,
) {
    if !index.contains_public_safe_callable_path(&witness.path) {
        errors.push(format!(
            "{subject} callback-adapter witness `{}` is not an exact public callable path",
            witness.path
        ));
    }
    if witness.native_symbols.is_empty() {
        errors.push(format!(
            "{subject} callback-adapter witness `{}` has no native installation symbol",
            witness.path
        ));
        return;
    }
    let owning_fields = inventory
        .structs
        .iter()
        .flat_map(|structure| {
            structure.fields.iter().filter_map(|field| {
                declaration_mentions_identifier(&field.signature, callback)
                    .then_some((structure.name.as_str(), field.name.as_str()))
            })
        })
        .collect::<Vec<_>>();
    let mut seen_symbols = BTreeSet::new();
    for symbol in &witness.native_symbols {
        if !seen_symbols.insert(symbol.as_str()) {
            errors.push(format!(
                "{subject} callback-adapter witness `{}` repeats native symbol `{symbol}`",
                witness.path
            ));
        }
        let related = inventory.functions.iter().any(|function| {
            if !function_matches_symbol(function, symbol) {
                return false;
            }
            let direct_callback_parameters = function
                .parameters
                .iter()
                .enumerate()
                .filter_map(|(index, parameter)| {
                    declaration_mentions_identifier(parameter, callback).then_some(index)
                })
                .collect::<Vec<_>>();
            if !direct_callback_parameters.is_empty() {
                return direct_callback_parameters.iter().any(|argument_index| {
                    index.path_reaches_symbol_with_callable_argument(
                        &witness.path,
                        symbol,
                        *argument_index,
                    )
                });
            }
            owning_fields.iter().any(|(owner, field)| {
                function
                    .parameters
                    .iter()
                    .enumerate()
                    .any(|(argument_index, parameter)| {
                        declaration_mentions_identifier(parameter, owner)
                            && index.path_reaches_symbol_with_callable_field_argument(
                                &witness.path,
                                symbol,
                                argument_index,
                                &field.replace('.', "::"),
                            )
                    })
                    && index.path_has_ffi_field_witness(
                        &witness.path,
                        &type_path(owner),
                        &field.replace('.', "::"),
                    )
            })
        });
        if !related {
            errors.push(format!(
                "{subject} callback-adapter witness `{}` names unrelated native symbol `{symbol}`",
                witness.path
            ));
        }
        if !index.path_reaches_symbol(&witness.path, symbol) {
            errors.push(format!(
                "{subject} callback-adapter witness `{}` does not reach native symbol `{symbol}`",
                witness.path
            ));
        }
    }
}

fn function_matches_symbol(function: &crate::c_api::FunctionDecl, symbol: &str) -> bool {
    function.name == symbol
        || function
            .physical_symbols
            .values()
            .any(|physical| physical == symbol)
}

fn declaration_mentions_identifier(declaration: &str, expected: &str) -> bool {
    declaration
        .split(|character: char| character != '_' && !character.is_ascii_alphanumeric())
        .any(|identifier| identifier == expected)
}

trait MappingCoordinate {
    fn mode(&self) -> &str;
    fn provider(&self) -> &str;
    fn abi_fingerprint(&self) -> &str;
}

impl MappingCoordinate for AbiTypeMapping {
    fn mode(&self) -> &str {
        &self.mode
    }

    fn provider(&self) -> &str {
        &self.provider
    }

    fn abi_fingerprint(&self) -> &str {
        &self.abi_fingerprint
    }
}

impl MappingCoordinate for AbiFieldMapping {
    fn mode(&self) -> &str {
        &self.mode
    }

    fn provider(&self) -> &str {
        &self.provider
    }

    fn abi_fingerprint(&self) -> &str {
        &self.abi_fingerprint
    }
}

fn mapping_proof_can_be_inherited<T>(previous: &[T], generated: &[T]) -> bool
where
    T: MappingCoordinate + Eq,
{
    if previous.is_empty() || generated.len() < previous.len() {
        return false;
    }
    let Some(previous_by_coordinate) = unique_mapping_coordinates(previous) else {
        return false;
    };
    let Some(generated_by_coordinate) = unique_mapping_coordinates(generated) else {
        return false;
    };

    if !previous_by_coordinate.iter().all(|(coordinate, mapping)| {
        generated_by_coordinate.get(coordinate).copied() == Some(*mapping)
    }) {
        return false;
    }
    if generated_by_coordinate.len() == previous_by_coordinate.len() {
        return true;
    }

    let reviewed_fingerprints = previous
        .iter()
        .map(MappingCoordinate::abi_fingerprint)
        .collect::<BTreeSet<_>>();
    if reviewed_fingerprints.is_empty() || reviewed_fingerprints.contains("") {
        return false;
    }
    generated_by_coordinate.iter().all(|(coordinate, mapping)| {
        previous_by_coordinate.contains_key(coordinate)
            || reviewed_fingerprints.contains(mapping.abi_fingerprint())
    })
}

fn unique_mapping_coordinates<T>(mappings: &[T]) -> Option<BTreeMap<(String, String), &T>>
where
    T: MappingCoordinate,
{
    let mut by_coordinate = BTreeMap::new();
    for mapping in mappings {
        let coordinate = (mapping.mode().to_owned(), mapping.provider().to_owned());
        if by_coordinate.insert(coordinate, mapping).is_some() {
            return None;
        }
    }
    Some(by_coordinate)
}

fn mapping_binding_index<'a>(
    subject: &str,
    mode: &str,
    provider: &str,
    routes: &'a AbiBindingRoutes,
    indexes: &'a AbiBindingIndexes,
    errors: &mut Vec<String>,
) -> Option<&'a SysAbiIndex> {
    mapping_binding(subject, mode, provider, routes, indexes, errors).map(|binding| &binding.index)
}

fn validate_binding_routes(
    routes: &AbiBindingRoutes,
    indexes: &AbiBindingIndexes,
    errors: &mut Vec<String>,
) {
    if routes.is_empty() {
        errors.push("upstream manifest has no executable binding routes".to_owned());
    }

    let mut routed_artifacts = BTreeSet::new();
    for (key, route) in routes {
        if key != &(route.mode.clone(), route.provider.clone()) {
            errors.push(format!(
                "manifest binding route key `{}/{}` does not match its declared coordinate `{}/{}`",
                key.0, key.1, route.mode, route.provider
            ));
        }
        routed_artifacts.insert(route.artifact.as_str());
        let Some(binding) = indexes.get(&route.artifact) else {
            errors.push(format!(
                "manifest binding route `{}/{}` references unknown artifact `{}`",
                route.mode, route.provider, route.artifact
            ));
            continue;
        };
        if !route_matches_binding(route, binding) {
            errors.push(format!(
                "manifest binding route `{}/{}` is incompatible with artifact `{}` coordinate {:?}/{:?}/{:?}",
                route.mode,
                route.provider,
                route.artifact,
                binding.precision,
                binding.target,
                binding.provider
            ));
        }
    }

    for artifact in indexes.keys() {
        if !routed_artifacts.contains(artifact.as_str()) {
            errors.push(format!(
                "binding artifact `{artifact}` has no executable manifest route"
            ));
        }
    }
}

fn route_matches_binding(route: &AbiBindingRoute, binding: &AbiBindingIndex) -> bool {
    if route.artifact != binding.artifact {
        return false;
    }
    let precision_matches = matches!(
        (route.mode.as_str(), binding.precision),
        ("single", Precision::Single) | ("double", Precision::Double)
    );
    let native_route = matches!(
        route.provider.as_str(),
        "source" | "system-static" | "prebuilt-static"
    );
    let wasm_route = matches!(
        route.provider.as_str(),
        "wasm-runtime" | "wasm-compile-only"
    );
    let target_matches = match binding.target {
        ArtifactTarget::Universal | ArtifactTarget::Native => native_route,
        ArtifactTarget::Wasm32UnknownUnknown | ArtifactTarget::Wasm32Wasip1 => wasm_route,
    };
    let flavor_matches = match binding.provider {
        ArtifactProvider::Universal
        | ArtifactProvider::Source
        | ArtifactProvider::SystemStatic
        | ArtifactProvider::PrebuiltStatic => native_route,
        ArtifactProvider::WasmRuntime | ArtifactProvider::WasmCompileOnly => wasm_route,
    };
    let rust_target_matches = match route.rust_target {
        RustTarget::X86_64UnknownLinuxGnu => matches!(
            binding.target,
            ArtifactTarget::Universal | ArtifactTarget::Native
        ),
        RustTarget::Wasm32UnknownUnknown => binding.target == ArtifactTarget::Wasm32UnknownUnknown,
        RustTarget::Wasm32Wasip1 => binding.target == ArtifactTarget::Wasm32Wasip1,
    };
    precision_matches && target_matches && flavor_matches && rust_target_matches
}

fn mapping_binding<'a>(
    subject: &str,
    mode: &str,
    provider: &str,
    routes: &AbiBindingRoutes,
    indexes: &'a AbiBindingIndexes,
    errors: &mut Vec<String>,
) -> Option<&'a AbiBindingIndex> {
    let key = (mode.to_owned(), provider.to_owned());
    let Some(route) = routes.get(&key) else {
        errors.push(format!(
            "{subject} has no manifest binding route for `{mode}/{provider}`"
        ));
        return None;
    };
    let Some(binding) = indexes.get(&route.artifact) else {
        errors.push(format!(
            "{subject} route `{mode}/{provider}` references unknown binding artifact `{}`",
            route.artifact
        ));
        return None;
    };
    if binding.artifact != route.artifact {
        errors.push(format!(
            "{subject} route `{mode}/{provider}` does not match the indexed artifact name"
        ));
        return None;
    }
    Some(binding)
}

fn require_route_binding<'a>(
    mode: &str,
    provider: &str,
    routes: &AbiBindingRoutes,
    indexes: &'a AbiBindingIndexes,
) -> Result<&'a AbiBindingIndex> {
    let route = routes
        .get(&(mode.to_owned(), provider.to_owned()))
        .ok_or_else(|| {
            Error::message(format!(
                "manifest has no binding route for `{mode}/{provider}`"
            ))
        })?;
    let binding = indexes.get(&route.artifact).ok_or_else(|| {
        Error::message(format!(
            "manifest binding route `{mode}/{provider}` references unknown artifact `{}`",
            route.artifact
        ))
    })?;
    if !route_matches_binding(route, binding) {
        return Err(Error::message(format!(
            "manifest binding route `{mode}/{provider}` is incompatible with artifact `{}`",
            route.artifact
        )));
    }
    Ok(binding)
}

fn route_artifact<'a>(mode: &str, provider: &str, routes: &'a AbiBindingRoutes) -> &'a str {
    routes
        .get(&(mode.to_owned(), provider.to_owned()))
        .map_or("<unrouted>", |route| route.artifact.as_str())
}

fn validate_mapping_coordinates<T: MappingCoordinate>(
    subject: &str,
    mappings: &[T],
    policy: Option<&AbiCapabilityPolicy>,
    errors: &mut Vec<String>,
) {
    let actual = mappings
        .iter()
        .map(|mapping| (mapping.mode().to_owned(), mapping.provider().to_owned()))
        .collect::<Vec<_>>();
    let unique = actual.iter().cloned().collect::<BTreeSet<_>>();
    if unique.len() != actual.len() {
        errors.push(format!("{subject} repeats an ABI mode/provider mapping"));
    }
    if let Some(policy) = policy {
        let expected = coordinates(policy).into_iter().collect::<BTreeSet<_>>();
        if unique != expected {
            errors.push(format!(
                "{subject} must map every mode/provider coordinate declared by ABI policy `{}` exactly once",
                policy.id
            ));
        }
    }
}

fn policy_reference<'a>(
    subject: &str,
    policy: &str,
    policies: &'a BTreeMap<&str, &'a AbiCapabilityPolicy>,
    used_policies: &mut BTreeSet<String>,
    errors: &mut Vec<String>,
) -> Option<&'a AbiCapabilityPolicy> {
    let Some(policy_value) = policies.get(policy).copied() else {
        errors.push(format!(
            "{subject} references unknown ABI policy `{policy}`"
        ));
        return None;
    };
    used_policies.insert(policy.to_owned());
    Some(policy_value)
}

fn require_precision_inventory_modes(
    inventories: &AbiPrecisionInventories,
    binding_routes: &AbiBindingRoutes,
) -> Result<()> {
    let expected = binding_routes
        .keys()
        .map(|(mode, _)| mode.clone())
        .collect::<BTreeSet<_>>();
    let actual = inventories.keys().cloned().collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(Error::message(format!(
            "precision C ABI inventories must cover exactly the executable modes; expected {expected:?}, found {actual:?}"
        )));
    }
    for (mode, inventory) in inventories {
        if inventory.precision.as_str() != mode {
            return Err(Error::message(format!(
                "precision C ABI inventory key `{mode}` contains `{}` declarations",
                inventory.precision.as_str()
            )));
        }
    }
    Ok(())
}

fn require_precision_inventory<'a>(
    mode: &str,
    inventories: &'a AbiPrecisionInventories,
) -> Result<&'a PrecisionCApiInventory> {
    let inventory = inventories.get(mode).ok_or_else(|| {
        Error::message(format!(
            "executable precision mode `{mode}` has no C ABI inventory"
        ))
    })?;
    if inventory.precision.as_str() != mode {
        return Err(Error::message(format!(
            "precision C ABI inventory key `{mode}` contains `{}` declarations",
            inventory.precision.as_str()
        )));
    }
    Ok(inventory)
}

fn require_exact_routed_binding_type_surfaces(
    inventory: &CApiInventory,
    precision_inventories: Option<&AbiPrecisionInventories>,
    binding_routes: &AbiBindingRoutes,
    binding_indexes: &AbiBindingIndexes,
) -> Result<()> {
    let mut checked = BTreeSet::new();
    for route in binding_routes.values() {
        if !checked.insert((route.artifact.as_str(), route.mode.as_str())) {
            continue;
        }
        let binding = binding_indexes.get(&route.artifact).ok_or_else(|| {
            Error::message(format!(
                "binding route `{}/{}` references missing artifact `{}`",
                route.mode, route.provider, route.artifact
            ))
        })?;
        let precision_inventory = precision_inventories
            .map(|inventories| require_precision_inventory(&route.mode, inventories))
            .transpose()?;
        require_exact_binding_type_surface(inventory, precision_inventory, binding)?;
    }
    Ok(())
}

fn require_exact_binding_type_surface(
    inventory: &CApiInventory,
    precision_inventory: Option<&PrecisionCApiInventory>,
    binding: &AbiBindingIndex,
) -> Result<()> {
    let mut expected_aggregates = BTreeSet::new();
    for declaration in &inventory.structs {
        let path = type_path(&declaration.name);
        let resolved = binding.index.resolved_type_path(&path)?.ok_or_else(|| {
            Error::message(format!(
                "generated Rust binding has no type for C struct `{}`",
                declaration.name
            ))
        })?;
        expected_aggregates.insert(resolved);
        let projections = require_exact_struct_projection(declaration, binding, &path)?;
        expected_aggregates.extend(
            projections
                .iter()
                .flat_map(|projection| projection.steps.iter())
                .map(|step| step.owner_type.clone()),
        );
    }
    if let Some(precision_inventory) = precision_inventory {
        for opaque in &precision_inventory.opaques {
            let path = type_path(&opaque.name);
            expected_aggregates.insert(path.clone());
            let Some(aggregate) = binding.surface.aggregates.get(&path) else {
                continue;
            };
            if !aggregate.bindgen_opaque {
                return Err(Error::message(format!(
                    "generated Rust aggregate `{path}` for opaque C type `{}` is not the exact bindgen opaque repr-C struct",
                    opaque.name
                )));
            }
        }
    }

    let actual_aggregates = binding
        .surface
        .aggregates
        .keys()
        .filter(|path| is_public_b2_type_path(path))
        .cloned()
        .collect::<BTreeSet<_>>();
    if actual_aggregates != expected_aggregates {
        let missing = expected_aggregates
            .difference(&actual_aggregates)
            .cloned()
            .collect::<Vec<_>>();
        let extra = actual_aggregates
            .difference(&expected_aggregates)
            .cloned()
            .collect::<Vec<_>>();
        return Err(Error::message(format!(
            "binding artifact `{}` public b2 aggregates do not exactly match the C aggregate inventory; missing {missing:?}, extra {extra:?}",
            binding.artifact
        )));
    }

    let expected_callbacks = inventory
        .callbacks
        .iter()
        .map(|callback| type_path(&callback.name))
        .collect::<BTreeSet<_>>();
    let mut actual_callbacks = BTreeSet::new();
    for path in binding
        .surface
        .type_aliases
        .iter()
        .filter(|path| is_public_b2_type_path(path))
    {
        if binding.index.callback_abi_shape(path)?.is_some() {
            actual_callbacks.insert(path.clone());
        }
    }
    if actual_callbacks != expected_callbacks {
        let missing = expected_callbacks
            .difference(&actual_callbacks)
            .cloned()
            .collect::<Vec<_>>();
        let extra = actual_callbacks
            .difference(&expected_callbacks)
            .cloned()
            .collect::<Vec<_>>();
        return Err(Error::message(format!(
            "binding artifact `{}` public b2 callback aliases do not exactly match the C callback inventory; missing {missing:?}, extra {extra:?}",
            binding.artifact
        )));
    }
    Ok(())
}

fn validate_referenced_binding_types(
    inventory: &CApiInventory,
    precision_inventories: Option<&AbiPrecisionInventories>,
    binding_routes: &AbiBindingRoutes,
    binding_indexes: &AbiBindingIndexes,
    errors: &mut Vec<String>,
) {
    let Some(precision_inventories) = precision_inventories else {
        return;
    };
    let mut checked = BTreeSet::new();
    for route in binding_routes.values() {
        if !checked.insert((route.artifact.as_str(), route.mode.as_str())) {
            continue;
        }
        let Some(binding) = binding_indexes.get(&route.artifact) else {
            continue;
        };
        let precision_inventory =
            match require_precision_inventory(&route.mode, precision_inventories) {
                Ok(inventory) => Some(inventory),
                Err(error) => {
                    errors.push(error.to_string());
                    continue;
                }
            };
        if let Err(error) =
            require_exact_binding_type_surface(inventory, precision_inventory, binding)
        {
            errors.push(format!(
                "binding artifact `{}` at `{}/{}`: {error}",
                route.artifact, route.mode, route.provider
            ));
        }
    }
}

fn is_public_b2_type_path(path: &str) -> bool {
    path.rsplit("::")
        .next()
        .is_some_and(|name| name.starts_with("b2"))
}

fn require_struct_abi_fingerprint(
    declaration: &StructDecl,
    c_inventory: &PrecisionCApiInventory,
    binding: &AbiBindingIndex,
    rust_path: &str,
) -> Result<String> {
    let projections = require_exact_struct_projection(declaration, binding, rust_path)?;
    let rust_index = &binding.index;
    let c_shape = c_inventory.type_shape(&declaration.name).ok_or_else(|| {
        Error::message(format!(
            "{}-precision C ABI inventory has no effective type `{}`",
            c_inventory.precision.as_str(),
            declaration.name
        ))
    })?;
    let rust_shape = rust_index.type_abi_shape(rust_path)?.ok_or_else(|| {
        Error::message(format!(
            "generated Rust binding has no ABI shape for `{rust_path}`"
        ))
    })?;
    let c_fingerprint = c_shape.fingerprint();
    let rust_fingerprint = rust_shape.fingerprint();
    if c_fingerprint == rust_fingerprint {
        return Ok(c_fingerprint);
    }

    // Bindgen materializes anonymous C unions/structs as named wrapper fields. Compare the same
    // public C leaf inventory through the exact generated access projections to normalize that
    // representational difference without accepting a primitive or callback type mismatch.
    let (projected_c, projected_rust) = projected_struct_shapes(
        declaration,
        c_inventory,
        rust_index,
        rust_path,
        &projections,
    )?;
    let projected_c_fingerprint = projected_c.fingerprint();
    let projected_rust_fingerprint = projected_rust.fingerprint();
    if projected_c_fingerprint != projected_rust_fingerprint {
        return Err(Error::message(format!(
            "{}-precision ABI shape mismatch for `{}`: C `{projected_c_fingerprint}`, generated Rust `{projected_rust_fingerprint}`",
            c_inventory.precision.as_str(),
            declaration.name
        )));
    }
    Ok(projected_c_fingerprint)
}

fn require_exact_struct_projection(
    declaration: &StructDecl,
    binding: &AbiBindingIndex,
    rust_path: &str,
) -> Result<Vec<SysAbiAccessProjection>> {
    let resolved_root = binding
        .index
        .resolved_type_path(rust_path)?
        .ok_or_else(|| {
            Error::message(format!(
                "generated Rust binding has no resolved ABI type for `{rust_path}`"
            ))
        })?;
    let root = binding
        .surface
        .aggregates
        .get(&resolved_root)
        .ok_or_else(|| {
            Error::message(format!(
                "generated Rust binding resolves C struct `{}` to non-aggregate `{resolved_root}`",
                declaration.name
            ))
        })?;
    if root.kind != AbiAggregateKind::Struct {
        return Err(Error::message(format!(
            "generated Rust aggregate `{resolved_root}` for C struct `{}` is a {}, expected struct",
            declaration.name,
            root.kind.as_str()
        )));
    }

    let projections = declaration
        .fields
        .iter()
        .map(|field| require_field_projection(&binding.index, rust_path, &field.name))
        .collect::<Result<Vec<_>>>()?;
    let mut projected_fields = BTreeMap::<String, Vec<String>>::new();
    projected_fields.entry(resolved_root).or_default();
    let mut overlay_locations = BTreeMap::<String, (Vec<SysAbiAccessStep>, String)>::new();
    let mut groups_by_location = BTreeMap::<(Vec<SysAbiAccessStep>, String), String>::new();
    for (field, projection) in declaration.fields.iter().zip(&projections) {
        for (step_index, step) in projection.steps.iter().enumerate() {
            if !step.field.starts_with("__bindgen_anon_") {
                continue;
            }
            let Some(next_step) = projection.steps.get(step_index + 1) else {
                continue;
            };
            let target_is_struct = binding
                .surface
                .aggregates
                .get(&next_step.owner_type)
                .is_some_and(|aggregate| aggregate.kind == AbiAggregateKind::Struct);
            let proven_union_alternative = binding
                .surface
                .aggregates
                .get(&step.owner_type)
                .is_some_and(|aggregate| aggregate.kind == AbiAggregateKind::Union)
                && field
                    .overlays
                    .iter()
                    .any(|overlay| overlay.alternative.starts_with("anonymous_struct@"));
            if target_is_struct && !proven_union_alternative {
                return Err(Error::message(format!(
                    "generated Rust projection for `{}::{}` crosses anonymous struct wrapper `{}::{}` without an exact C anonymous-struct overlay proof",
                    declaration.name, field.name, step.owner_type, step.field
                )));
            }
        }
        for step in &projection.steps {
            let owner_fields = projected_fields.entry(step.owner_type.clone()).or_default();
            if !owner_fields.contains(&step.field) {
                owner_fields.push(step.field.clone());
            }
        }

        let union_steps = projection
            .steps
            .iter()
            .enumerate()
            .filter_map(|(index, step)| {
                binding
                    .surface
                    .aggregates
                    .get(&step.owner_type)
                    .is_some_and(|aggregate| aggregate.kind == AbiAggregateKind::Union)
                    .then_some((index, step))
            })
            .collect::<Vec<_>>();
        if union_steps.len() != field.overlays.len() {
            return Err(Error::message(format!(
                "generated Rust projection for `{}::{}` crosses {} union owner(s), but the C declaration records {} overlay group(s); struct/union kind drift is not allowed",
                declaration.name,
                field.name,
                union_steps.len(),
                field.overlays.len()
            )));
        }
        for (overlay, (step_index, step)) in field.overlays.iter().zip(union_steps) {
            if !overlay.alternative.starts_with("anonymous_")
                && step.field != overlay.alternative
                && step.field.strip_suffix('_') != Some(overlay.alternative.as_str())
            {
                return Err(Error::message(format!(
                    "generated Rust union `{}` selects field `{}`, expected C overlay alternative `{}` for `{}::{}`",
                    step.owner_type, step.field, overlay.alternative, declaration.name, field.name
                )));
            }
            let location = (
                projection.steps[..step_index].to_vec(),
                step.owner_type.clone(),
            );
            if let Some(previous) =
                overlay_locations.insert(overlay.group.clone(), location.clone())
                && previous != location
            {
                return Err(Error::message(format!(
                    "C overlay group `{}` maps to multiple generated Rust union locations: `{}` and `{}`",
                    overlay.group,
                    describe_union_location(&previous),
                    describe_union_location(&location)
                )));
            }
            if let Some(previous) =
                groups_by_location.insert(location.clone(), overlay.group.clone())
                && previous != overlay.group
            {
                return Err(Error::message(format!(
                    "generated Rust union location `{}` merges distinct C overlay groups `{previous}` and `{}`",
                    describe_union_location(&location),
                    overlay.group
                )));
            }
        }
    }

    for (owner, projected) in projected_fields {
        let aggregate = binding.surface.aggregates.get(&owner).ok_or_else(|| {
            Error::message(format!(
                "generated Rust projection owner `{owner}` has no aggregate surface"
            ))
        })?;
        let generated = aggregate.fields.iter().cloned().collect::<BTreeSet<_>>();
        let expected = projected.iter().cloned().collect::<BTreeSet<_>>();
        let order_mismatch = aggregate.kind == AbiAggregateKind::Struct
            && aggregate.fields.as_slice() != projected.as_slice();
        if generated != expected || order_mismatch {
            let missing = expected.difference(&generated).cloned().collect::<Vec<_>>();
            let extra = generated.difference(&expected).cloned().collect::<Vec<_>>();
            if missing.is_empty() && extra.is_empty() {
                return Err(Error::message(format!(
                    "generated Rust struct `{owner}` field order does not match the exact C field projection for `{}`; expected {projected:?}, generated {:?}",
                    declaration.name, aggregate.fields
                )));
            }
            return Err(Error::message(format!(
                "generated Rust aggregate `{owner}` is not covered by the exact C field projection for `{}`; missing generated fields {missing:?}, unmatched generated fields {extra:?}",
                declaration.name
            )));
        }
    }
    Ok(projections)
}

fn describe_union_location(location: &(Vec<SysAbiAccessStep>, String)) -> String {
    let (prefix, owner) = location;
    let access = prefix
        .iter()
        .map(|step| step.field.as_str())
        .collect::<Vec<_>>()
        .join("::");
    if access.is_empty() {
        owner.clone()
    } else {
        format!("{access} ({owner})")
    }
}

fn projected_struct_shapes(
    declaration: &StructDecl,
    c_inventory: &PrecisionCApiInventory,
    rust_index: &SysAbiIndex,
    rust_path: &str,
    projections: &[SysAbiAccessProjection],
) -> Result<(AbiTypeShape, AbiTypeShape)> {
    let c_root = c_inventory.type_shape(&declaration.name).ok_or_else(|| {
        Error::message(format!(
            "{}-precision C ABI inventory has no effective type `{}`",
            c_inventory.precision.as_str(),
            declaration.name
        ))
    })?;
    let mut c_fields = Vec::with_capacity(declaration.fields.len());
    let mut rust_fields = Vec::with_capacity(declaration.fields.len());
    for (field, projection) in declaration.fields.iter().zip(projections) {
        let c_shape = effective_field_shape(c_root, &field.name).ok_or_else(|| {
            Error::message(format!(
                "{}-precision C ABI type `{}` has no field `{}`",
                c_inventory.precision.as_str(),
                declaration.name,
                field.name
            ))
        })?;
        let rust_shape = rust_index
            .field_access_abi_shape(projection)?
            .ok_or_else(|| {
                Error::message(format!(
                    "generated Rust binding has no ABI shape for projected field `{rust_path}::{}`",
                    field.name
                ))
            })?;
        c_fields.push(AbiFieldShape {
            name: field.name.clone(),
            shape: c_shape.clone(),
            overlays: field.overlays.clone(),
        });
        rust_fields.push(AbiFieldShape {
            name: field.name.clone(),
            shape: rust_shape,
            overlays: field.overlays.clone(),
        });
    }
    Ok((
        AbiTypeShape::Aggregate { fields: c_fields },
        AbiTypeShape::Aggregate {
            fields: rust_fields,
        },
    ))
}

fn require_field_abi_fingerprint(
    struct_name: &str,
    field_name: &str,
    c_inventory: &PrecisionCApiInventory,
    rust_index: &SysAbiIndex,
    projection: &SysAbiAccessProjection,
) -> Result<String> {
    let c_root = c_inventory.type_shape(struct_name).ok_or_else(|| {
        Error::message(format!(
            "{}-precision C ABI inventory has no effective type `{struct_name}`",
            c_inventory.precision.as_str()
        ))
    })?;
    let c_shape = effective_field_shape(c_root, field_name).ok_or_else(|| {
        Error::message(format!(
            "{}-precision C ABI type `{struct_name}` has no field `{field_name}`",
            c_inventory.precision.as_str()
        ))
    })?;
    let rust_shape = rust_index
        .field_access_abi_shape(projection)?
        .ok_or_else(|| {
            Error::message(format!(
                "generated Rust binding has no ABI shape for projected field `{struct_name}::{field_name}`"
            ))
        })?;
    let c_fingerprint = c_shape.fingerprint();
    let rust_fingerprint = rust_shape.fingerprint();
    if c_fingerprint != rust_fingerprint {
        return Err(Error::message(format!(
            "{}-precision ABI field shape mismatch for `{struct_name}::{field_name}`: C `{c_fingerprint}`, generated Rust `{rust_fingerprint}`",
            c_inventory.precision.as_str()
        )));
    }
    Ok(c_fingerprint)
}

fn effective_field_shape<'a>(root: &'a AbiTypeShape, field_path: &str) -> Option<&'a AbiTypeShape> {
    let segments = field_path.split('.').collect::<Vec<_>>();
    effective_field_shape_segments(root, &segments)
}

fn effective_field_shape_segments<'a>(
    shape: &'a AbiTypeShape,
    segments: &[&str],
) -> Option<&'a AbiTypeShape> {
    let AbiTypeShape::Aggregate { fields } = shape else {
        return None;
    };
    let complete_path = segments.join(".");
    if let Some(field) = fields.iter().find(|field| field.name == complete_path) {
        return Some(&field.shape);
    }
    let (first, remaining) = segments.split_first()?;
    let field = fields.iter().find(|field| field.name == *first)?;
    if remaining.is_empty() {
        Some(&field.shape)
    } else {
        effective_field_shape_segments(&field.shape, remaining)
    }
}

fn require_callback_abi_fingerprint(
    callback_name: &str,
    c_inventory: &PrecisionCApiInventory,
    rust_index: &SysAbiIndex,
    rust_path: &str,
) -> Result<String> {
    let c_fingerprint = c_inventory
        .callback(callback_name)
        .map(|callback| callback.fingerprint.clone())
        .ok_or_else(|| {
            Error::message(format!(
                "{}-precision C ABI inventory has no callback `{callback_name}`",
                c_inventory.precision.as_str()
            ))
        })?;
    let rust_fingerprint = rust_index
        .callback_abi_fingerprint(rust_path)?
        .ok_or_else(|| {
            Error::message(format!(
                "generated Rust binding has no callback ABI shape for `{rust_path}`"
            ))
        })?;
    if c_fingerprint != rust_fingerprint {
        return Err(Error::message(format!(
            "{}-precision callback ABI mismatch for `{callback_name}`: C `{c_fingerprint}`, generated Rust `{rust_fingerprint}`",
            c_inventory.precision.as_str()
        )));
    }
    Ok(c_fingerprint)
}

fn validate_struct_mapping_fingerprint(
    subject: &str,
    declaration: &StructDecl,
    mapping: &AbiTypeMapping,
    precision_inventories: &AbiPrecisionInventories,
    binding: &AbiBindingIndex,
    errors: &mut Vec<String>,
) {
    match require_precision_inventory(&mapping.mode, precision_inventories).and_then(|inventory| {
        require_struct_abi_fingerprint(
            declaration,
            inventory,
            binding,
            &type_path(&declaration.name),
        )
    }) {
        Ok(expected) if mapping.abi_fingerprint != expected => errors.push(format!(
            "{subject} at `{}/{}` records ABI fingerprint `{}`, expected `{expected}`",
            mapping.mode, mapping.provider, mapping.abi_fingerprint
        )),
        Ok(_) => {}
        Err(error) => errors.push(format!(
            "{subject} at `{}/{}`: {error}",
            mapping.mode, mapping.provider
        )),
    }
}

#[allow(clippy::too_many_arguments)]
fn validate_field_mapping_fingerprint(
    subject: &str,
    struct_name: &str,
    field_name: &str,
    mapping: &AbiFieldMapping,
    precision_inventories: &AbiPrecisionInventories,
    rust_index: &SysAbiIndex,
    projection: &SysAbiAccessProjection,
    errors: &mut Vec<String>,
) {
    match require_precision_inventory(&mapping.mode, precision_inventories).and_then(|inventory| {
        require_field_abi_fingerprint(struct_name, field_name, inventory, rust_index, projection)
    }) {
        Ok(expected) if mapping.abi_fingerprint != expected => errors.push(format!(
            "{subject} at `{}/{}` records ABI fingerprint `{}`, expected `{expected}`",
            mapping.mode, mapping.provider, mapping.abi_fingerprint
        )),
        Ok(_) => {}
        Err(error) => errors.push(format!(
            "{subject} at `{}/{}`: {error}",
            mapping.mode, mapping.provider
        )),
    }
}

fn validate_callback_mapping_fingerprint(
    subject: &str,
    callback_name: &str,
    mapping: &AbiTypeMapping,
    precision_inventories: &AbiPrecisionInventories,
    rust_index: &SysAbiIndex,
    errors: &mut Vec<String>,
) {
    match require_precision_inventory(&mapping.mode, precision_inventories).and_then(|inventory| {
        require_callback_abi_fingerprint(
            callback_name,
            inventory,
            rust_index,
            &type_path(callback_name),
        )
    }) {
        Ok(expected) if mapping.abi_fingerprint != expected => errors.push(format!(
            "{subject} at `{}/{}` records ABI fingerprint `{}`, expected `{expected}`",
            mapping.mode, mapping.provider, mapping.abi_fingerprint
        )),
        Ok(_) => {}
        Err(error) => errors.push(format!(
            "{subject} at `{}/{}`: {error}",
            mapping.mode, mapping.provider
        )),
    }
}

fn require_resolved_type(index: &SysAbiIndex, path: &str, c_name: &str) -> Result<String> {
    index.resolved_type_path(path)?.ok_or_else(|| {
        Error::message(format!(
            "ABI declaration `{c_name}` has no exact generated Rust type path `{path}`"
        ))
    })
}

fn require_field_projection(
    index: &SysAbiIndex,
    root_path: &str,
    c_field: &str,
) -> Result<SysAbiAccessProjection> {
    let direct_segments = c_field.split('.').collect::<Vec<_>>();
    if let Some(projection) = index.project_field_access(root_path, &direct_segments)? {
        return Ok(projection);
    }
    let Some((last, prefix)) = direct_segments.split_last() else {
        return Err(Error::message("ABI field path cannot be empty"));
    };
    let escaped_last = format!("{last}_");
    let mut escaped = prefix.to_vec();
    escaped.push(&escaped_last);
    index
        .project_field_access(root_path, &escaped)?
        .ok_or_else(|| {
            Error::message(format!(
                "ABI field `{root_path}::{c_field}` has no unique generated Rust access chain"
            ))
        })
}

fn field_mapping(
    mode: &str,
    provider: &str,
    projection: &SysAbiAccessProjection,
    abi_fingerprint: String,
) -> AbiFieldMapping {
    AbiFieldMapping {
        mode: mode.to_owned(),
        provider: provider.to_owned(),
        root_path: projection.root_type.clone(),
        resolved_root_path: projection.resolved_root_type.clone(),
        steps: projection
            .steps
            .iter()
            .map(|step| AbiAccessStep {
                owner_type: step.owner_type.clone(),
                field: step.field.clone(),
            })
            .collect(),
        abi_fingerprint,
    }
}

fn validate_referenced_binding_functions(
    inventory: &CApiInventory,
    expected_function_count: Option<usize>,
    binding_routes: &AbiBindingRoutes,
    binding_indexes: &AbiBindingIndexes,
    function_symbols: &AbiFunctionSymbols,
    errors: &mut Vec<String>,
) {
    let expected_functions = inventory
        .functions
        .iter()
        .map(|function| function.name.as_str())
        .collect::<BTreeSet<_>>();
    for (coordinate, route) in binding_routes {
        let Some(binding) = binding_indexes.get(&route.artifact) else {
            continue;
        };
        if let Err(error) = binding
            .surface
            .require_wasm_import_modules(binding.precision, expected_function_count.is_some())
        {
            errors.push(format!(
                "binding artifact `{}` at `{}/{}`: {error}",
                route.artifact, coordinate.0, coordinate.1
            ));
        }
        let Some(symbols) = function_symbols.get(coordinate) else {
            errors.push(format!(
                "binding route `{}/{}` has no physical function symbol map",
                coordinate.0, coordinate.1
            ));
            continue;
        };
        let actual_functions = symbols.keys().map(String::as_str).collect::<BTreeSet<_>>();
        if actual_functions != expected_functions {
            errors.push(format!(
                "binding route `{}/{}` physical symbol map does not cover the exact active C function inventory",
                coordinate.0, coordinate.1
            ));
        }
        let expected_symbols = symbols.values().cloned().collect::<BTreeSet<_>>();
        let actual_symbols = &binding.surface.functions;
        let missing_symbols = expected_symbols
            .difference(actual_symbols)
            .cloned()
            .collect::<Vec<_>>();
        let extra_symbols = actual_symbols
            .difference(&expected_symbols)
            .cloned()
            .collect::<Vec<_>>();
        if !missing_symbols.is_empty() || !extra_symbols.is_empty() {
            errors.push(format!(
                "binding artifact `{}` public extern-C b2* functions do not exactly match header-derived physical symbols at `{}/{}`; missing {missing_symbols:?}, extra {extra_symbols:?}",
                route.artifact, coordinate.0, coordinate.1
            ));
        }
        if let Some(expected_count) = expected_function_count {
            if expected_symbols.len() != expected_count {
                errors.push(format!(
                    "binding route `{}/{}` header-derived physical symbol count is {}, expected exactly {expected_count}",
                    coordinate.0,
                    coordinate.1,
                    expected_symbols.len()
                ));
            }
            if actual_symbols.len() != expected_count {
                errors.push(format!(
                    "binding artifact `{}` public extern-C b2* function count is {}, expected exactly {expected_count}",
                    route.artifact,
                    actual_symbols.len()
                ));
            }
        }
        for (logical_name, physical_symbol) in symbols {
            let path = type_path(physical_symbol);
            if !binding.index.contains_function_path(&path) {
                errors.push(format!(
                    "binding artifact `{}` is missing active C function `{logical_name}` physical symbol `{physical_symbol}` at canonical path `{path}`",
                    route.artifact
                ));
            }
        }
    }
}

fn sys_projection(mapping: &AbiFieldMapping) -> SysAbiAccessProjection {
    SysAbiAccessProjection {
        root_type: mapping.root_path.clone(),
        resolved_root_type: mapping.resolved_root_path.clone(),
        steps: mapping
            .steps
            .iter()
            .map(|step| SysAbiAccessStep {
                owner_type: step.owner_type.clone(),
                field: step.field.clone(),
            })
            .collect(),
    }
}

fn coordinates(policy: &AbiCapabilityPolicy) -> Vec<(String, String)> {
    policy
        .modes
        .iter()
        .flat_map(|mode| {
            policy
                .providers
                .iter()
                .map(move |provider| (mode.clone(), provider.clone()))
        })
        .collect()
}

fn type_path(name: &str) -> String {
    format!("boxdd_sys::ffi::{name}")
}

fn validate_capability_rationale(subject: &str, rationale: &str, errors: &mut Vec<String>) {
    if !has_rationale(rationale) {
        errors.push(format!("{subject} needs a specific reviewed rationale"));
    }
}

fn validate_registry_values(
    subject: &str,
    registry: &str,
    values: &[String],
    allowed: &BTreeSet<&str>,
    errors: &mut Vec<String>,
) {
    let mut seen = BTreeSet::new();
    for value in values {
        if !allowed.contains(value.as_str()) {
            errors.push(format!(
                "{subject} references unsupported {registry} `{value}`"
            ));
        }
        if !seen.insert(value.as_str()) {
            errors.push(format!("{subject} repeats {registry} `{value}`"));
        }
    }
}

fn value_set(values: &[String]) -> BTreeSet<&str> {
    values.iter().map(String::as_str).collect()
}

fn has_rationale(value: &str) -> bool {
    let value = value.trim();
    value.len() >= 24
        && !matches!(
            value.to_ascii_lowercase().as_str(),
            "todo" | "tbd" | "deferred"
        )
}

fn is_policy_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--")
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, fs};

    use tempfile::tempdir;

    use super::{
        AbiBindingIndex, AbiBindingIndexes, AbiBindingRoute, AbiCapabilityPolicy,
        AbiPrecisionInventories, AbiTypeMapping, declaration_mentions_identifier,
        inherit_policy_route_matrix, map_precision_inventory, mapping_proof_can_be_inherited,
    };
    use crate::{
        c_api::{CAbiPrecision, parse_headers, parse_headers_for_precision},
        commands::api_coverage::Classification,
        commands::upstream_sync::{ArtifactProvider, ArtifactTarget, Precision, RustTarget},
    };

    #[test]
    fn callback_owner_signatures_require_exact_typedef_identifiers() {
        assert!(declaration_mentions_identifier("b2Foo callback", "b2Foo"));
        assert!(!declaration_mentions_identifier(
            "b2FooExtended callback",
            "b2Foo"
        ));
        assert!(!declaration_mentions_identifier(
            "b2Foo callback",
            "b2FooExtended"
        ));
    }

    #[test]
    fn reviewed_policy_inherits_current_route_matrix_without_changing_review() {
        let reviewed = AbiCapabilityPolicy {
            id: "safe-abi-adapter".to_owned(),
            classification: Classification::Safe,
            rationale: "The reviewed Safe adapter has an exact native ABI witness.".to_owned(),
            modes: vec!["single".to_owned()],
            providers: vec!["source".to_owned()],
            availability: vec!["always".to_owned()],
            evidence: vec!["abi-header-parser".to_owned()],
        };
        let current = AbiCapabilityPolicy {
            modes: vec!["single".to_owned(), "double".to_owned()],
            providers: vec!["source".to_owned()],
            ..reviewed.clone()
        };

        let inherited = inherit_policy_route_matrix(&reviewed, &current);
        assert_eq!(inherited.modes, current.modes);
        assert_eq!(inherited.providers, current.providers);
        assert_eq!(inherited.availability, current.availability);
        assert_eq!(inherited.classification, reviewed.classification);
        assert_eq!(inherited.rationale, reviewed.rationale);
        assert_eq!(inherited.evidence, reviewed.evidence);
    }

    #[test]
    fn reviewed_mapping_requires_exact_existing_proof_and_equivalent_added_modes() {
        let single = AbiTypeMapping {
            mode: "single".to_owned(),
            provider: "source".to_owned(),
            path: "boxdd_sys::ffi::b2Pos".to_owned(),
            resolved_path: "boxdd_sys::ffi::b2Vec2".to_owned(),
            abi_fingerprint: "blake3-v1:same".to_owned(),
        };
        assert!(mapping_proof_can_be_inherited(
            std::slice::from_ref(&single),
            std::slice::from_ref(&single),
        ));

        let mut double = single.clone();
        double.mode = "double".to_owned();
        double.resolved_path = "boxdd_sys::ffi::b2Pos".to_owned();
        assert!(mapping_proof_can_be_inherited(
            std::slice::from_ref(&single),
            &[single.clone(), double.clone()],
        ));

        double.abi_fingerprint = "blake3-v1:double".to_owned();
        assert!(!mapping_proof_can_be_inherited(
            std::slice::from_ref(&single),
            &[single.clone(), double],
        ));

        let mut legacy = single.clone();
        legacy.abi_fingerprint.clear();
        let mut legacy_double = legacy.clone();
        legacy_double.mode = "double".to_owned();
        assert!(!mapping_proof_can_be_inherited(
            std::slice::from_ref(&legacy),
            &[legacy.clone(), legacy_double],
        ));
    }

    #[test]
    fn precision_mapping_rejects_f64_field_bound_as_f32() {
        let error = map_fixture(
            "typedef struct b2Pos { double x; } b2Pos;",
            "#[repr(C)] pub struct b2Pos { pub x: f32 }",
            CAbiPrecision::Double,
        )
        .expect_err("different primitive field widths must fail closed");
        assert!(
            error.to_string().contains("ABI shape mismatch"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn precision_mapping_rejects_unmatched_generated_struct_fields() {
        let error = map_fixture(
            "typedef struct b2Pos { float x; } b2Pos;",
            "#[repr(C)] pub struct b2Pos { pub x: f32, pub extra: u64 }",
            CAbiPrecision::Single,
        )
        .expect_err("generated fields absent from C must fail closed");
        assert!(
            error
                .to_string()
                .contains("unmatched generated fields [\"extra\"]"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn precision_mapping_rejects_root_struct_union_kind_drift() {
        let error = map_fixture(
            "typedef struct b2Pos { float x; } b2Pos;",
            "#[repr(C)] pub union b2Pos { pub x: f32 }",
            CAbiPrecision::Single,
        )
        .expect_err("a Rust union cannot represent a C struct");
        assert!(
            error.to_string().contains("is a union, expected struct"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn precision_mapping_rejects_anonymous_union_kind_drift() {
        let error = map_fixture(
            "typedef struct b2Example { union { float x; unsigned int y; }; } b2Example;",
            r#"
                #[repr(C)]
                pub struct b2Example {
                    pub __bindgen_anon_1: b2Example__bindgen_ty_1,
                }
                #[repr(C)]
                pub struct b2Example__bindgen_ty_1 {
                    pub x: f32,
                    pub y: u32,
                }
            "#,
            CAbiPrecision::Single,
        )
        .expect_err("an anonymous C union cannot become a Rust struct");
        assert!(
            error
                .to_string()
                .contains("without an exact C anonymous-struct overlay proof"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn precision_mapping_rejects_reordered_repr_c_struct_fields() {
        let error = map_fixture(
            "typedef struct b2Pair { unsigned char first; unsigned long long second; } b2Pair;",
            "#[repr(C)] pub struct b2Pair { pub second: u64, pub first: u8 }",
            CAbiPrecision::Single,
        )
        .expect_err("repr-C field order must match the C declaration exactly");
        assert!(
            error.to_string().contains("field order does not match"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn binding_surface_rejects_cfg_gated_abi_fields() {
        let error = map_fixture(
            "typedef struct b2Pos { float x; } b2Pos;",
            "#[repr(C)] pub struct b2Pos { #[cfg(any())] pub x: f32 }",
            CAbiPrecision::Single,
        )
        .expect_err("cfg-gated ABI fields must not count as generated surface");
        assert!(
            error.to_string().contains("uses unsupported `#[cfg]`"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn binding_surface_rejects_cfg_in_callback_bare_fn_parameters() {
        let error = binding_fixture(
            r#"
                pub type b2Callback = Option<
                    unsafe extern "C" fn(#[cfg(any())] value: u32)
                >;
            "#,
            Precision::Single,
        )
        .expect_err("cfg-gated callback parameters must fail closed");
        assert!(
            error.to_string().contains("uses unsupported `#[cfg]`"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn binding_surface_rejects_cfg_on_callback_bare_fn_variadics() {
        let error = binding_fixture(
            r#"
                pub type b2Callback = Option<
                    unsafe extern "C" fn(value: u32, #[cfg(any())] ...)
                >;
            "#,
            Precision::Single,
        )
        .expect_err("cfg-gated callback variadics must fail closed");
        assert!(
            error.to_string().contains("uses unsupported `#[cfg]`"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn binding_surface_rejects_cfg_inside_foreign_function_types_and_variadics() {
        for binding in [
            r#"
                unsafe extern "C" {
                    pub fn b2Run(
                        callback: Option<unsafe extern "C" fn(#[cfg(any())] value: u32)>
                    );
                }
            "#,
            r#"
                unsafe extern "C" {
                    pub fn b2Run(value: u32, #[cfg(any())] ...);
                }
            "#,
        ] {
            let error = binding_fixture(binding, Precision::Single)
                .expect_err("cfg-gated foreign ABI parameters must fail closed");
            assert!(
                error.to_string().contains("uses unsupported `#[cfg]`"),
                "unexpected error: {error}"
            );
        }
    }

    #[test]
    fn binding_surface_rejects_safe_or_non_c_callback_function_pointers() {
        for binding in [
            "pub type b2Callback = Option<extern \"C\" fn(value: u32)>;",
            "pub type b2Callback = Option<unsafe fn(value: u32)>;",
        ] {
            let error = binding_fixture(binding, Precision::Single)
                .expect_err("C callbacks must remain unsafe extern-C function pointers");
            assert!(
                error
                    .to_string()
                    .contains("is not `unsafe extern \"C\" fn`"),
                "unexpected error: {error}"
            );
        }
    }

    #[test]
    fn binding_surface_rejects_link_ordinal_symbol_remapping() {
        let error = binding_fixture(
            r#"
                unsafe extern "C" {
                    #[link_ordinal(7)]
                    pub fn b2Run();
                }
            "#,
            Precision::Single,
        )
        .expect_err("ordinal imports bypass physical symbol validation");
        assert!(
            error
                .to_string()
                .contains("uses unsupported `#[link_ordinal]`"),
            "unexpected error: {error}"
        );

        let renamed = binding_fixture(
            r#"unsafe extern "C" { #[link_name = "b2Forged"] pub fn forged(); }"#,
            Precision::Single,
        )
        .expect_err("a non-b2 Rust name must not remap to a b2 physical symbol");
        assert!(
            renamed
                .to_string()
                .contains("uses unsupported `#[link_name]`"),
            "unexpected error: {renamed}"
        );
    }

    #[test]
    fn binding_surface_rejects_wrong_or_partial_wasm_import_modules() {
        let wrong = binding_fixture(
            r#"
                #[link(wasm_import_module = "wrong-module")]
                unsafe extern "C" { pub fn b2Run(); }
            "#,
            Precision::Single,
        )
        .expect_err("wrong precision module must fail closed");
        assert!(
            wrong.to_string().contains("expected `box2d-sys-v0-single`"),
            "unexpected error: {wrong}"
        );

        let partial = binding_fixture(
            r#"
                #[link(wasm_import_module = "box2d-sys-v0-single")]
                unsafe extern "C" { pub fn b2First(); }
                unsafe extern "C" { pub fn b2Second(); }
            "#,
            Precision::Single,
        )
        .expect_err("every b2 extern block must carry the same module");
        assert!(
            partial.to_string().contains("is missing `#[link"),
            "unexpected error: {partial}"
        );

        let native_link_override = binding_fixture(
            r#"
                #[link(name = "forged", wasm_import_module = "box2d-sys-v0-single")]
                unsafe extern "C" { pub fn b2Run(); }
            "#,
            Precision::Single,
        )
        .expect_err("additional link metadata can alter the physical linkage");
        assert!(
            native_link_override
                .to_string()
                .contains("unsupported `#[link]` metadata"),
            "unexpected error: {native_link_override}"
        );
    }

    #[test]
    fn target_binding_surface_requires_wasm_module_on_every_extern_block() {
        let binding = binding_fixture("unsafe extern \"C\" { pub fn b2Run(); }", Precision::Double)
            .expect("minimal fixtures may defer mandatory module validation");
        let error = binding
            .surface
            .require_wasm_import_modules(Precision::Double, true)
            .expect_err("a formal target binding must declare its precision module");
        assert!(
            error.to_string().contains("box2d-sys-v0-double"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn binding_surface_rejects_unindexable_public_abi_extensions() {
        for binding in [
            "include!(\"other.rs\");",
            "mod nested { pub struct b2Hidden; }",
            "pub use nested::b2Hidden;",
            "generate_bindings!();",
            "unsafe extern \"C\" { generate_bindings!(); }",
            "pub fn b2Forged() {}",
            "#[unsafe(no_mangle)] pub extern \"C\" fn forged() {}",
            "pub struct b2Value { pub raw: u32 } impl b2Value { pub fn forged(&self) {} }",
            "#[repr(C)] pub struct b2Value { pub bytes: [u8; abi_len!()] }",
            "#[derive(Forged)] #[repr(C)] pub struct b2Value { pub raw: u32 }",
        ] {
            let error = binding_fixture(binding, Precision::Single)
                .expect_err("unindexable ABI extensions must fail closed");
            assert!(
                error
                    .to_string()
                    .contains("cannot be indexed as a closed-world public ABI surface"),
                "unexpected error for `{binding}`: {error}"
            );
        }
    }

    #[test]
    fn binding_surface_rejects_file_level_cfg() {
        let error = binding_fixture(
            "#![cfg(any())]\n#[repr(C)] pub struct b2Hidden { pub value: u32 }",
            Precision::Single,
        )
        .expect_err("file-level cfg can erase the entire indexed ABI surface");
        assert!(
            error.to_string().contains("uses unsupported `#[cfg]`"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn precision_mapping_rejects_extra_public_b2_aggregates() {
        let error = map_fixture(
            "typedef struct b2Pos { float x; } b2Pos;",
            r#"
                #[repr(C)] pub struct b2Pos { pub x: f32 }
                #[repr(C)] pub struct b2Forged { pub value: u32 }
            "#,
            CAbiPrecision::Single,
        )
        .expect_err("extra public aggregates must fail the reverse exact-set");
        assert!(
            error
                .to_string()
                .contains("public b2 aggregates do not exactly match"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn precision_mapping_rejects_extra_public_b2_callback_aliases() {
        let error = map_fixture(
            "typedef void b2Callback(float value);",
            r#"
                pub type b2Callback = Option<unsafe extern "C" fn(value: f32)>;
                pub type b2Forged = Option<unsafe extern "C" fn(value: f32)>;
            "#,
            CAbiPrecision::Single,
        )
        .expect_err("extra callback aliases must fail the reverse exact-set");
        assert!(
            error
                .to_string()
                .contains("public b2 callback aliases do not exactly match"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn precision_mapping_rejects_forged_opaque_aggregate_layout() {
        let error = map_fixture(
            r#"
                typedef struct b2Opaque b2Opaque;
                B2_API void b2UseOpaque(b2Opaque* value);
            "#,
            r#"
                #[repr(C)] pub struct b2Opaque { pub forged: u8 }
                unsafe extern "C" { pub fn b2UseOpaque(value: *mut b2Opaque); }
            "#,
            CAbiPrecision::Single,
        )
        .expect_err("opaque C tags must use the exact bindgen opaque representation");
        assert!(
            error
                .to_string()
                .contains("exact bindgen opaque repr-C struct"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn precision_mapping_accepts_exact_reachable_bindgen_union_wrapper() {
        map_fixture(
            "typedef struct b2Example { union { float x; unsigned int y; }; } b2Example;",
            r#"
                #[repr(C)]
                pub struct b2Example {
                    pub __bindgen_anon_1: b2Example__bindgen_ty_1,
                }
                #[repr(C)]
                pub union b2Example__bindgen_ty_1 {
                    pub x: f32,
                    pub y: u32,
                }
            "#,
            CAbiPrecision::Single,
        )
        .expect("reachable bindgen anonymous union wrappers belong to the exact surface");
    }

    #[test]
    fn precision_mapping_rejects_unproven_anonymous_struct_wrapper() {
        let error = map_fixture(
            "typedef struct b2Example { struct { float x; }; } b2Example;",
            r#"
                #[repr(C)]
                pub struct b2Example {
                    pub __bindgen_anon_1: b2Example__bindgen_ty_1,
                }
                #[repr(C)]
                pub struct b2Example__bindgen_ty_1 {
                    pub x: f32,
                }
            "#,
            CAbiPrecision::Single,
        )
        .expect_err("flattened C fields do not prove anonymous struct grouping");
        assert!(
            error
                .to_string()
                .contains("without an exact C anonymous-struct overlay proof"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn precision_mapping_rejects_distinct_overlays_merged_into_one_union_location() {
        let error = map_fixture(
            r#"
                typedef struct b2Example {
                    union { float first; unsigned int second; };
                    union { float third; unsigned int fourth; };
                } b2Example;
            "#,
            r#"
                #[repr(C)]
                pub struct b2Example {
                    pub __bindgen_anon_1: b2Example__bindgen_ty_1,
                }
                #[repr(C)]
                pub union b2Example__bindgen_ty_1 {
                    pub first: f32,
                    pub second: u32,
                    pub third: f32,
                    pub fourth: u32,
                }
            "#,
            CAbiPrecision::Single,
        )
        .expect_err("separate C storage overlays cannot share one Rust union location");
        assert!(
            error
                .to_string()
                .contains("merges distinct C overlay groups"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn precision_mapping_rejects_callback_parameter_drift() {
        let error = map_fixture(
            "typedef void b2Callback(double value);",
            "pub type b2Callback = Option<unsafe extern \"C\" fn(value: f32)>;",
            CAbiPrecision::Double,
        )
        .expect_err("callback parameter drift must fail closed");
        assert!(error.to_string().contains("callback ABI mismatch"));
    }

    #[test]
    fn vendored_single_precision_mapping_matches_generated_binding_shapes() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let include = root.join("boxdd-sys/box2d/include/box2d");
        let binding_path = root.join("boxdd-sys/src/bindings.rs");
        if !binding_path.exists() {
            return;
        }
        let inventory = parse_headers(&include).expect("vendored C inventory");
        let precision_inventory = parse_headers_for_precision(&include, CAbiPrecision::Single)
            .expect("single precision C inventory");
        let routes = BTreeMap::from([(
            ("single".to_owned(), "source".to_owned()),
            AbiBindingRoute {
                mode: "single".to_owned(),
                provider: "source".to_owned(),
                artifact: "bindings".to_owned(),
                rust_target: RustTarget::X86_64UnknownLinuxGnu,
                rust_features: Vec::new(),
            },
        )]);
        let bindings = AbiBindingIndexes::from([(
            "bindings".to_owned(),
            AbiBindingIndex::from_path(
                "bindings",
                Precision::Single,
                ArtifactTarget::Universal,
                ArtifactProvider::Source,
                &binding_path,
            )
            .expect("generated single binding index"),
        )]);
        let inventories =
            AbiPrecisionInventories::from([("single".to_owned(), precision_inventory)]);
        map_precision_inventory(&inventory, &inventories, &routes, &bindings)
            .expect("vendored single precision ABI mapping");
    }

    fn binding_fixture(binding: &str, precision: Precision) -> crate::Result<AbiBindingIndex> {
        let root = tempdir().expect("temporary binding fixture root");
        let binding_path = root.path().join("bindings.rs");
        fs::write(&binding_path, binding).expect("fixture binding");
        AbiBindingIndex::from_path(
            "fixture-bindings",
            precision,
            ArtifactTarget::Universal,
            ArtifactProvider::Source,
            &binding_path,
        )
    }

    fn map_fixture(
        header: &str,
        binding: &str,
        precision: CAbiPrecision,
    ) -> crate::Result<super::AbiContract> {
        let root = tempdir().expect("temporary fixture root");
        let include = root.path().join("include");
        fs::create_dir(&include).expect("fixture include directory");
        fs::write(include.join("fixture.h"), header).expect("fixture header");
        let binding_path = root.path().join("bindings.rs");
        fs::write(&binding_path, binding).expect("fixture binding");

        let mode = precision.as_str().to_owned();
        let inventory = parse_headers(&include)?;
        let precision_inventory = parse_headers_for_precision(&include, precision)?;
        let binding_precision = match precision {
            CAbiPrecision::Single => Precision::Single,
            CAbiPrecision::Double => Precision::Double,
        };
        let routes = BTreeMap::from([(
            (mode.clone(), "source".to_owned()),
            AbiBindingRoute {
                mode: mode.clone(),
                provider: "source".to_owned(),
                artifact: "fixture-bindings".to_owned(),
                rust_target: RustTarget::X86_64UnknownLinuxGnu,
                rust_features: Vec::new(),
            },
        )]);
        let bindings = AbiBindingIndexes::from([(
            "fixture-bindings".to_owned(),
            AbiBindingIndex::from_path(
                "fixture-bindings",
                binding_precision,
                ArtifactTarget::Universal,
                ArtifactProvider::Source,
                &binding_path,
            )?,
        )]);
        let precision_inventories = AbiPrecisionInventories::from([(mode, precision_inventory)]);
        map_precision_inventory(&inventory, &precision_inventories, &routes, &bindings)
    }
}
