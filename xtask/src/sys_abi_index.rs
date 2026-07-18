use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use syn::{Fields, ForeignItem, Item, PathArguments, Type, Visibility};

use crate::{Error, Result};

const FFI_PATH_PREFIX: &str = "boxdd_sys::ffi";

/// Exact Rust paths that are provably exported by a generated `boxdd-sys` binding file.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SysAbiIndex {
    type_paths: BTreeSet<String>,
    field_paths: BTreeSet<String>,
    function_paths: BTreeSet<String>,
    aggregate_fields: BTreeMap<String, BTreeMap<String, FieldType>>,
    aliases: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct FieldType {
    target_type: Option<String>,
    anonymous_wrapper: bool,
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
}

fn index_syntax(syntax: &syn::File) -> SysAbiIndex {
    let mut index = SysAbiIndex::default();

    for item in &syntax.items {
        match item {
            Item::Struct(item) if is_public(&item.vis) => {
                let type_path = qualified_path(&item.ident.to_string());
                index.type_paths.insert(type_path.clone());
                index_named_fields(&mut index, &type_path, &item.fields);
            }
            Item::Union(item) if is_public(&item.vis) => {
                let type_path = qualified_path(&item.ident.to_string());
                index.type_paths.insert(type_path.clone());
                index_fields(&mut index, &type_path, &item.fields.named);
            }
            Item::Type(item) if is_public(&item.vis) => {
                let type_path = qualified_path(&item.ident.to_string());
                index.type_paths.insert(type_path.clone());
                if let Some(target) = local_type_path(&item.ty) {
                    index.aliases.insert(type_path, target);
                }
            }
            Item::ForeignMod(item) if is_unsafe_c_foreign_module(item) => {
                for foreign_item in &item.items {
                    let ForeignItem::Fn(function) = foreign_item else {
                        continue;
                    };
                    if is_public(&function.vis) {
                        index
                            .function_paths
                            .insert(qualified_path(&function.sig.ident.to_string()));
                    }
                }
            }
            _ => {}
        }
    }

    index
}

fn index_named_fields(index: &mut SysAbiIndex, type_path: &str, fields: &Fields) {
    if let Fields::Named(fields) = fields {
        index_fields(index, type_path, &fields.named);
    }
}

fn index_fields(
    index: &mut SysAbiIndex,
    type_path: &str,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) {
    let mut field_types = BTreeMap::new();
    for field in fields {
        if !is_public(&field.vis) {
            continue;
        }
        let Some(ident) = &field.ident else {
            continue;
        };
        let field_name = ident.to_string();
        index
            .field_paths
            .insert(format!("{type_path}::{field_name}"));
        field_types.insert(
            field_name.clone(),
            FieldType {
                target_type: local_type_path(&field.ty),
                anonymous_wrapper: is_bindgen_anonymous_field(&field_name),
            },
        );
    }
    index
        .aggregate_fields
        .insert(type_path.to_owned(), field_types);
}

fn local_type_path(rust_type: &Type) -> Option<String> {
    let Type::Path(rust_type) = rust_type else {
        return None;
    };
    if rust_type.qself.is_some() || rust_type.path.leading_colon.is_some() {
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

    fn index(source: &str) -> SysAbiIndex {
        let syntax = syn::parse_file(source).expect("test bindings must parse");
        index_syntax(&syntax)
    }

    #[test]
    fn indexes_the_checked_in_pregenerated_bindings() {
        let bindings =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../boxdd-sys/src/bindings_pregenerated.rs");
        let index = index_bindings(&bindings).expect("pregenerated bindings must be indexable");

        assert!(index.contains_type_path("boxdd_sys::ffi::b2Vec2"));
        assert!(index.contains_field_path("boxdd_sys::ffi::b2Vec2::x"));
        assert!(index.contains_type_path("boxdd_sys::ffi::b2TaskCallback"));
        assert!(index.contains_function_path("boxdd_sys::ffi::b2SetAllocator"));
        assert!(index.contains_function_path("boxdd_sys::ffi::b2World_Step"));
        assert!(!index.contains_field_path("boxdd_sys::ffi::b2Vec2::missing"));
        assert!(!index.contains_type_path("boxdd_sys::ffi::MissingCallback"));
        assert!(!index.contains_function_path("boxdd_sys::ffi::b2MissingFunction"));
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
}
