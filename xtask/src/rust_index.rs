use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    path::{Path, PathBuf},
};

use quote::ToTokens;
use syn::{
    Attribute, Expr, ExprCall, ExprMethodCall, File, FnArg, ImplItem, Item, ItemImpl, ItemMod,
    ItemUse, Meta, Pat, Signature, Type, UseTree, Visibility, parse::Parser, visit::Visit,
};

use crate::{Error, Result};

#[derive(Clone, Debug, Default)]
pub struct RustIndex {
    public_paths: BTreeSet<String>,
    public_type_paths: BTreeSet<String>,
    public_callable_paths: BTreeSet<String>,
    public_safe_callable_paths: BTreeSet<String>,
    public_field_paths: BTreeSet<String>,
    public_alias_paths: BTreeMap<String, String>,
    callable_return_types: BTreeMap<String, String>,
    symbol_paths: BTreeMap<String, BTreeSet<String>>,
    callable_argument_symbol_paths: BTreeMap<(String, usize), BTreeSet<String>>,
    callable_field_argument_symbol_paths: BTreeMap<(String, usize, String), BTreeSet<String>>,
    ffi_type_paths: BTreeMap<String, BTreeSet<String>>,
    ffi_field_paths: BTreeMap<(String, String), BTreeSet<String>>,
    safe_ffi_type_paths: BTreeMap<String, BTreeSet<String>>,
    safe_ffi_field_paths: BTreeMap<(String, String), BTreeSet<String>>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RustIndexCoordinate {
    enabled_flags: BTreeSet<String>,
    known_flags: BTreeSet<String>,
    cfg_values: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestEvidenceIndex {
    pub fingerprint: String,
    pub called_public_paths: BTreeSet<String>,
    pub called_local_paths: BTreeSet<String>,
    pub dropped_public_types: BTreeSet<String>,
    pub implementation_reachable_symbols: BTreeSet<String>,
    pub unresolved_calls: BTreeSet<TestEvidenceGap>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct TestEvidenceGap {
    pub expression: String,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DiscoveredTestItem {
    pub file: String,
    pub item: String,
    pub package: String,
    pub gate: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveredTestEvidence {
    pub test: DiscoveredTestItem,
    pub index: TestEvidenceIndex,
}

impl RustIndexCoordinate {
    pub fn source_single() -> Self {
        Self::source_for_target(
            std::env::consts::ARCH,
            std::env::consts::OS,
            if cfg!(unix) {
                ["unix"].as_slice()
            } else if cfg!(windows) {
                ["windows"].as_slice()
            } else {
                [].as_slice()
            },
            if cfg!(target_endian = "little") {
                "little"
            } else {
                "big"
            },
            if cfg!(target_pointer_width = "64") {
                "64"
            } else if cfg!(target_pointer_width = "32") {
                "32"
            } else {
                "16"
            },
            if cfg!(panic = "unwind") {
                "unwind"
            } else {
                "abort"
            },
        )
    }

    pub fn source_for_target<I, S>(
        target_arch: &str,
        target_os: &str,
        target_families: I,
        target_endian: &str,
        target_pointer_width: &str,
        panic: &str,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let families = target_families
            .into_iter()
            .map(|family| family.as_ref().to_owned())
            .collect::<BTreeSet<_>>();
        let mut coordinate = Self {
            enabled_flags: BTreeSet::new(),
            known_flags: ["docsrs", "doctest", "miri", "test", "unix", "windows"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            cfg_values: BTreeMap::new(),
        };
        if families.contains("unix") {
            coordinate.enabled_flags.insert("unix".into());
        }
        if families.contains("windows") {
            coordinate.enabled_flags.insert("windows".into());
        }
        coordinate.set_cfg_values("feature", std::iter::empty::<&str>());
        coordinate.set_cfg_values("target_arch", [target_arch]);
        coordinate.set_cfg_values("target_os", [target_os]);
        coordinate.set_cfg_values("target_family", families);
        coordinate.set_cfg_values("target_endian", [target_endian]);
        coordinate.set_cfg_values("target_pointer_width", [target_pointer_width]);
        coordinate.set_cfg_values("panic", [panic]);
        coordinate
    }

    pub fn wasm32_unknown_unknown() -> Self {
        Self::source_for_target("wasm32", "unknown", ["wasm"], "little", "32", "abort")
            .with_cfg_values("target_abi", [""])
            .with_cfg_values("target_env", [""])
            .with_cfg_values("target_vendor", ["unknown"])
            .with_cfg_values(
                "target_feature",
                [
                    "bulk-memory",
                    "multivalue",
                    "mutable-globals",
                    "nontrapping-fptoint",
                    "reference-types",
                    "sign-ext",
                ],
            )
            .with_cfg_values("target_has_atomic", ["8", "16", "32", "64", "ptr"])
            .with_cfg_values(
                "target_has_atomic_primitive_alignment",
                ["8", "16", "32", "64", "ptr"],
            )
    }

    pub fn wasm32_wasip1() -> Self {
        Self::source_for_target("wasm32", "wasi", ["wasm"], "little", "32", "abort")
            .with_cfg_values("target_abi", [""])
            .with_cfg_values("target_env", ["p1"])
            .with_cfg_values("target_vendor", ["unknown"])
            .with_cfg_values(
                "target_feature",
                [
                    "bulk-memory",
                    "crt-static",
                    "multivalue",
                    "mutable-globals",
                    "nontrapping-fptoint",
                    "reference-types",
                    "sign-ext",
                ],
            )
            .with_cfg_values("target_has_atomic", ["8", "16", "32", "64", "ptr"])
            .with_cfg_values(
                "target_has_atomic_primitive_alignment",
                ["8", "16", "32", "64", "ptr"],
            )
    }

    pub fn with_cfg_flag(mut self, name: impl Into<String>, enabled: bool) -> Self {
        let name = name.into();
        self.known_flags.insert(name.clone());
        if enabled {
            self.enabled_flags.insert(name);
        } else {
            self.enabled_flags.remove(&name);
        }
        self
    }

    pub fn with_cfg_value(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.cfg_values
            .entry(name.into())
            .or_default()
            .insert(value.into());
        self
    }

    pub fn with_cfg_values<I, S>(mut self, name: impl Into<String>, values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let name = name.into();
        self.set_cfg_values(&name, values);
        self
    }

    fn set_cfg_values<I, S>(&mut self, name: &str, values: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.cfg_values.insert(
            name.to_owned(),
            values
                .into_iter()
                .map(|value| value.as_ref().to_owned())
                .collect(),
        );
    }

    fn test() -> Self {
        Self::source_single().with_cfg_flag("test", true)
    }
}

impl Default for RustIndexCoordinate {
    fn default() -> Self {
        Self::source_single()
    }
}

#[derive(Clone, Debug)]
struct Node {
    module: String,
    owner: Option<TypeRef>,
    is_drop: bool,
    ident: String,
    public_path: Option<String>,
    calls: BTreeSet<CallRef>,
    abi_types: BTreeSet<TypeRef>,
    abi_fields: BTreeSet<AbiFieldRef>,
    return_type: Option<TypeRef>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct AbiFieldRef {
    raw_type: TypeRef,
    raw_field_chain: String,
    safe_owner: Option<TypeRef>,
    safe_field: Option<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct RawStorageRef {
    raw_type: TypeRef,
    member: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct CallRef {
    target: CallTarget,
    arguments: Vec<Option<CallTarget>>,
    callable_field_arguments: BTreeMap<usize, BTreeMap<String, CallTarget>>,
    parameter_offset: usize,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum CallTarget {
    Path {
        segments: Vec<String>,
        qself: Option<Vec<String>>,
    },
    Method {
        ident: String,
        receiver: MethodReceiver,
    },
    Parameter(usize),
    Closure(Box<ClosureSummary>),
    OptionSome {
        wrapper: Vec<String>,
        value: Box<CallTarget>,
    },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ClosureSummary {
    calls: BTreeSet<CallRef>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum MethodReceiver {
    CurrentOwner,
    ExplicitType(Vec<String>),
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct TypeRef {
    module: String,
    segments: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum PublicAliasResolution {
    Missing,
    Unique(String),
    Invalid,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ProvenTrait {
    Public,
    Drop,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CfgDecision {
    Enabled,
    Disabled,
    Unknown,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ResolvedTarget {
    Node(usize),
    CSymbol(String),
    Closure {
        definition_node: usize,
        summary: Box<ClosureSummary>,
    },
}

#[derive(Default)]
struct IndexBuilder {
    crate_src: PathBuf,
    crate_name: String,
    coordinate: RustIndexCoordinate,
    has_boxdd_sys_dependency: bool,
    nodes: Vec<Node>,
    public_paths: BTreeSet<String>,
    public_field_paths: BTreeSet<String>,
    public_callable_paths: BTreeSet<String>,
    public_alias_targets: BTreeMap<String, BTreeSet<String>>,
    public_alias_entries: BTreeSet<String>,
    public_declarations: BTreeSet<String>,
    public_type_declarations: BTreeSet<String>,
    public_safe_callable_paths: BTreeSet<String>,
    declared_traits: BTreeSet<String>,
    public_fields_by_owner: BTreeMap<String, BTreeSet<String>>,
    raw_storage_by_owner: BTreeMap<String, BTreeSet<RawStorageRef>>,
    canonical_public_paths: BTreeMap<String, String>,
    public_alias_paths: BTreeMap<String, String>,
    declared_types: BTreeMap<String, BTreeSet<String>>,
    declared_type_paths: BTreeSet<String>,
    imports: BTreeMap<String, BTreeMap<String, BTreeSet<String>>>,
    import_bindings: BTreeMap<String, BTreeMap<String, BTreeSet<Vec<String>>>>,
    glob_imports: BTreeMap<String, BTreeSet<String>>,
    local_modules: BTreeMap<String, BTreeSet<String>>,
    self_extern_aliases: BTreeMap<String, BTreeSet<String>>,
    raii_contains: BTreeMap<String, BTreeSet<TypeRef>>,
    visited_files: BTreeSet<PathBuf>,
}

pub fn index_boxdd(root: &Path) -> Result<RustIndex> {
    index_crate(&root.join("boxdd/src/lib.rs"), "boxdd")
}

pub fn index_boxdd_for_coordinate(
    root: &Path,
    coordinate: &RustIndexCoordinate,
) -> Result<RustIndex> {
    index_crate_for_coordinate(&root.join("boxdd/src/lib.rs"), "boxdd", coordinate)
}

pub fn index_boxdd_routes(
    root: &Path,
    coordinates: &BTreeMap<(String, String), RustIndexCoordinate>,
) -> Result<BTreeMap<(String, String), RustIndex>> {
    let mut indexes_by_coordinate: BTreeMap<RustIndexCoordinate, RustIndex> = BTreeMap::new();
    let mut routes = BTreeMap::new();
    for (route, coordinate) in coordinates {
        let index = if let Some(index) = indexes_by_coordinate.get(coordinate) {
            index.clone()
        } else {
            let index = index_boxdd_for_coordinate(root, coordinate)?;
            indexes_by_coordinate.insert(coordinate.clone(), index.clone());
            index
        };
        routes.insert(route.clone(), index);
    }
    Ok(routes)
}

pub fn index_crate(lib_rs: &Path, crate_name: &str) -> Result<RustIndex> {
    index_crate_for_coordinate(lib_rs, crate_name, &RustIndexCoordinate::source_single())
}

pub fn index_crate_for_coordinate(
    lib_rs: &Path,
    crate_name: &str,
    coordinate: &RustIndexCoordinate,
) -> Result<RustIndex> {
    let source = fs::read_to_string(lib_rs).map_err(|source| Error::io(lib_rs, source))?;
    let syntax = syn::parse_file(&source).map_err(|error| {
        Error::message(format!(
            "{}: invalid Rust syntax: {error}",
            lib_rs.display()
        ))
    })?;
    let mut builder = IndexBuilder {
        crate_src: lib_rs
            .parent()
            .ok_or_else(|| Error::message(format!("{} has no parent", lib_rs.display())))?
            .to_owned(),
        crate_name: crate_name.to_owned(),
        coordinate: coordinate.clone(),
        has_boxdd_sys_dependency: crate_declares_boxdd_sys_dependency(lib_rs)?,
        ..IndexBuilder::default()
    };
    builder.collect_public_surface_file(lib_rs, &syntax, &[], true, &mut BTreeSet::new())?;
    builder.resolve_public_surface(crate_name);
    builder.collect_file(lib_rs, &syntax, crate_name, &[], true)?;
    Ok(builder.finish())
}

fn crate_declares_boxdd_sys_dependency(lib_rs: &Path) -> Result<bool> {
    let Some(manifest_path) = lib_rs.parent().and_then(|parent| {
        parent
            .ancestors()
            .map(|directory| directory.join("Cargo.toml"))
            .find(|path| path.is_file())
    }) else {
        return Ok(false);
    };
    let manifest = read_cargo_manifest(&manifest_path)?;
    let Some(dependencies) = manifest.get("dependencies").and_then(toml::Value::as_table) else {
        return Ok(false);
    };
    dependency_table_declares_boxdd_sys(dependencies, &manifest_path, &manifest)
}

fn read_cargo_manifest(path: &Path) -> Result<toml::Value> {
    let source = fs::read_to_string(path).map_err(|source| Error::io(path, source))?;
    toml::from_str(&source).map_err(|error| {
        Error::message(format!(
            "{}: invalid Cargo manifest: {error}",
            path.display()
        ))
    })
}

fn dependency_table_declares_boxdd_sys(
    dependencies: &toml::value::Table,
    manifest_path: &Path,
    manifest: &toml::Value,
) -> Result<bool> {
    let matches = dependencies
        .iter()
        .filter(|(name, _)| name.replace('-', "_") == "boxdd_sys")
        .collect::<Vec<_>>();
    let [(name, specification)] = matches.as_slice() else {
        return Ok(false);
    };

    if specification
        .as_table()
        .and_then(|table| table.get("workspace"))
        .and_then(toml::Value::as_bool)
        == Some(true)
    {
        if specification
            .as_table()
            .is_some_and(|table| table.contains_key("package"))
        {
            return Ok(false);
        }
        let Some((workspace_path, workspace)) =
            resolve_workspace_manifest(manifest_path, manifest)?
        else {
            return Ok(false);
        };
        let Some(workspace_specification) = workspace
            .get("workspace")
            .and_then(|workspace| workspace.get("dependencies"))
            .and_then(toml::Value::as_table)
            .and_then(|dependencies| dependencies.get(*name))
        else {
            return Ok(false);
        };
        return dependency_specification_names_boxdd_sys(
            name,
            workspace_specification,
            &workspace_path,
        );
    }

    dependency_specification_names_boxdd_sys(name, specification, manifest_path)
}

fn dependency_specification_names_boxdd_sys(
    dependency_name: &str,
    specification: &toml::Value,
    manifest_path: &Path,
) -> Result<bool> {
    let table = specification.as_table();
    if table
        .and_then(|table| table.get("workspace"))
        .and_then(toml::Value::as_bool)
        == Some(true)
    {
        return Ok(false);
    }
    let package_name = table
        .and_then(|table| table.get("package"))
        .and_then(toml::Value::as_str)
        .unwrap_or(dependency_name);
    if package_name != "boxdd-sys" {
        return Ok(false);
    }

    let Some(relative_path) = table
        .and_then(|table| table.get("path"))
        .and_then(toml::Value::as_str)
    else {
        return Ok(true);
    };
    let Some(manifest_directory) = manifest_path.parent() else {
        return Ok(false);
    };
    let dependency_manifest_path = manifest_directory.join(relative_path).join("Cargo.toml");
    if !dependency_manifest_path.is_file() {
        return Ok(false);
    }
    let dependency_manifest = read_cargo_manifest(&dependency_manifest_path)?;
    Ok(dependency_manifest
        .get("package")
        .and_then(|package| package.get("name"))
        .and_then(toml::Value::as_str)
        == Some("boxdd-sys"))
}

fn resolve_workspace_manifest(
    member_manifest_path: &Path,
    member_manifest: &toml::Value,
) -> Result<Option<(PathBuf, toml::Value)>> {
    if member_manifest.get("workspace").is_some() {
        return Ok(Some((
            member_manifest_path.to_owned(),
            member_manifest.clone(),
        )));
    }

    if let Some(relative_workspace) = member_manifest
        .get("package")
        .and_then(|package| package.get("workspace"))
        .and_then(toml::Value::as_str)
    {
        let Some(member_directory) = member_manifest_path.parent() else {
            return Ok(None);
        };
        let workspace_path = member_directory.join(relative_workspace).join("Cargo.toml");
        if !workspace_path.is_file() {
            return Ok(None);
        }
        let workspace = read_cargo_manifest(&workspace_path)?;
        return Ok(workspace
            .get("workspace")
            .is_some()
            .then_some((workspace_path, workspace)));
    }

    let Some(member_directory) = member_manifest_path.parent() else {
        return Ok(None);
    };
    for ancestor in member_directory.ancestors().skip(1) {
        let workspace_path = ancestor.join("Cargo.toml");
        if !workspace_path.is_file() {
            continue;
        }
        let workspace = read_cargo_manifest(&workspace_path)?;
        if workspace.get("workspace").is_some() {
            return Ok(Some((workspace_path, workspace)));
        }
    }
    Ok(None)
}

impl RustIndex {
    pub fn contains_public_path(&self, path: &str) -> bool {
        self.public_paths.contains(path)
    }

    pub fn contains_public_type_path(&self, path: &str) -> bool {
        self.public_type_paths.contains(path)
    }

    pub fn contains_public_callable_path(&self, path: &str) -> bool {
        self.public_callable_paths.contains(path)
    }

    pub fn contains_public_safe_callable_path(&self, path: &str) -> bool {
        self.public_safe_callable_paths.contains(path)
    }

    pub fn paths_for_symbol(&self, symbol: &str) -> impl Iterator<Item = &str> {
        self.symbol_paths
            .get(symbol)
            .into_iter()
            .flatten()
            .map(String::as_str)
    }

    pub fn path_reaches_symbol(&self, path: &str, symbol: &str) -> bool {
        self.symbol_paths
            .get(symbol)
            .is_some_and(|paths| paths.contains(path))
    }

    pub fn path_reaches_symbol_with_callable_argument(
        &self,
        path: &str,
        symbol: &str,
        argument_index: usize,
    ) -> bool {
        self.callable_argument_symbol_paths
            .get(&(symbol.to_owned(), argument_index))
            .is_some_and(|paths| paths.contains(path))
    }

    pub fn path_reaches_symbol_with_callable_field_argument(
        &self,
        path: &str,
        symbol: &str,
        argument_index: usize,
        raw_field: &str,
    ) -> bool {
        self.callable_field_argument_symbol_paths
            .get(&(symbol.to_owned(), argument_index, raw_field.to_owned()))
            .is_some_and(|paths| paths.contains(path))
    }

    pub fn contains_public_field_path(&self, path: &str) -> bool {
        self.public_field_paths.contains(path)
    }

    pub fn path_has_ffi_type_witness(&self, public_path: &str, raw_type_path: &str) -> bool {
        self.ffi_type_paths
            .get(raw_type_path)
            .is_some_and(|paths| paths.contains(public_path))
    }

    pub fn paths_with_ffi_type_witness(&self, raw_type_path: &str) -> impl Iterator<Item = &str> {
        self.ffi_type_paths
            .get(raw_type_path)
            .into_iter()
            .flatten()
            .map(String::as_str)
    }

    pub fn path_has_safe_ffi_type_witness(&self, public_path: &str, raw_type_path: &str) -> bool {
        self.safe_ffi_type_paths
            .get(raw_type_path)
            .is_some_and(|paths| paths.contains(public_path))
    }

    pub fn paths_with_safe_ffi_type_witness(
        &self,
        raw_type_path: &str,
    ) -> impl Iterator<Item = &str> {
        self.safe_ffi_type_paths
            .get(raw_type_path)
            .into_iter()
            .flatten()
            .map(String::as_str)
    }

    pub fn path_has_ffi_field_witness(
        &self,
        public_path: &str,
        raw_type_path: &str,
        raw_field_chain: &str,
    ) -> bool {
        self.ffi_field_paths
            .get(&(raw_type_path.to_owned(), raw_field_chain.to_owned()))
            .is_some_and(|paths| paths.contains(public_path))
    }

    pub fn paths_with_ffi_field_witness(
        &self,
        raw_type_path: &str,
        raw_field_chain: &str,
    ) -> impl Iterator<Item = &str> {
        self.ffi_field_paths
            .get(&(raw_type_path.to_owned(), raw_field_chain.to_owned()))
            .into_iter()
            .flatten()
            .map(String::as_str)
    }

    pub fn path_has_safe_ffi_field_witness(
        &self,
        public_path: &str,
        raw_type_path: &str,
        raw_field_chain: &str,
    ) -> bool {
        self.safe_ffi_field_paths
            .get(&(raw_type_path.to_owned(), raw_field_chain.to_owned()))
            .is_some_and(|paths| paths.contains(public_path))
    }

    pub fn paths_with_safe_ffi_field_witness(
        &self,
        raw_type_path: &str,
        raw_field_chain: &str,
    ) -> impl Iterator<Item = &str> {
        self.safe_ffi_field_paths
            .get(&(raw_type_path.to_owned(), raw_field_chain.to_owned()))
            .into_iter()
            .flatten()
            .map(String::as_str)
    }

    pub fn public_path_count(&self) -> usize {
        self.public_paths.len()
    }
}

impl IndexBuilder {
    fn collect_public_surface_file(
        &mut self,
        file: &Path,
        syntax: &File,
        modules: &[String],
        modules_public: bool,
        visited: &mut BTreeSet<PathBuf>,
    ) -> Result<()> {
        if !attributes_are_enabled(&syntax.attrs, &self.coordinate) {
            return Ok(());
        }
        let canonical = fs::canonicalize(file).map_err(|source| Error::io(file, source))?;
        if !visited.insert(canonical) {
            return Ok(());
        }
        self.collect_public_surface_items(&syntax.items, modules, modules_public, visited)
    }

    fn collect_public_surface_items(
        &mut self,
        items: &[Item],
        modules: &[String],
        modules_public: bool,
        visited: &mut BTreeSet<PathBuf>,
    ) -> Result<()> {
        for item in items {
            if !item_is_enabled(item, &self.coordinate) {
                continue;
            }
            match item {
                Item::Mod(module) => {
                    self.local_modules
                        .entry(module_key(modules))
                        .or_default()
                        .insert(module.ident.to_string());
                    let mut child_modules = modules.to_vec();
                    child_modules.push(module.ident.to_string());
                    let child_public = modules_public && is_public(&module.vis);
                    if let Some((_, items)) = &module.content {
                        self.collect_public_surface_items(
                            items,
                            &child_modules,
                            child_public,
                            visited,
                        )?;
                    } else {
                        let child_file = resolve_module_file(
                            &self.crate_src,
                            modules,
                            &module.ident.to_string(),
                        )?;
                        let source = fs::read_to_string(&child_file)
                            .map_err(|source| Error::io(&child_file, source))?;
                        let syntax = syn::parse_file(&source).map_err(|error| {
                            Error::message(format!(
                                "{}: invalid Rust syntax: {error}",
                                child_file.display()
                            ))
                        })?;
                        self.collect_public_surface_file(
                            &child_file,
                            &syntax,
                            &child_modules,
                            child_public,
                            visited,
                        )?;
                    }
                }
                Item::Use(item) => {
                    self.collect_imports(modules, item);
                    if !is_public(&item.vis) {
                        continue;
                    }
                    let module = module_key(modules);
                    let mut entries = Vec::new();
                    flatten_use_tree(&item.tree, &mut Vec::new(), &mut entries);
                    for (alias, raw_path) in entries {
                        let alias = item_key(modules, &alias);
                        self.public_alias_targets
                            .entry(alias.clone())
                            .or_default()
                            .extend(normalize_import_paths(&module, &raw_path));
                        if modules_public {
                            self.public_alias_entries.insert(alias);
                        }
                    }
                }
                Item::Fn(item) if is_public(&item.vis) => {
                    self.record_public_declaration(
                        modules,
                        modules_public,
                        &item.sig.ident.to_string(),
                    );
                }
                Item::Struct(item) => {
                    self.collect_type(modules, &item.ident.to_string());
                    self.record_raw_storage(modules, &item.ident.to_string(), item.fields.iter());
                    if is_public(&item.vis) {
                        self.record_public_fields(
                            modules,
                            &item.ident.to_string(),
                            item.fields.iter(),
                        );
                        self.record_public_type_declaration(
                            modules,
                            modules_public,
                            &item.ident.to_string(),
                        );
                    }
                }
                Item::Enum(item) => {
                    self.collect_type(modules, &item.ident.to_string());
                    if is_public(&item.vis) {
                        self.record_public_type_declaration(
                            modules,
                            modules_public,
                            &item.ident.to_string(),
                        );
                    }
                }
                Item::Type(item) => {
                    self.collect_type(modules, &item.ident.to_string());
                    if is_public(&item.vis) {
                        self.record_public_type_declaration(
                            modules,
                            modules_public,
                            &item.ident.to_string(),
                        );
                    }
                }
                Item::Union(item) => {
                    self.collect_type(modules, &item.ident.to_string());
                    self.record_raw_storage(
                        modules,
                        &item.ident.to_string(),
                        item.fields.named.iter(),
                    );
                    if is_public(&item.vis) {
                        self.record_public_fields(
                            modules,
                            &item.ident.to_string(),
                            item.fields.named.iter(),
                        );
                        self.record_public_type_declaration(
                            modules,
                            modules_public,
                            &item.ident.to_string(),
                        );
                    }
                }
                Item::Trait(item) => {
                    let name = item.ident.to_string();
                    self.collect_type(modules, &name);
                    self.declared_traits.insert(item_key(modules, &name));
                    if is_public(&item.vis) {
                        self.record_public_declaration(modules, modules_public, &name);
                    }
                }
                Item::Const(item) if is_public(&item.vis) => {
                    self.record_public_declaration(
                        modules,
                        modules_public,
                        &item.ident.to_string(),
                    );
                }
                Item::Static(item) if is_public(&item.vis) => {
                    self.record_public_declaration(
                        modules,
                        modules_public,
                        &item.ident.to_string(),
                    );
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn record_public_declaration(&mut self, modules: &[String], modules_public: bool, name: &str) {
        let declaration = item_key(modules, name);
        self.public_declarations.insert(declaration.clone());
        if modules_public {
            self.public_alias_entries.insert(declaration);
        }
    }

    fn record_public_type_declaration(
        &mut self,
        modules: &[String],
        modules_public: bool,
        name: &str,
    ) {
        self.public_type_declarations
            .insert(item_key(modules, name));
        self.record_public_declaration(modules, modules_public, name);
    }

    fn record_public_fields<'a>(
        &mut self,
        modules: &[String],
        owner: &str,
        fields: impl IntoIterator<Item = &'a syn::Field>,
    ) {
        let owner = item_key(modules, owner);
        for field in fields {
            let Some(name) = &field.ident else {
                continue;
            };
            if is_public(&field.vis) && attributes_are_enabled(&field.attrs, &self.coordinate) {
                self.public_fields_by_owner
                    .entry(owner.clone())
                    .or_default()
                    .insert(name.to_string());
            }
        }
    }

    fn record_raw_storage<'a>(
        &mut self,
        modules: &[String],
        owner: &str,
        fields: impl IntoIterator<Item = &'a syn::Field>,
    ) {
        let module = module_key(modules);
        let owner = item_key(modules, owner);
        for (index, field) in fields.into_iter().enumerate() {
            if !attributes_are_enabled(&field.attrs, &self.coordinate) {
                continue;
            }
            let Some(segments) = direct_value_type_path(&field.ty) else {
                continue;
            };
            self.raw_storage_by_owner
                .entry(owner.clone())
                .or_default()
                .insert(RawStorageRef {
                    raw_type: TypeRef {
                        module: module.clone(),
                        segments,
                    },
                    member: field
                        .ident
                        .as_ref()
                        .map_or_else(|| index.to_string(), ToString::to_string),
                });
        }
    }

    fn resolve_public_surface(&mut self, crate_name: &str) {
        let entries = self.public_alias_entries.clone();
        let mut aliases_by_declaration: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut resolved_aliases = Vec::new();
        for alias in entries {
            let PublicAliasResolution::Unique(declaration) =
                self.resolve_public_alias(&alias, &mut BTreeSet::new())
            else {
                continue;
            };
            let public_path = format!("{crate_name}::{alias}");
            self.public_paths.insert(public_path.clone());
            resolved_aliases.push((public_path.clone(), declaration.clone()));
            aliases_by_declaration
                .entry(declaration)
                .or_default()
                .insert(public_path);
        }
        self.canonical_public_paths = aliases_by_declaration
            .into_iter()
            .filter_map(|(declaration, aliases)| {
                aliases
                    .into_iter()
                    .min_by_key(|alias| (alias.matches("::").count(), alias.clone()))
                    .map(|alias| (declaration, alias))
            })
            .collect();
        self.public_alias_paths = resolved_aliases
            .into_iter()
            .filter_map(|(alias, declaration)| {
                self.canonical_public_paths
                    .get(&declaration)
                    .cloned()
                    .map(|canonical| (alias, canonical))
            })
            .collect();
        self.public_field_paths = self
            .public_fields_by_owner
            .iter()
            .filter_map(|(owner, fields)| {
                self.canonical_public_paths
                    .get(owner)
                    .map(|owner| (owner, fields))
            })
            .flat_map(|(owner, fields)| fields.iter().map(move |field| format!("{owner}::{field}")))
            .collect();
        self.public_paths
            .extend(self.public_field_paths.iter().cloned());
    }

    fn resolve_public_alias(
        &self,
        alias: &str,
        visiting: &mut BTreeSet<String>,
    ) -> PublicAliasResolution {
        if self.public_declarations.contains(alias) {
            return PublicAliasResolution::Unique(alias.to_owned());
        }
        let Some(targets) = self.public_alias_targets.get(alias) else {
            return PublicAliasResolution::Missing;
        };
        if !visiting.insert(alias.to_owned()) {
            return PublicAliasResolution::Invalid;
        }
        let mut declarations = BTreeSet::new();
        for target in targets {
            match self.resolve_public_alias(target, visiting) {
                PublicAliasResolution::Missing => {}
                PublicAliasResolution::Unique(declaration) => {
                    declarations.insert(declaration);
                }
                PublicAliasResolution::Invalid => {
                    visiting.remove(alias);
                    return PublicAliasResolution::Invalid;
                }
            }
        }
        visiting.remove(alias);
        match declarations.len() {
            0 => PublicAliasResolution::Missing,
            1 => PublicAliasResolution::Unique(
                declarations.into_iter().next().expect("one declaration"),
            ),
            _ => PublicAliasResolution::Invalid,
        }
    }

    fn public_type_path(&self, _crate_name: &str, item: &str) -> Option<String> {
        self.canonical_public_paths.get(item).cloned()
    }

    fn collect_file(
        &mut self,
        file: &Path,
        syntax: &File,
        crate_name: &str,
        modules: &[String],
        modules_public: bool,
    ) -> Result<()> {
        if !attributes_are_enabled(&syntax.attrs, &self.coordinate) {
            return Ok(());
        }
        let canonical = fs::canonicalize(file).map_err(|source| Error::io(file, source))?;
        if !self.visited_files.insert(canonical) {
            return Ok(());
        }
        self.collect_items(file, &syntax.items, crate_name, modules, modules_public)
    }

    fn collect_items(
        &mut self,
        file: &Path,
        items: &[Item],
        crate_name: &str,
        modules: &[String],
        modules_public: bool,
    ) -> Result<()> {
        for item in items {
            if !item_is_enabled(item, &self.coordinate) {
                continue;
            }
            match item {
                Item::Mod(module) => {
                    self.local_modules
                        .entry(module_key(modules))
                        .or_default()
                        .insert(module.ident.to_string());
                    self.collect_module(file, module, crate_name, modules, modules_public)?
                }
                Item::Fn(function) => {
                    let name = function.sig.ident.to_string();
                    let public_path = is_public(&function.vis)
                        .then(|| {
                            self.canonical_public_paths
                                .get(&item_key(modules, &name))
                                .cloned()
                        })
                        .flatten();
                    if let Some(path) = &public_path {
                        self.public_paths.insert(path.clone());
                        self.public_callable_paths.insert(path.clone());
                        if function.sig.unsafety.is_none() {
                            self.public_safe_callable_paths.insert(path.clone());
                        }
                    }
                    self.nodes.push(node_from_body(
                        NodeIdentity {
                            module: module_key(modules),
                            owner: None,
                            is_drop: false,
                            ident: name,
                            public_path,
                        },
                        &function.sig,
                        &function.block,
                        &[],
                    ));
                }
                Item::Impl(item_impl) => self.collect_impl(item_impl, crate_name, modules),
                Item::Struct(item) => {
                    self.collect_type(modules, &item.ident.to_string());
                    self.collect_raii_fields(crate_name, modules, item);
                }
                Item::Enum(item) => self.collect_type(modules, &item.ident.to_string()),
                Item::Type(item) => self.collect_type(modules, &item.ident.to_string()),
                Item::Trait(item) => {
                    let name = item.ident.to_string();
                    self.collect_type(modules, &name);
                    let owner = self.public_type_path(crate_name, &item_key(modules, &name));
                    let trait_public = owner.is_some();
                    let owner = owner.unwrap_or_else(|| path_with(crate_name, modules, &name));
                    for trait_item in &item.items {
                        if let syn::TraitItem::Fn(method) = trait_item {
                            if !attributes_are_enabled(&method.attrs, &self.coordinate) {
                                continue;
                            }
                            let public_path = trait_public.then(|| {
                                let path = format!("{owner}::{}", method.sig.ident);
                                self.public_paths.insert(path.clone());
                                self.public_callable_paths.insert(path.clone());
                                if method.sig.unsafety.is_none() {
                                    self.public_safe_callable_paths.insert(path.clone());
                                }
                                path
                            });
                            if let Some(block) = &method.default {
                                self.nodes.push(node_from_body(
                                    NodeIdentity {
                                        module: module_key(modules),
                                        owner: Some(TypeRef {
                                            module: module_key(modules),
                                            segments: vec![name.clone()],
                                        }),
                                        is_drop: false,
                                        ident: method.sig.ident.to_string(),
                                        public_path,
                                    },
                                    &method.sig,
                                    block,
                                    &[],
                                ));
                            }
                        }
                    }
                }
                Item::Const(item) if modules_public && is_public(&item.vis) => {
                    self.public_paths.insert(path_with(
                        crate_name,
                        modules,
                        &item.ident.to_string(),
                    ));
                }
                Item::Static(item) if modules_public && is_public(&item.vis) => {
                    self.public_paths.insert(path_with(
                        crate_name,
                        modules,
                        &item.ident.to_string(),
                    ));
                }
                Item::Use(item) => {
                    self.collect_imports(modules, item);
                }
                Item::ExternCrate(item) => self.collect_extern_crate(modules, item),
                _ => {}
            }
        }
        Ok(())
    }

    fn collect_module(
        &mut self,
        file: &Path,
        module: &ItemMod,
        crate_name: &str,
        modules: &[String],
        parent_public: bool,
    ) -> Result<()> {
        let mut child_modules = modules.to_vec();
        child_modules.push(module.ident.to_string());
        let child_public = parent_public && is_public(&module.vis);
        if child_public {
            self.public_paths
                .insert(format!("{crate_name}::{}", child_modules.join("::")));
        }
        if let Some((_, items)) = &module.content {
            return self.collect_items(file, items, crate_name, &child_modules, child_public);
        }
        let child_file = resolve_module_file(&self.crate_src, modules, &module.ident.to_string())?;
        let source =
            fs::read_to_string(&child_file).map_err(|source| Error::io(&child_file, source))?;
        let syntax = syn::parse_file(&source).map_err(|error| {
            Error::message(format!(
                "{}: invalid Rust syntax: {error}",
                child_file.display()
            ))
        })?;
        self.collect_file(
            &child_file,
            &syntax,
            crate_name,
            &child_modules,
            child_public,
        )
    }

    fn resolve_trait_identity(
        &self,
        trait_path: &syn::Path,
        modules: &[String],
    ) -> Option<ProvenTrait> {
        let module = module_key(modules);
        let segments = path_segments(trait_path);
        let candidates = self.path_candidates(&module, &segments);
        let local_traits = candidates
            .iter()
            .filter(|candidate| self.declared_traits.contains(*candidate))
            .collect::<BTreeSet<_>>();
        if !local_traits.is_empty() {
            return (local_traits.len() == 1
                && self
                    .canonical_public_paths
                    .contains_key(*local_traits.first().expect("one local trait")))
            .then_some(ProvenTrait::Public);
        }

        let proven = candidates
            .iter()
            .filter_map(|candidate| standard_trait_identity(candidate))
            .collect::<BTreeSet<_>>();
        if proven.len() == 1 {
            return proven.first().copied();
        }
        if !proven.is_empty() || segments.len() != 1 {
            return None;
        }

        let name = segments.first()?;
        let explicitly_bound = self
            .imports
            .get(&module)
            .is_some_and(|imports| imports.contains_key(name));
        if explicitly_bound {
            return None;
        }
        prelude_trait_identity(name)
    }

    fn collect_impl(&mut self, item_impl: &ItemImpl, crate_name: &str, modules: &[String]) {
        let Some(owner_name) = type_name(&item_impl.self_ty) else {
            return;
        };
        let owner_reference = TypeRef {
            module: module_key(modules),
            segments: type_path_segments(&item_impl.self_ty)
                .unwrap_or_else(|| vec![owner_name.clone()]),
        };
        let owner_path = self
            .resolve_type_ref(&owner_reference)
            .and_then(|item| self.public_type_path(crate_name, &item));
        let owner = owner_path
            .clone()
            .unwrap_or_else(|| path_with(crate_name, modules, &owner_name));
        let trait_path = item_impl.trait_.as_ref().map(|(_, path, _)| path);
        let owner_public = owner_path.is_some();
        let trait_identity = trait_path.and_then(|path| self.resolve_trait_identity(path, modules));
        let public_trait = owner_public && trait_identity.is_some();
        let is_drop = trait_identity == Some(ProvenTrait::Drop);
        let raw_storage = self
            .raw_storage_by_owner
            .get(&item_key(modules, &owner_name))
            .map(|storage| storage.iter().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        for item in &item_impl.items {
            let ImplItem::Fn(method) = item else {
                continue;
            };
            if !attributes_are_enabled(&method.attrs, &self.coordinate) {
                continue;
            }
            let is_reachable = owner_public
                && ((is_public(&method.vis) && item_impl.trait_.is_none()) || public_trait);
            let public_path = is_reachable.then(|| {
                if is_drop {
                    owner.clone()
                } else {
                    format!("{owner}::{}", method.sig.ident)
                }
            });
            if let Some(path) = &public_path {
                self.public_paths.insert(path.clone());
                if !is_drop {
                    self.public_callable_paths.insert(path.clone());
                    if method.sig.unsafety.is_none() {
                        self.public_safe_callable_paths.insert(path.clone());
                    }
                }
            }
            self.nodes.push(node_from_body(
                NodeIdentity {
                    module: module_key(modules),
                    owner: Some(owner_reference.clone()),
                    is_drop,
                    ident: method.sig.ident.to_string(),
                    public_path,
                },
                &method.sig,
                &method.block,
                &raw_storage,
            ));
        }
    }

    fn collect_type(&mut self, modules: &[String], name: &str) {
        let path = item_key(modules, name);
        self.declared_types
            .entry(name.to_owned())
            .or_default()
            .insert(path.clone());
        self.declared_type_paths.insert(path);
    }

    fn collect_imports(&mut self, modules: &[String], item: &ItemUse) {
        let mut entries = Vec::new();
        flatten_use_tree(&item.tree, &mut Vec::new(), &mut entries);
        let mut glob_paths = Vec::new();
        flatten_use_globs(&item.tree, &mut Vec::new(), &mut glob_paths);
        let module = module_key(modules);
        for (alias, raw_path) in entries {
            self.import_bindings
                .entry(module.clone())
                .or_default()
                .entry(alias.clone())
                .or_default()
                .insert(raw_path.clone());
            for path in normalize_import_paths(&module, &raw_path) {
                self.imports
                    .entry(module.clone())
                    .or_default()
                    .entry(alias.clone())
                    .or_default()
                    .insert(path);
            }
        }
        for raw_path in glob_paths {
            for path in normalize_import_paths(&module, &raw_path) {
                self.glob_imports
                    .entry(module.clone())
                    .or_default()
                    .insert(path);
            }
        }
    }

    fn collect_extern_crate(&mut self, modules: &[String], item: &syn::ItemExternCrate) {
        let module = module_key(modules);
        let source = item.ident.to_string();
        let alias = item
            .rename
            .as_ref()
            .map_or_else(|| source.clone(), |(_, alias)| alias.to_string());
        if source == "self" {
            self.self_extern_aliases
                .entry(module.clone())
                .or_default()
                .insert(alias.clone());
        }
        self.import_bindings
            .entry(module.clone())
            .or_default()
            .entry(alias.clone())
            .or_default()
            .insert(vec![source.clone()]);
        self.imports
            .entry(module)
            .or_default()
            .entry(alias)
            .or_default()
            .insert(if source == "self" {
                String::new()
            } else {
                source
            });
    }

    fn collect_raii_fields(
        &mut self,
        crate_name: &str,
        modules: &[String],
        item: &syn::ItemStruct,
    ) {
        let name = item.ident.to_string();
        let public_path = is_public(&item.vis)
            .then(|| self.public_type_path(crate_name, &item_key(modules, &name)))
            .flatten();
        let Some(public_path) = public_path else {
            return;
        };
        for field in &item.fields {
            let mut owned = BTreeSet::new();
            collect_owned_type_refs(&field.ty, &module_key(modules), &mut owned);
            self.raii_contains
                .entry(public_path.clone())
                .or_default()
                .extend(owned);
        }
    }

    fn path_candidates(&self, module: &str, segments: &[String]) -> BTreeSet<String> {
        let mut candidates = BTreeSet::new();
        let Some(first) = segments.first() else {
            return candidates;
        };
        if first == "crate" || first == &self.crate_name {
            if segments.len() > 1 {
                self.expand_qualified_path(&segments[1..].join("::"), &mut candidates);
            }
            return candidates;
        }
        if first == "self" {
            self.expand_qualified_path(&join_path(module, &segments[1..]), &mut candidates);
            return candidates;
        }
        if first == "super" {
            let mut base = split_path(module);
            let mut offset = 0;
            while segments
                .get(offset)
                .is_some_and(|segment| segment == "super")
            {
                if base.pop().is_none() {
                    return BTreeSet::new();
                }
                offset += 1;
            }
            self.expand_qualified_path(&join_segments(&base, &segments[offset..]), &mut candidates);
            return candidates;
        }
        self.expand_scope_candidates(module, segments, &mut BTreeSet::new(), &mut candidates);
        self.expand_qualified_path(&segments.join("::"), &mut candidates);
        candidates
    }

    fn expand_qualified_path(&self, path: &str, candidates: &mut BTreeSet<String>) {
        candidates.insert(path.to_owned());
        let parts = split_path(path);
        if parts.is_empty() {
            return;
        }
        let module = parts[..parts.len() - 1].join("::");
        self.expand_scope_candidates(
            &module,
            &parts[parts.len() - 1..],
            &mut BTreeSet::new(),
            candidates,
        );
    }

    fn expand_scope_candidates(
        &self,
        module: &str,
        segments: &[String],
        visited: &mut BTreeSet<(String, Vec<String>)>,
        candidates: &mut BTreeSet<String>,
    ) {
        if !visited.insert((module.to_owned(), segments.to_vec())) {
            return;
        }
        candidates.insert(join_path(module, segments));
        let Some(first) = segments.first() else {
            return;
        };
        if let Some(imports) = self.imports.get(module).and_then(|items| items.get(first)) {
            for imported in imports {
                candidates.insert(join_path(imported, &segments[1..]));
            }
        }
        if let Some(globs) = self.glob_imports.get(module) {
            for imported_module in globs {
                self.expand_scope_candidates(imported_module, segments, visited, candidates);
            }
        }
    }

    fn resolve_type_ref(&self, reference: &TypeRef) -> Option<String> {
        let exact = self
            .path_candidates(&reference.module, &reference.segments)
            .into_iter()
            .filter(|candidate| self.is_declared_type(candidate))
            .collect::<BTreeSet<_>>();
        if exact.len() == 1 {
            return exact.into_iter().next();
        }
        if !exact.is_empty() {
            return None;
        }
        if reference.segments.len() != 1 {
            return None;
        }
        let ident = reference.segments.last()?;
        let candidates = self.declared_types.get(ident)?;
        (candidates.len() == 1)
            .then(|| candidates.first().cloned())
            .flatten()
    }

    fn is_declared_type(&self, path: &str) -> bool {
        self.declared_type_paths.contains(path)
    }

    fn is_proven_boxdd_ffi_function_path(&self, module: &str, segments: &[String]) -> bool {
        self.has_boxdd_sys_dependency
            && is_boxdd_ffi_function_path_shape(segments)
            && !self.boxdd_sys_is_shadowed(module)
    }

    fn resolve_ffi_type_ref(&self, reference: &TypeRef) -> Option<String> {
        if !self.has_boxdd_sys_dependency || self.boxdd_sys_is_shadowed(&reference.module) {
            return None;
        }
        if self.import_binding_is_ambiguous(&reference.module, &reference.segments) {
            return None;
        }
        let candidates = self.path_candidates(&reference.module, &reference.segments);
        if candidates
            .iter()
            .any(|candidate| self.is_declared_type(candidate))
        {
            return None;
        }
        let raw_types = candidates
            .into_iter()
            .filter(|candidate| is_boxdd_ffi_type_path_shape(&split_path(candidate)))
            .collect::<BTreeSet<_>>();
        (raw_types.len() == 1)
            .then(|| raw_types.into_iter().next())
            .flatten()
    }

    fn boxdd_sys_is_shadowed(&self, module: &str) -> bool {
        let mut scope = split_path(module);
        let mut visited = BTreeSet::new();
        loop {
            if self.scope_shadows_boxdd_sys(&scope.join("::"), &mut visited) {
                return true;
            }
            if scope.pop().is_none() {
                return false;
            }
        }
    }

    fn import_binding_is_ambiguous(&self, module: &str, segments: &[String]) -> bool {
        let Some(first) = segments.first() else {
            return false;
        };
        self.import_bindings
            .get(module)
            .and_then(|bindings| bindings.get(first))
            .is_some_and(|sources| sources.len() != 1)
    }

    fn scope_shadows_boxdd_sys(&self, module: &str, visited: &mut BTreeSet<String>) -> bool {
        if !visited.insert(module.to_owned()) {
            return false;
        }
        if self
            .local_modules
            .get(module)
            .is_some_and(|modules| modules.contains("boxdd_sys"))
            || self
                .self_extern_aliases
                .get(module)
                .is_some_and(|aliases| aliases.contains("boxdd_sys"))
            || self
                .imports
                .get(module)
                .and_then(|imports| imports.get("boxdd_sys"))
                .is_some_and(|paths| !paths.contains("boxdd_sys"))
        {
            return true;
        }

        self.glob_imports.get(module).is_some_and(|imports| {
            imports.iter().any(|imported_module| {
                self.is_declared_module(imported_module)
                    && self.scope_shadows_boxdd_sys(imported_module, visited)
            })
        })
    }

    fn is_declared_module(&self, path: &str) -> bool {
        let mut segments = split_path(path);
        let Some(name) = segments.pop() else {
            return true;
        };
        self.local_modules
            .get(&segments.join("::"))
            .is_some_and(|modules| modules.contains(&name))
    }

    fn resolve_call_target(
        &self,
        caller_index: usize,
        target: &CallTarget,
        parameter_bindings: &BTreeMap<usize, ResolvedTarget>,
        resolved_owners: &[Option<String>],
        by_module_ident: &BTreeMap<(String, String), Vec<usize>>,
        by_owner_ident: &BTreeMap<(String, String), Vec<usize>>,
    ) -> Option<ResolvedTarget> {
        let caller = &self.nodes[caller_index];
        let mut targets = BTreeSet::new();
        match target {
            CallTarget::OptionSome { wrapper, value } => {
                if self.option_some_is_unshadowed(&caller.module, wrapper, by_module_ident)
                    && let Some(target) = self.resolve_call_target(
                        caller_index,
                        value,
                        parameter_bindings,
                        resolved_owners,
                        by_module_ident,
                        by_owner_ident,
                    )
                {
                    targets.insert(target);
                }
            }
            CallTarget::Closure(summary) => {
                targets.insert(ResolvedTarget::Closure {
                    definition_node: caller_index,
                    summary: summary.clone(),
                });
            }
            CallTarget::Parameter(index) => {
                if let Some(target) = parameter_bindings.get(index) {
                    targets.insert(target.clone());
                }
            }
            CallTarget::Method { ident, receiver } => {
                let owner = match receiver {
                    MethodReceiver::CurrentOwner => resolved_owners[caller_index].clone(),
                    MethodReceiver::ExplicitType(segments) => self.resolve_type_ref(&TypeRef {
                        module: caller.module.clone(),
                        segments: segments.clone(),
                    }),
                };
                if let Some(owner) = owner
                    && let Some(nodes) = by_owner_ident.get(&(owner, ident.clone()))
                {
                    targets.extend(nodes.iter().copied().map(ResolvedTarget::Node));
                }
            }
            CallTarget::Path { segments, qself } => {
                let ident = segments.last()?;
                if let Some(qself) = qself {
                    if let Some(owner) = self.resolve_type_ref(&TypeRef {
                        module: caller.module.clone(),
                        segments: qself.clone(),
                    }) && let Some(nodes) = by_owner_ident.get(&(owner, ident.clone()))
                    {
                        targets.extend(nodes.iter().copied().map(ResolvedTarget::Node));
                    }
                } else if segments.first().is_some_and(|segment| segment == "Self") {
                    if segments.len() == 2
                        && let Some(owner) = &resolved_owners[caller_index]
                        && let Some(nodes) = by_owner_ident.get(&(owner.clone(), ident.clone()))
                    {
                        targets.extend(nodes.iter().copied().map(ResolvedTarget::Node));
                    }
                } else {
                    let can_resolve_external_crate = !segments.first().is_some_and(|segment| {
                        matches!(segment.as_str(), "crate" | "self" | "super" | "Self")
                    }) && !self
                        .import_binding_is_ambiguous(&caller.module, segments);
                    for candidate in self.path_candidates(&caller.module, segments) {
                        let parts = split_path(&candidate);
                        let Some(target_ident) = parts.last() else {
                            continue;
                        };
                        if can_resolve_external_crate
                            && self.is_proven_boxdd_ffi_function_path(&caller.module, &parts)
                        {
                            targets.insert(ResolvedTarget::CSymbol(target_ident.clone()));
                        }
                        let target_module = parts[..parts.len() - 1].join("::");
                        if let Some(nodes) =
                            by_module_ident.get(&(target_module.clone(), target_ident.clone()))
                        {
                            targets.extend(nodes.iter().copied().map(ResolvedTarget::Node));
                        }
                        if parts.len() > 1 {
                            let owner = parts[..parts.len() - 1].join("::");
                            if self.is_declared_type(&owner)
                                && let Some(nodes) =
                                    by_owner_ident.get(&(owner, target_ident.clone()))
                            {
                                targets.extend(nodes.iter().copied().map(ResolvedTarget::Node));
                            }
                        }
                    }
                    if segments.len() > 1 {
                        let owner_ref = TypeRef {
                            module: caller.module.clone(),
                            segments: segments[..segments.len() - 1].to_vec(),
                        };
                        if let Some(owner) = self.resolve_type_ref(&owner_ref)
                            && let Some(nodes) = by_owner_ident.get(&(owner, ident.clone()))
                        {
                            targets.extend(nodes.iter().copied().map(ResolvedTarget::Node));
                        }
                    }
                }
            }
        }
        (targets.len() == 1)
            .then(|| targets.into_iter().next())
            .flatten()
    }

    fn option_some_is_unshadowed(
        &self,
        module: &str,
        wrapper: &[String],
        by_module_ident: &BTreeMap<(String, String), Vec<usize>>,
    ) -> bool {
        match wrapper {
            [some] if some == "Some" => {
                !by_module_ident.contains_key(&(module.to_owned(), some.clone()))
                    && !self
                        .imports
                        .get(module)
                        .is_some_and(|imports| imports.contains_key(some))
                    && !self
                        .local_modules
                        .get(module)
                        .is_some_and(|modules| modules.contains(some))
                    && !self.is_declared_type(&join_path(module, wrapper))
            }
            [option, some] if option == "Option" && some == "Some" => {
                self.resolve_type_ref(&TypeRef {
                    module: module.to_owned(),
                    segments: vec![option.clone()],
                })
                .is_none()
                    && !self
                        .imports
                        .get(module)
                        .is_some_and(|imports| imports.contains_key(option))
            }
            [root, option, enum_name, some]
                if matches!(root.as_str(), "core" | "std")
                    && option == "option"
                    && enum_name == "Option"
                    && some == "Some" =>
            {
                !self.root_name_is_shadowed(module, root)
            }
            _ => false,
        }
    }

    fn root_name_is_shadowed(&self, module: &str, name: &str) -> bool {
        let mut scope = split_path(module);
        loop {
            let scope_key = scope.join("::");
            if self
                .local_modules
                .get(&scope_key)
                .is_some_and(|modules| modules.contains(name))
                || self
                    .self_extern_aliases
                    .get(&scope_key)
                    .is_some_and(|aliases| aliases.contains(name))
                || self
                    .imports
                    .get(&scope_key)
                    .is_some_and(|imports| imports.contains_key(name))
            {
                return true;
            }
            if scope.pop().is_none() {
                return false;
            }
        }
    }

    fn finish(self) -> RustIndex {
        let resolved_owners = self
            .nodes
            .iter()
            .map(|node| {
                node.owner
                    .as_ref()
                    .and_then(|owner| self.resolve_type_ref(owner))
            })
            .collect::<Vec<_>>();
        let mut by_module_ident: BTreeMap<(String, String), Vec<usize>> = BTreeMap::new();
        let mut by_owner_ident: BTreeMap<(String, String), Vec<usize>> = BTreeMap::new();
        for (index, node) in self.nodes.iter().enumerate() {
            if node.owner.is_none() {
                by_module_ident
                    .entry((node.module.clone(), node.ident.clone()))
                    .or_default()
                    .push(index);
            }
            if let Some(owner) = &resolved_owners[index] {
                by_owner_ident
                    .entry((owner.clone(), node.ident.clone()))
                    .or_default()
                    .push(index);
            }
        }
        let mut symbol_paths: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut callable_argument_symbol_paths = BTreeMap::new();
        let mut callable_field_argument_symbol_paths = BTreeMap::new();
        let mut ffi_type_paths: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut ffi_field_paths: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
        let mut safe_ffi_type_paths: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut safe_ffi_field_paths: BTreeMap<(String, String), BTreeSet<String>> =
            BTreeMap::new();
        let resolved_node_abi = resolve_node_abi(&self);
        let public_roots = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| {
                node.public_path
                    .as_ref()
                    .map(|public_path| (index, public_path.clone()))
            })
            .collect::<Vec<_>>();
        trace_reachable(
            &self,
            &resolved_node_abi,
            &public_roots,
            &resolved_owners,
            &by_module_ident,
            &by_owner_ident,
            &mut symbol_paths,
            &mut callable_argument_symbol_paths,
            &mut callable_field_argument_symbol_paths,
            &mut ffi_type_paths,
            &mut ffi_field_paths,
        );
        let safe_public_roots = public_roots
            .iter()
            .filter(|(_, public_path)| self.public_safe_callable_paths.contains(public_path))
            .cloned()
            .collect::<Vec<_>>();
        let mut safe_symbols = BTreeMap::new();
        let mut safe_callable_arguments = BTreeMap::new();
        let mut safe_callable_fields = BTreeMap::new();
        trace_reachable(
            &self,
            &resolved_node_abi,
            &safe_public_roots,
            &resolved_owners,
            &by_module_ident,
            &by_owner_ident,
            &mut safe_symbols,
            &mut safe_callable_arguments,
            &mut safe_callable_fields,
            &mut safe_ffi_type_paths,
            &mut safe_ffi_field_paths,
        );

        for (owner, storage) in &self.raw_storage_by_owner {
            let Some(public_path) = self.canonical_public_paths.get(owner) else {
                continue;
            };
            for storage in storage {
                if let Some(raw_type) = self.resolve_ffi_type_ref(&storage.raw_type) {
                    ffi_type_paths
                        .entry(raw_type)
                        .or_default()
                        .insert(public_path.clone());
                    safe_ffi_type_paths
                        .entry(
                            self.resolve_ffi_type_ref(&storage.raw_type)
                                .expect("resolved above"),
                        )
                        .or_default()
                        .insert(public_path.clone());
                }
            }
        }

        let mut conversion_roots = Vec::new();
        for (owner, public_path) in &self.canonical_public_paths {
            for (index, node) in self.nodes.iter().enumerate() {
                if resolved_owners[index].as_ref() == Some(owner)
                    && is_abi_conversion_method(&node.ident)
                {
                    conversion_roots.push((index, public_path.clone()));
                }
            }
        }
        if !conversion_roots.is_empty() {
            let mut conversion_symbols = BTreeMap::new();
            let mut conversion_callable_arguments = BTreeMap::new();
            let mut conversion_callable_fields = BTreeMap::new();
            let mut conversion_fields = BTreeMap::new();
            trace_reachable(
                &self,
                &resolved_node_abi,
                &conversion_roots,
                &resolved_owners,
                &by_module_ident,
                &by_owner_ident,
                &mut conversion_symbols,
                &mut conversion_callable_arguments,
                &mut conversion_callable_fields,
                &mut ffi_type_paths,
                &mut conversion_fields,
            );
        }
        let safe_conversion_roots = conversion_roots
            .iter()
            .filter(|(index, _)| {
                self.nodes[*index]
                    .public_path
                    .as_ref()
                    .is_some_and(|path| self.public_safe_callable_paths.contains(path))
            })
            .cloned()
            .collect::<Vec<_>>();
        if !safe_conversion_roots.is_empty() {
            let mut conversion_symbols = BTreeMap::new();
            let mut conversion_callable_arguments = BTreeMap::new();
            let mut conversion_callable_fields = BTreeMap::new();
            let mut conversion_fields = BTreeMap::new();
            trace_reachable(
                &self,
                &resolved_node_abi,
                &safe_conversion_roots,
                &resolved_owners,
                &by_module_ident,
                &by_owner_ident,
                &mut conversion_symbols,
                &mut conversion_callable_arguments,
                &mut conversion_callable_fields,
                &mut safe_ffi_type_paths,
                &mut conversion_fields,
            );
        }

        for node in &self.nodes {
            for field in &node.abi_fields {
                let (Some(safe_owner), Some(safe_field)) = (&field.safe_owner, &field.safe_field)
                else {
                    continue;
                };
                let Some(owner) = self.resolve_type_ref(safe_owner) else {
                    continue;
                };
                if !self
                    .public_fields_by_owner
                    .get(&owner)
                    .is_some_and(|fields| fields.contains(safe_field))
                {
                    continue;
                }
                let Some(public_owner) = self.canonical_public_paths.get(&owner) else {
                    continue;
                };
                let Some(raw_type) = self.resolve_ffi_type_ref(&field.raw_type) else {
                    continue;
                };
                let public_field = format!("{public_owner}::{safe_field}");
                if self.public_field_paths.contains(&public_field) {
                    ffi_field_paths
                        .entry((raw_type, field.raw_field_chain.clone()))
                        .or_default()
                        .insert(public_field.clone());
                    if node
                        .public_path
                        .as_ref()
                        .is_some_and(|path| self.public_safe_callable_paths.contains(path))
                    {
                        safe_ffi_field_paths
                            .entry((
                                self.resolve_ffi_type_ref(&field.raw_type)
                                    .expect("resolved above"),
                                field.raw_field_chain.clone(),
                            ))
                            .or_default()
                            .insert(public_field);
                    }
                }
            }
        }
        let mut raii_roots = Vec::new();
        for (public_path, contained_types) in &self.raii_contains {
            for contained_type in contained_types {
                let Some(owner) = self.resolve_type_ref(contained_type) else {
                    continue;
                };
                let drop_nodes = self
                    .nodes
                    .iter()
                    .enumerate()
                    .filter_map(|(index, node)| {
                        (node.is_drop && resolved_owners[index].as_ref() == Some(&owner))
                            .then_some(index)
                    })
                    .collect::<Vec<_>>();
                if drop_nodes.len() == 1 {
                    raii_roots.push((drop_nodes[0], public_path.clone()));
                }
            }
        }
        if !raii_roots.is_empty() {
            let mut raii_types = BTreeMap::new();
            let mut raii_fields = BTreeMap::new();
            trace_reachable(
                &self,
                &resolved_node_abi,
                &raii_roots,
                &resolved_owners,
                &by_module_ident,
                &by_owner_ident,
                &mut symbol_paths,
                &mut callable_argument_symbol_paths,
                &mut callable_field_argument_symbol_paths,
                &mut raii_types,
                &mut raii_fields,
            );
        }
        let public_type_paths = self
            .public_type_declarations
            .iter()
            .filter_map(|declaration| self.canonical_public_paths.get(declaration).cloned())
            .collect();
        let callable_return_types = self
            .nodes
            .iter()
            .filter_map(|node| {
                let public_path = node.public_path.as_ref()?;
                let return_type = node.return_type.as_ref()?;
                let declaration = self.resolve_type_ref(return_type)?;
                let public_type = self.canonical_public_paths.get(&declaration)?;
                Some((public_path.clone(), public_type.clone()))
            })
            .collect();
        RustIndex {
            public_paths: self.public_paths,
            public_type_paths,
            public_callable_paths: self.public_callable_paths,
            public_safe_callable_paths: self.public_safe_callable_paths,
            public_field_paths: self.public_field_paths,
            public_alias_paths: self.public_alias_paths,
            callable_return_types,
            symbol_paths,
            callable_argument_symbol_paths,
            callable_field_argument_symbol_paths,
            ffi_type_paths,
            ffi_field_paths,
            safe_ffi_type_paths,
            safe_ffi_field_paths,
        }
    }
}

type ParameterBindings = BTreeMap<usize, ResolvedTarget>;
type TraceBindingKey = (usize, ResolvedTarget);
type ResolvedCallEdges = Vec<ResolvedCallEdge>;

#[derive(Clone)]
struct ResolvedCallEdge {
    target: ResolvedTarget,
    target_bindings: ParameterBindings,
    callable_arguments: BTreeSet<usize>,
    callable_field_arguments: BTreeSet<(usize, String)>,
}

#[derive(Default)]
struct ResolvedNodeAbi {
    types: BTreeSet<String>,
    fields: BTreeSet<(String, String)>,
}

fn resolve_node_abi(builder: &IndexBuilder) -> Vec<ResolvedNodeAbi> {
    builder
        .nodes
        .iter()
        .map(|node| {
            let mut resolved = ResolvedNodeAbi::default();
            for raw_type in &node.abi_types {
                if let Some(raw_type) = builder.resolve_ffi_type_ref(raw_type) {
                    resolved.types.insert(raw_type);
                }
            }
            for field in &node.abi_fields {
                let Some(raw_type) = builder.resolve_ffi_type_ref(&field.raw_type) else {
                    continue;
                };
                resolved.types.insert(raw_type.clone());
                resolved
                    .fields
                    .insert((raw_type, field.raw_field_chain.clone()));
            }
            resolved
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn trace_reachable(
    builder: &IndexBuilder,
    resolved_node_abi: &[ResolvedNodeAbi],
    roots: &[(usize, String)],
    resolved_owners: &[Option<String>],
    by_module_ident: &BTreeMap<(String, String), Vec<usize>>,
    by_owner_ident: &BTreeMap<(String, String), Vec<usize>>,
    symbol_paths: &mut BTreeMap<String, BTreeSet<String>>,
    callable_argument_symbol_paths: &mut BTreeMap<(String, usize), BTreeSet<String>>,
    callable_field_argument_symbol_paths: &mut BTreeMap<(String, usize, String), BTreeSet<String>>,
    type_paths: &mut BTreeMap<String, BTreeSet<String>>,
    field_paths: &mut BTreeMap<(String, String), BTreeSet<String>>,
) {
    let mut public_paths = Vec::<String>::new();
    let mut public_path_ids = BTreeMap::<String, usize>::new();
    let mut indexed_roots = Vec::with_capacity(roots.len());
    for (root, public_path) in roots {
        let path_id = if let Some(path_id) = public_path_ids.get(public_path) {
            *path_id
        } else {
            let path_id = public_paths.len();
            public_paths.push(public_path.clone());
            public_path_ids.insert(public_path.clone(), path_id);
            path_id
        };
        indexed_roots.push((*root, path_id));
    }
    let stable_edges = stabilize_trace_graph(
        builder,
        &indexed_roots,
        resolved_owners,
        by_module_ident,
        by_owner_ident,
    );
    let mut queue = indexed_roots
        .iter()
        .map(|(root, path_id)| (*path_id, ResolvedTarget::Node(*root)))
        .collect::<VecDeque<_>>();
    let mut visited = BTreeSet::new();
    while let Some((path_id, execution)) = queue.pop_front() {
        if !visited.insert((path_id, execution.clone())) {
            continue;
        }
        let public_path = &public_paths[path_id];
        if let ResolvedTarget::Node(index) = &execution {
            let abi = &resolved_node_abi[*index];
            for raw_type in &abi.types {
                type_paths
                    .entry(raw_type.clone())
                    .or_default()
                    .insert(public_path.clone());
            }
            for (raw_type, raw_field_chain) in &abi.fields {
                type_paths
                    .entry(raw_type.clone())
                    .or_default()
                    .insert(public_path.clone());
                field_paths
                    .entry((raw_type.clone(), raw_field_chain.clone()))
                    .or_default()
                    .insert(public_path.clone());
            }
        }

        let Some(edges) = stable_edges.get(&(path_id, execution.clone())) else {
            continue;
        };
        for edge in edges {
            match &edge.target {
                ResolvedTarget::CSymbol(symbol) => {
                    symbol_paths
                        .entry(symbol.clone())
                        .or_default()
                        .insert(public_path.clone());
                    for argument_index in &edge.callable_arguments {
                        callable_argument_symbol_paths
                            .entry((symbol.clone(), *argument_index))
                            .or_default()
                            .insert(public_path.clone());
                    }
                    for (argument_index, raw_field) in &edge.callable_field_arguments {
                        callable_field_argument_symbol_paths
                            .entry((symbol.clone(), *argument_index, raw_field.clone()))
                            .or_default()
                            .insert(public_path.clone());
                    }
                }
                target @ (ResolvedTarget::Node(_) | ResolvedTarget::Closure { .. }) => {
                    queue.push_back((path_id, target.clone()));
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn stabilize_trace_graph(
    builder: &IndexBuilder,
    roots: &[(usize, usize)],
    resolved_owners: &[Option<String>],
    by_module_ident: &BTreeMap<(String, String), Vec<usize>>,
    by_owner_ident: &BTreeMap<(String, String), Vec<usize>>,
) -> BTreeMap<TraceBindingKey, ResolvedCallEdges> {
    let mut stable = BTreeMap::<TraceBindingKey, ParameterBindings>::new();
    let mut stable_edges = BTreeMap::<TraceBindingKey, ResolvedCallEdges>::new();
    let mut edge_cache = BTreeMap::<(ResolvedTarget, ParameterBindings), ResolvedCallEdges>::new();
    let mut queue = roots
        .iter()
        .map(|(root, path_id)| {
            (
                *path_id,
                ResolvedTarget::Node(*root),
                ParameterBindings::new(),
            )
        })
        .collect::<VecDeque<_>>();

    while let Some((path_id, execution, incoming)) = queue.pop_front() {
        let key = (path_id, execution.clone());
        let bindings = if let Some(current) = stable.get_mut(&key) {
            let previous_len = current.len();
            current.retain(|parameter, target| incoming.get(parameter) == Some(target));
            if current.len() == previous_len {
                continue;
            }
            current.clone()
        } else {
            stable.insert(key.clone(), incoming.clone());
            incoming
        };

        let cache_key = (execution.clone(), bindings.clone());
        let edges = if let Some(edges) = edge_cache.get(&cache_key) {
            edges.clone()
        } else {
            let edges = resolved_call_edges(
                builder,
                &execution,
                &bindings,
                resolved_owners,
                by_module_ident,
                by_owner_ident,
            );
            edge_cache.insert(cache_key, edges.clone());
            edges
        };
        stable_edges.insert(key, edges.clone());
        for edge in edges {
            let ResolvedCallEdge {
                target,
                target_bindings,
                ..
            } = edge;
            if matches!(
                target,
                ResolvedTarget::Node(_) | ResolvedTarget::Closure { .. }
            ) {
                queue.push_back((path_id, target, target_bindings));
            }
        }
    }
    stable_edges
}

#[allow(clippy::too_many_arguments)]
fn resolved_call_edges(
    builder: &IndexBuilder,
    execution: &ResolvedTarget,
    parameter_bindings: &ParameterBindings,
    resolved_owners: &[Option<String>],
    by_module_ident: &BTreeMap<(String, String), Vec<usize>>,
    by_owner_ident: &BTreeMap<(String, String), Vec<usize>>,
) -> ResolvedCallEdges {
    let (context_index, calls) = match execution {
        ResolvedTarget::Node(index) => (*index, &builder.nodes[*index].calls),
        ResolvedTarget::Closure {
            definition_node,
            summary,
        } => (*definition_node, &summary.calls),
        ResolvedTarget::CSymbol(_) => return Vec::new(),
    };
    let mut edges = Vec::new();
    for call in calls {
        let Some(target) = builder.resolve_call_target(
            context_index,
            &call.target,
            parameter_bindings,
            resolved_owners,
            by_module_ident,
            by_owner_ident,
        ) else {
            continue;
        };
        let mut target_bindings = ParameterBindings::new();
        let mut callable_arguments = BTreeSet::new();
        for (argument_index, argument) in call.arguments.iter().enumerate() {
            let Some(argument) = argument else {
                continue;
            };
            if let Some(resolved) = builder.resolve_call_target(
                context_index,
                argument,
                parameter_bindings,
                resolved_owners,
                by_module_ident,
                by_owner_ident,
            ) {
                if matches!(
                    resolved,
                    ResolvedTarget::Node(_) | ResolvedTarget::Closure { .. }
                ) {
                    callable_arguments.insert(argument_index);
                }
                target_bindings.insert(argument_index + call.parameter_offset, resolved);
            }
        }
        let mut callable_field_arguments = BTreeSet::new();
        for (argument_index, fields) in &call.callable_field_arguments {
            for (raw_field, callable) in fields {
                if builder
                    .resolve_call_target(
                        context_index,
                        callable,
                        parameter_bindings,
                        resolved_owners,
                        by_module_ident,
                        by_owner_ident,
                    )
                    .is_some_and(|target| {
                        matches!(
                            target,
                            ResolvedTarget::Node(_) | ResolvedTarget::Closure { .. }
                        )
                    })
                {
                    callable_field_arguments.insert((*argument_index, raw_field.clone()));
                }
            }
        }
        edges.push(ResolvedCallEdge {
            target,
            target_bindings,
            callable_arguments,
            callable_field_arguments,
        });
    }
    edges
}

struct NodeIdentity {
    module: String,
    owner: Option<TypeRef>,
    is_drop: bool,
    ident: String,
    public_path: Option<String>,
}

fn node_from_body(
    identity: NodeIdentity,
    signature: &Signature,
    body: &syn::Block,
    raw_storage: &[RawStorageRef],
) -> Node {
    let NodeIdentity {
        module,
        owner,
        is_drop,
        ident,
        public_path,
    } = identity;
    let mut calls = CallVisitor::new(signature);
    calls.visit_block(body);
    let mut abi = AbiWitnessVisitor::new(&module, owner.as_ref(), signature, raw_storage);
    abi.visit_block(body);
    let return_type =
        return_value_type(&signature.output, owner.as_ref()).map(|segments| TypeRef {
            module: module.clone(),
            segments,
        });
    Node {
        module,
        owner,
        is_drop,
        ident,
        public_path,
        calls: calls.calls,
        abi_types: abi.types,
        abi_fields: abi.fields,
        return_type,
    }
}

fn return_value_type(output: &syn::ReturnType, owner: Option<&TypeRef>) -> Option<Vec<String>> {
    let syn::ReturnType::Type(_, ty) = output else {
        return None;
    };
    return_value_type_inner(ty, owner)
}

fn return_value_type_inner(ty: &Type, owner: Option<&TypeRef>) -> Option<Vec<String>> {
    match ty {
        Type::Group(group) => return_value_type_inner(&group.elem, owner),
        Type::Paren(paren) => return_value_type_inner(&paren.elem, owner),
        Type::Reference(reference) => return_value_type_inner(&reference.elem, owner),
        Type::Path(path) if path.qself.is_none() => {
            let segments = path_segments(&path.path);
            if segments.as_slice() == ["Self"] {
                return owner.map(|owner| owner.segments.clone());
            }
            let last = path.path.segments.last()?;
            if matches!(
                last.ident.to_string().as_str(),
                "Result" | "Option" | "ApiResult" | "Box" | "Arc" | "Rc"
            ) && let syn::PathArguments::AngleBracketed(arguments) = &last.arguments
            {
                return arguments.args.iter().find_map(|argument| {
                    let syn::GenericArgument::Type(ty) = argument else {
                        return None;
                    };
                    return_value_type_inner(ty, owner)
                });
            }
            Some(segments)
        }
        _ => None,
    }
}

#[derive(Clone, Debug)]
enum LocalBinding {
    Callable(CallTarget),
    Shadow,
}

struct CallVisitor {
    calls: BTreeSet<CallRef>,
    bindings: Vec<BTreeMap<String, LocalBinding>>,
    callable_fields: Vec<BTreeMap<String, BTreeMap<String, CallTarget>>>,
}

impl CallVisitor {
    fn new(signature: &Signature) -> Self {
        let mut parameters = BTreeMap::new();
        for (index, argument) in signature.inputs.iter().enumerate() {
            if let FnArg::Typed(argument) = argument {
                let mut names = BTreeSet::new();
                collect_pattern_idents(&argument.pat, &mut names);
                let simple = simple_pattern_ident(&argument.pat);
                for name in names {
                    let binding = if simple.as_deref() == Some(&name) {
                        LocalBinding::Callable(CallTarget::Parameter(index))
                    } else {
                        LocalBinding::Shadow
                    };
                    parameters.insert(name, binding);
                }
            }
        }
        Self {
            calls: BTreeSet::new(),
            bindings: vec![parameters],
            callable_fields: vec![BTreeMap::new()],
        }
    }

    fn for_closure(closure: &syn::ExprClosure) -> Self {
        let mut parameters = BTreeMap::new();
        for (index, pattern) in closure.inputs.iter().enumerate() {
            let mut names = BTreeSet::new();
            collect_pattern_idents(pattern, &mut names);
            let simple = simple_pattern_ident(pattern);
            for name in names {
                let binding = if simple.as_deref() == Some(&name) {
                    LocalBinding::Callable(CallTarget::Parameter(index))
                } else {
                    LocalBinding::Shadow
                };
                parameters.insert(name, binding);
            }
        }
        Self {
            calls: BTreeSet::new(),
            bindings: vec![parameters],
            callable_fields: vec![BTreeMap::new()],
        }
    }

    fn binding(&self, ident: &str) -> Option<&LocalBinding> {
        self.bindings
            .iter()
            .rev()
            .find_map(|scope| scope.get(ident))
    }

    fn bind(&mut self, ident: String, binding: LocalBinding) {
        self.bindings
            .last_mut()
            .expect("call visitor always has a scope")
            .insert(ident, binding);
    }

    fn invalidate(&mut self, ident: &str) {
        if let Some(scope) = self
            .bindings
            .iter_mut()
            .rev()
            .find(|scope| scope.contains_key(ident))
        {
            scope.insert(ident.to_owned(), LocalBinding::Shadow);
        }
    }

    fn callable_fields(&self, ident: &str) -> Option<&BTreeMap<String, CallTarget>> {
        self.callable_fields
            .iter()
            .rev()
            .find_map(|scope| scope.get(ident))
    }

    fn bind_callable_fields(&mut self, ident: String, fields: BTreeMap<String, CallTarget>) {
        self.callable_fields
            .last_mut()
            .expect("call visitor always has a field scope")
            .insert(ident, fields);
    }

    fn clear_callable_fields(&mut self, ident: &str) {
        if let Some(fields) = self
            .callable_fields
            .iter_mut()
            .rev()
            .find_map(|scope| scope.get_mut(ident))
        {
            fields.clear();
        }
    }

    fn set_callable_field(&mut self, ident: &str, field: String, target: Option<CallTarget>) {
        let Some(fields) = self
            .callable_fields
            .iter_mut()
            .rev()
            .find_map(|scope| scope.get_mut(ident))
        else {
            return;
        };
        if let Some(target) = target {
            fields.insert(field, target);
        } else {
            fields.remove(&field);
        }
    }

    fn callable_field_arguments<'a>(
        &self,
        arguments: impl IntoIterator<Item = (usize, &'a Expr)>,
    ) -> BTreeMap<usize, BTreeMap<String, CallTarget>> {
        arguments
            .into_iter()
            .filter_map(|(index, expression)| {
                let ident = expression_binding_ident(expression)?;
                let fields = self.callable_fields(&ident)?.clone();
                (!fields.is_empty()).then_some((index, fields))
            })
            .collect()
    }

    fn callable_fields_from_initializer(&self, expression: &Expr) -> BTreeMap<String, CallTarget> {
        let Expr::Struct(structure) = strip_expression_wrappers(expression) else {
            return BTreeMap::new();
        };
        structure
            .fields
            .iter()
            .filter_map(|field| {
                self.alias_from_expr(&field.expr)
                    .map(|target| (member_name(&field.member), target))
            })
            .collect()
    }

    fn alias_from_expr(&self, expression: &Expr) -> Option<CallTarget> {
        match expression {
            Expr::Path(path) if path.qself.is_none() => {
                let segments = path_segments(&path.path);
                let ident = segments.last()?;
                if segments.len() == 1 {
                    match self.binding(ident) {
                        Some(LocalBinding::Callable(target)) => Some(target.clone()),
                        Some(LocalBinding::Shadow) => None,
                        None => Some(CallTarget::Path {
                            segments,
                            qself: None,
                        }),
                    }
                } else {
                    Some(CallTarget::Path {
                        segments,
                        qself: None,
                    })
                }
            }
            Expr::Path(path) => Some(CallTarget::Path {
                segments: path_segments(&path.path),
                qself: path
                    .qself
                    .as_ref()
                    .and_then(|qself| type_path_segments(&qself.ty)),
            }),
            Expr::Cast(cast) => self.alias_from_expr(&cast.expr),
            Expr::Group(group) => self.alias_from_expr(&group.expr),
            Expr::Paren(paren) => self.alias_from_expr(&paren.expr),
            Expr::Reference(reference) => self.alias_from_expr(&reference.expr),
            Expr::Closure(closure) => {
                let mut visitor = Self::for_closure(closure);
                visitor.visit_expr(&closure.body);
                Some(CallTarget::Closure(Box::new(ClosureSummary {
                    calls: visitor.calls,
                })))
            }
            Expr::Call(call) if call.args.len() == 1 => {
                let Expr::Path(wrapper) = call.func.as_ref() else {
                    return None;
                };
                if wrapper.qself.is_some() {
                    return None;
                }
                let wrapper = path_segments(&wrapper.path);
                if !is_option_some_wrapper(&wrapper) {
                    return None;
                }
                Some(CallTarget::OptionSome {
                    wrapper,
                    value: Box::new(self.alias_from_expr(&call.args[0])?),
                })
            }
            _ => None,
        }
    }
}

fn is_option_some_wrapper(segments: &[String]) -> bool {
    matches!(segments, [some] if some == "Some")
        || matches!(segments, [option, some] if option == "Option" && some == "Some")
        || matches!(
            segments,
            [root, option, enum_name, some]
                if matches!(root.as_str(), "core" | "std")
                    && option == "option"
                    && enum_name == "Option"
                    && some == "Some"
        )
}

fn expression_binding_ident(expression: &Expr) -> Option<String> {
    let Expr::Path(path) = strip_expression_wrappers(expression) else {
        return None;
    };
    (path.qself.is_none() && path.path.segments.len() == 1)
        .then(|| path.path.segments[0].ident.to_string())
}

fn escaping_binding_ident(expression: &Expr) -> Option<String> {
    let mut expression = expression;
    loop {
        expression = match expression {
            Expr::Group(group) => &group.expr,
            Expr::Paren(paren) => &paren.expr,
            Expr::Reference(reference) if reference.mutability.is_some() => &reference.expr,
            Expr::Reference(_) => return None,
            _ => return expression_binding_ident(expression),
        };
    }
}

impl<'ast> Visit<'ast> for CallVisitor {
    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.bindings.push(BTreeMap::new());
        self.callable_fields.push(BTreeMap::new());
        for statement in &block.stmts {
            self.visit_stmt(statement);
            if statement_definitely_stops(statement) {
                break;
            }
        }
        self.callable_fields.pop();
        self.bindings.pop();
    }

    fn visit_expr_if(&mut self, expression: &'ast syn::ExprIf) {
        self.visit_expr(&expression.cond);
        match constant_bool(&expression.cond) {
            Some(true) => self.visit_block(&expression.then_branch),
            Some(false) => {
                if let Some((_, alternate)) = &expression.else_branch {
                    self.visit_expr(alternate);
                }
            }
            None => {
                self.visit_block(&expression.then_branch);
                if let Some((_, alternate)) = &expression.else_branch {
                    self.visit_expr(alternate);
                }
            }
        }
    }

    fn visit_expr_while(&mut self, expression: &'ast syn::ExprWhile) {
        self.visit_expr(&expression.cond);
        if constant_bool(&expression.cond) != Some(false) {
            self.visit_block(&expression.body);
        }
    }

    fn visit_expr_match(&mut self, expression: &'ast syn::ExprMatch) {
        self.visit_expr(&expression.expr);
        let Some(value) = constant_bool(&expression.expr) else {
            for arm in &expression.arms {
                if let Some((_, guard)) = &arm.guard {
                    self.visit_expr(guard);
                }
                self.visit_expr(&arm.body);
            }
            return;
        };
        for arm in &expression.arms {
            match bool_pattern_matches(&arm.pat, value) {
                Some(false) => continue,
                Some(true) if arm.guard.is_none() => {
                    self.visit_expr(&arm.body);
                    return;
                }
                _ => {
                    if let Some((_, guard)) = &arm.guard {
                        self.visit_expr(guard);
                    }
                    self.visit_expr(&arm.body);
                }
            }
        }
    }

    fn visit_expr_binary(&mut self, expression: &'ast syn::ExprBinary) {
        if !matches!(expression.op, syn::BinOp::And(_) | syn::BinOp::Or(_)) {
            syn::visit::visit_expr_binary(self, expression);
            return;
        }
        self.visit_expr(&expression.left);
        let left = constant_bool(&expression.left);
        let right_executes = match expression.op {
            syn::BinOp::And(_) => left != Some(false),
            syn::BinOp::Or(_) => left != Some(true),
            _ => unreachable!(),
        };
        if right_executes {
            self.visit_expr(&expression.right);
        }
    }

    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if let Some(target) = self.alias_from_expr(&call.func) {
            self.calls.insert(CallRef {
                target,
                arguments: call
                    .args
                    .iter()
                    .map(|argument| self.alias_from_expr(argument))
                    .collect(),
                callable_field_arguments: self
                    .callable_field_arguments(call.args.iter().enumerate()),
                parameter_offset: 0,
            });
        } else if !matches!(call.func.as_ref(), Expr::Path(_)) {
            self.visit_expr(&call.func);
        }
        for argument in &call.args {
            self.visit_expr(argument);
        }
        for argument in &call.args {
            if let Some(ident) = escaping_binding_ident(argument) {
                self.clear_callable_fields(&ident);
            }
        }
    }

    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        let receiver = if is_self_receiver(&call.receiver) {
            Some(MethodReceiver::CurrentOwner)
        } else {
            explicit_receiver_type(&call.receiver).map(|segments| {
                if segments.as_slice() == ["Self"] {
                    MethodReceiver::CurrentOwner
                } else {
                    MethodReceiver::ExplicitType(segments)
                }
            })
        };
        if let Some(receiver) = receiver {
            self.calls.insert(CallRef {
                target: CallTarget::Method {
                    ident: call.method.to_string(),
                    receiver,
                },
                arguments: call
                    .args
                    .iter()
                    .map(|argument| self.alias_from_expr(argument))
                    .collect(),
                callable_field_arguments: self
                    .callable_field_arguments(call.args.iter().enumerate()),
                parameter_offset: 1,
            });
        }
        self.visit_expr(&call.receiver);
        for argument in &call.args {
            self.visit_expr(argument);
        }
        for argument in &call.args {
            if let Some(ident) = escaping_binding_ident(argument) {
                self.clear_callable_fields(&ident);
            }
        }
        if let Some(ident) = expression_binding_ident(&call.receiver) {
            self.clear_callable_fields(&ident);
        }
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        let alias = local
            .init
            .as_ref()
            .and_then(|init| self.alias_from_expr(&init.expr));
        let callable_fields = local.init.as_ref().map_or_else(BTreeMap::new, |init| {
            self.callable_fields_from_initializer(&init.expr)
        });
        if let Some(init) = &local.init {
            self.visit_expr(&init.expr);
            if let Some((_, diverge)) = &init.diverge {
                self.visit_expr(diverge);
            }
        }
        let mut names = BTreeSet::new();
        collect_pattern_idents(&local.pat, &mut names);
        let simple_binding = simple_pattern_ident(&local.pat);
        for name in names {
            self.bind_callable_fields(
                name.clone(),
                if simple_binding.as_deref() == Some(&name) {
                    callable_fields.clone()
                } else {
                    BTreeMap::new()
                },
            );
            let binding = if simple_binding.as_deref() == Some(&name) {
                alias.as_ref().map_or(LocalBinding::Shadow, |target| {
                    LocalBinding::Callable(target.clone())
                })
            } else {
                LocalBinding::Shadow
            };
            self.bind(name, binding);
        }
    }

    fn visit_expr_assign(&mut self, assign: &'ast syn::ExprAssign) {
        let field_update = expression_member_chain(&assign.left).and_then(|(base, members)| {
            let Expr::Path(path) = base else {
                return None;
            };
            if path.qself.is_some() || path.path.segments.len() != 1 || members.is_empty() {
                return None;
            }
            Some((
                path.path.segments[0].ident.to_string(),
                members.join("::"),
                self.alias_from_expr(&assign.right),
            ))
        });
        self.visit_expr(&assign.left);
        self.visit_expr(&assign.right);
        if let Some((ident, field, target)) = field_update {
            self.set_callable_field(&ident, field, target);
        }
        if let Expr::Path(path) = assign.left.as_ref()
            && path.qself.is_none()
            && path.path.segments.len() == 1
        {
            let ident = path.path.segments[0].ident.to_string();
            self.invalidate(&ident);
            self.clear_callable_fields(&ident);
        }
    }

    fn visit_item(&mut self, _item: &'ast Item) {}

    fn visit_expr_closure(&mut self, _closure: &'ast syn::ExprClosure) {}

    fn visit_expr_async(&mut self, _async: &'ast syn::ExprAsync) {}

    fn visit_expr_const(&mut self, _constant: &'ast syn::ExprConst) {}
}

#[derive(Clone, Debug)]
enum AbiBinding {
    Typed(TypeRef),
    Shadow,
}

struct AbiWitnessVisitor {
    module: String,
    owner: Option<TypeRef>,
    raw_storage: Vec<RawStorageRef>,
    types: BTreeSet<TypeRef>,
    fields: BTreeSet<AbiFieldRef>,
    bindings: Vec<BTreeMap<String, AbiBinding>>,
}

impl AbiWitnessVisitor {
    fn new(
        module: &str,
        owner: Option<&TypeRef>,
        signature: &Signature,
        raw_storage: &[RawStorageRef],
    ) -> Self {
        let mut parameters = BTreeMap::new();
        for argument in &signature.inputs {
            let FnArg::Typed(argument) = argument else {
                continue;
            };
            let Some(name) = simple_pattern_ident(&argument.pat) else {
                continue;
            };
            let binding =
                direct_value_type_path(&argument.ty).map_or(AbiBinding::Shadow, |segments| {
                    AbiBinding::Typed(TypeRef {
                        module: module.to_owned(),
                        segments,
                    })
                });
            parameters.insert(name, binding);
        }
        let mut visitor = Self {
            module: module.to_owned(),
            owner: owner.cloned(),
            raw_storage: raw_storage.to_vec(),
            types: BTreeSet::new(),
            fields: BTreeSet::new(),
            bindings: vec![parameters],
        };
        visitor.visit_signature(signature);
        visitor
    }

    fn binding(&self, ident: &str) -> Option<&AbiBinding> {
        self.bindings
            .iter()
            .rev()
            .find_map(|scope| scope.get(ident))
    }

    fn bind(&mut self, ident: String, binding: AbiBinding) {
        self.bindings
            .last_mut()
            .expect("ABI visitor always has a scope")
            .insert(ident, binding);
    }

    fn invalidate(&mut self, ident: &str) {
        if let Some(scope) = self
            .bindings
            .iter_mut()
            .rev()
            .find(|scope| scope.contains_key(ident))
        {
            scope.insert(ident.to_owned(), AbiBinding::Shadow);
        }
    }

    fn expression_type_ref(&self, expression: &Expr) -> Option<TypeRef> {
        let segments = match expression {
            Expr::Struct(structure) => Some(path_segments(&structure.path)),
            Expr::Cast(cast) => direct_value_type_path(&cast.ty),
            Expr::Group(group) => return self.expression_type_ref(&group.expr),
            Expr::Paren(paren) => return self.expression_type_ref(&paren.expr),
            Expr::Reference(reference) => return self.expression_type_ref(&reference.expr),
            _ => None,
        }?;
        Some(TypeRef {
            module: self.module.clone(),
            segments,
        })
    }

    fn raw_field_access(&self, expression: &Expr) -> Option<(TypeRef, String)> {
        let (base, members) = expression_member_chain(expression)?;
        if members.is_empty() {
            return None;
        }
        let Expr::Path(path) = strip_expression_wrappers(base) else {
            return None;
        };
        if path.qself.is_some() || path.path.segments.len() != 1 {
            return None;
        }
        let ident = path.path.segments[0].ident.to_string();
        if ident == "self" {
            let storage = self
                .raw_storage
                .iter()
                .filter(|storage| members.first() == Some(&storage.member))
                .collect::<Vec<_>>();
            let [storage] = storage.as_slice() else {
                return None;
            };
            let chain = members[1..].join("::");
            return (!chain.is_empty()).then(|| (storage.raw_type.clone(), chain));
        }
        match self.binding(&ident) {
            Some(AbiBinding::Typed(raw_type)) => Some((raw_type.clone(), members.join("::"))),
            Some(AbiBinding::Shadow) | None => None,
        }
    }

    fn raw_accesses_in(&self, expression: &Expr) -> BTreeSet<(TypeRef, String)> {
        let mut collector = RawFieldCollector {
            visitor: self,
            fields: BTreeSet::new(),
        };
        collector.visit_expr(expression);
        collector.fields
    }

    fn safe_fields_in(&self, expression: &Expr) -> BTreeSet<String> {
        let mut collector = SafeFieldCollector::default();
        collector.visit_expr(expression);
        collector.fields
    }

    fn structure_owner(&self, structure: &syn::ExprStruct) -> Option<TypeRef> {
        let segments = path_segments(&structure.path);
        if segments.as_slice() == ["Self"] {
            self.owner.clone()
        } else {
            Some(TypeRef {
                module: self.module.clone(),
                segments,
            })
        }
    }

    fn record_relation(
        &mut self,
        raw_type: TypeRef,
        raw_field_chain: String,
        safe_owner: Option<TypeRef>,
        safe_field: Option<String>,
    ) {
        if raw_field_chain.is_empty() {
            return;
        }
        self.types.insert(raw_type.clone());
        self.fields.insert(AbiFieldRef {
            raw_type,
            raw_field_chain,
            safe_owner,
            safe_field,
        });
    }
}

impl<'ast> Visit<'ast> for AbiWitnessVisitor {
    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.bindings.push(BTreeMap::new());
        for statement in &block.stmts {
            self.visit_stmt(statement);
        }
        self.bindings.pop();
    }

    fn visit_type_path(&mut self, ty: &'ast syn::TypePath) {
        if ty.qself.is_none() {
            let segments = path_segments(&ty.path);
            if !segments.is_empty() {
                self.types.insert(TypeRef {
                    module: self.module.clone(),
                    segments,
                });
            }
        }
        syn::visit::visit_type_path(self, ty);
    }

    fn visit_expr_struct(&mut self, structure: &'ast syn::ExprStruct) {
        let structure_type = TypeRef {
            module: self.module.clone(),
            segments: path_segments(&structure.path),
        };
        self.types.insert(structure_type.clone());
        let safe_owner = self.structure_owner(structure);
        for field in &structure.fields {
            let raw_field = member_name(&field.member);
            for (raw_type, raw_chain) in self.raw_accesses_in(&field.expr) {
                self.record_relation(
                    raw_type,
                    raw_chain,
                    safe_owner.clone(),
                    named_member(&field.member),
                );
            }
            let safe_fields = self.safe_fields_in(&field.expr);
            if safe_fields.is_empty() {
                self.record_relation(structure_type.clone(), raw_field, None, None);
            } else {
                for safe_field in safe_fields {
                    self.record_relation(
                        structure_type.clone(),
                        raw_field.clone(),
                        self.owner.clone(),
                        Some(safe_field),
                    );
                }
            }
        }
        syn::visit::visit_expr_struct(self, structure);
    }

    fn visit_expr_field(&mut self, field: &'ast syn::ExprField) {
        if let Some((raw_type, raw_chain)) = self.raw_field_access(&Expr::Field(field.clone())) {
            self.record_relation(raw_type, raw_chain, None, None);
        }
        syn::visit::visit_expr_field(self, field);
    }

    fn visit_expr_assign(&mut self, assign: &'ast syn::ExprAssign) {
        let left_raw = self.raw_accesses_in(&assign.left);
        let right_raw = self.raw_accesses_in(&assign.right);
        let left_safe = self.safe_fields_in(&assign.left);
        let right_safe = self.safe_fields_in(&assign.right);
        for (raw_type, raw_chain) in left_raw {
            if right_safe.is_empty() {
                self.record_relation(raw_type, raw_chain, None, None);
            } else {
                for safe_field in &right_safe {
                    self.record_relation(
                        raw_type.clone(),
                        raw_chain.clone(),
                        self.owner.clone(),
                        Some(safe_field.clone()),
                    );
                }
            }
        }
        for (raw_type, raw_chain) in right_raw {
            if left_safe.is_empty() {
                self.record_relation(raw_type, raw_chain, None, None);
            } else {
                for safe_field in &left_safe {
                    self.record_relation(
                        raw_type.clone(),
                        raw_chain.clone(),
                        self.owner.clone(),
                        Some(safe_field.clone()),
                    );
                }
            }
        }
        self.visit_expr(&assign.left);
        self.visit_expr(&assign.right);
        if let Expr::Path(path) = assign.left.as_ref()
            && path.qself.is_none()
            && path.path.segments.len() == 1
        {
            self.invalidate(&path.path.segments[0].ident.to_string());
        }
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        let explicit_type = pattern_type(&local.pat).and_then(direct_value_type_path);
        self.visit_pat(&local.pat);
        if let Some(init) = &local.init {
            self.visit_expr(&init.expr);
            if let Some((_, diverge)) = &init.diverge {
                self.visit_expr(diverge);
            }
        }
        let inferred = local
            .init
            .as_ref()
            .and_then(|init| self.expression_type_ref(&init.expr));
        let binding = explicit_type
            .map(|segments| TypeRef {
                module: self.module.clone(),
                segments,
            })
            .or(inferred)
            .map_or(AbiBinding::Shadow, AbiBinding::Typed);
        let mut names = BTreeSet::new();
        collect_pattern_idents(&local.pat, &mut names);
        let simple = simple_local_pattern_ident(&local.pat);
        for name in names {
            self.bind(
                name.clone(),
                if simple.as_deref() == Some(&name) {
                    binding.clone()
                } else {
                    AbiBinding::Shadow
                },
            );
        }
    }

    fn visit_item(&mut self, _item: &'ast Item) {}

    fn visit_expr_closure(&mut self, _closure: &'ast syn::ExprClosure) {}

    fn visit_expr_async(&mut self, _async: &'ast syn::ExprAsync) {}
}

struct RawFieldCollector<'a> {
    visitor: &'a AbiWitnessVisitor,
    fields: BTreeSet<(TypeRef, String)>,
}

impl<'ast> Visit<'ast> for RawFieldCollector<'_> {
    fn visit_expr_field(&mut self, field: &'ast syn::ExprField) {
        if let Some(raw) = self.visitor.raw_field_access(&Expr::Field(field.clone())) {
            self.fields.insert(raw);
        }
        syn::visit::visit_expr_field(self, field);
    }

    fn visit_item(&mut self, _item: &'ast Item) {}

    fn visit_expr_closure(&mut self, _closure: &'ast syn::ExprClosure) {}

    fn visit_expr_async(&mut self, _async: &'ast syn::ExprAsync) {}
}

#[derive(Default)]
struct SafeFieldCollector {
    fields: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for SafeFieldCollector {
    fn visit_expr_field(&mut self, field: &'ast syn::ExprField) {
        if let Some((base, members)) = expression_member_chain(&Expr::Field(field.clone()))
            && let Expr::Path(path) = strip_expression_wrappers(base)
            && path.qself.is_none()
            && path.path.segments.len() == 1
            && path.path.segments[0].ident == "self"
            && let Some(member) = members.first()
            && member.parse::<usize>().is_err()
        {
            self.fields.insert(member.clone());
        }
        syn::visit::visit_expr_field(self, field);
    }

    fn visit_item(&mut self, _item: &'ast Item) {}

    fn visit_expr_closure(&mut self, _closure: &'ast syn::ExprClosure) {}

    fn visit_expr_async(&mut self, _async: &'ast syn::ExprAsync) {}
}

fn strip_expression_wrappers(mut expression: &Expr) -> &Expr {
    loop {
        expression = match expression {
            Expr::Group(group) => &group.expr,
            Expr::Paren(paren) => &paren.expr,
            Expr::Reference(reference) => &reference.expr,
            Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_)) => &unary.expr,
            _ => return expression,
        };
    }
}

fn expression_member_chain(expression: &Expr) -> Option<(&Expr, Vec<String>)> {
    let mut expression = strip_expression_wrappers(expression);
    let mut members = Vec::new();
    while let Expr::Field(field) = expression {
        members.push(member_name(&field.member));
        expression = strip_expression_wrappers(&field.base);
    }
    members.reverse();
    Some((expression, members))
}

fn member_name(member: &syn::Member) -> String {
    match member {
        syn::Member::Named(ident) => ident.to_string(),
        syn::Member::Unnamed(index) => index.index.to_string(),
    }
}

fn named_member(member: &syn::Member) -> Option<String> {
    match member {
        syn::Member::Named(ident) => Some(ident.to_string()),
        syn::Member::Unnamed(_) => None,
    }
}

fn path_segments(path: &syn::Path) -> Vec<String> {
    path.segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect()
}

fn is_boxdd_ffi_function_path_shape(segments: &[String]) -> bool {
    segments.len() == 3
        && segments[0] == "boxdd_sys"
        && segments[1] == "ffi"
        && segments[2].starts_with("b2")
}

fn is_boxdd_ffi_type_path_shape(segments: &[String]) -> bool {
    segments.len() == 3
        && segments[0] == "boxdd_sys"
        && segments[1] == "ffi"
        && segments[2].starts_with("b2")
}

fn is_abi_conversion_method(ident: &str) -> bool {
    matches!(
        ident,
        "from_raw"
            | "into_raw"
            | "as_raw"
            | "to_raw"
            | "raw"
            | "raw_mut"
            | "from_ffi"
            | "into_ffi"
            | "as_ffi"
            | "to_ffi"
            | "new"
            | "try_new"
            | "default"
    )
}

fn is_self_receiver(expression: &Expr) -> bool {
    match expression {
        Expr::Path(path) => {
            path.qself.is_none()
                && path.path.segments.len() == 1
                && path.path.segments[0].ident == "self"
        }
        Expr::Group(group) => is_self_receiver(&group.expr),
        Expr::Paren(paren) => is_self_receiver(&paren.expr),
        Expr::Reference(reference) => is_self_receiver(&reference.expr),
        Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_)) => {
            is_self_receiver(&unary.expr)
        }
        _ => false,
    }
}

fn explicit_receiver_type(expression: &Expr) -> Option<Vec<String>> {
    match expression {
        Expr::Struct(structure) => Some(path_segments(&structure.path)),
        Expr::Cast(cast) => type_path_segments(&cast.ty),
        Expr::Group(group) => explicit_receiver_type(&group.expr),
        Expr::Paren(paren) => explicit_receiver_type(&paren.expr),
        Expr::Reference(reference) => explicit_receiver_type(&reference.expr),
        _ => None,
    }
}

pub fn discover_test_evidence_items(root: &Path) -> Result<Vec<DiscoveredTestItem>> {
    let canonical_root = fs::canonicalize(root).map_err(|source| Error::io(root, source))?;
    let mut candidates = BTreeSet::new();
    for directory in ["boxdd/tests", "xtask/tests"] {
        collect_rust_files(&canonical_root.join(directory), false, &mut candidates)?;
    }
    let xtask_src = canonical_root.join("xtask/src");
    let mut source_candidates = BTreeSet::new();
    collect_rust_files(&xtask_src, true, &mut source_candidates)?;
    for candidate in source_candidates {
        if source_file_is_test_reachable(&xtask_src, &candidate)? {
            candidates.insert(candidate);
        }
    }

    let mut discovered = BTreeSet::new();
    for path in candidates {
        let relative = path.strip_prefix(&canonical_root).map_err(|_| {
            Error::message(format!("{} escapes the repository root", path.display()))
        })?;
        let file = relative
            .components()
            .filter_map(|component| component.as_os_str().to_str())
            .collect::<Vec<_>>()
            .join("/");
        let package = relative
            .components()
            .next()
            .and_then(|component| component.as_os_str().to_str())
            .unwrap_or_default()
            .to_owned();
        let source = fs::read_to_string(&path).map_err(|source| Error::io(&path, source))?;
        let syntax = syn::parse_file(&source).map_err(|error| {
            Error::message(format!("{}: invalid Rust syntax: {error}", path.display()))
        })?;
        if has_conditional_attributes(&syntax.attrs) {
            continue;
        }
        let mut items = Vec::new();
        collect_discovered_test_functions(
            &syntax.items,
            &mut Vec::new(),
            file.starts_with("xtask/src/"),
            &mut items,
        );
        for item in items {
            discovered.insert(DiscoveredTestItem {
                file: file.clone(),
                item,
                package: package.clone(),
                gate: "nextest".to_owned(),
            });
        }
    }
    Ok(discovered.into_iter().collect())
}

pub fn discover_indexed_test_evidence(
    root: &Path,
    rust_index: &RustIndex,
) -> Result<Vec<DiscoveredTestEvidence>> {
    discover_test_evidence_items(root)?
        .into_iter()
        .map(|test| {
            let index = index_test_evidence_for_gate(
                root,
                &test.file,
                &test.item,
                &test.package,
                &test.gate,
                rust_index,
            )?;
            Ok(DiscoveredTestEvidence { test, index })
        })
        .collect()
}

fn collect_rust_files(
    directory: &Path,
    recursive: bool,
    files: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    if !directory.is_dir() {
        return Ok(());
    }
    let entries = fs::read_dir(directory).map_err(|source| Error::io(directory, source))?;
    for entry in entries {
        let entry = entry.map_err(|source| Error::io(directory, source))?;
        let path = entry.path();
        if recursive && path.is_dir() {
            collect_rust_files(&path, true, files)?;
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            files.insert(path);
        }
    }
    Ok(())
}

fn collect_discovered_test_functions(
    items: &[Item],
    modules: &mut Vec<String>,
    allow_test_container: bool,
    discovered: &mut Vec<String>,
) {
    for item in items {
        match item {
            Item::Fn(function)
                if function.attrs.iter().any(is_exact_test_attribute)
                    && !function.block.stmts.is_empty()
                    && !function.attrs.iter().any(|attribute| {
                        attribute.path().is_ident("ignore")
                            || attribute.path().is_ident("cfg")
                            || attribute.path().is_ident("cfg_attr")
                    }) =>
            {
                let mut path = modules.clone();
                path.push(function.sig.ident.to_string());
                discovered.push(path.join("::"));
            }
            Item::Mod(module)
                if module_configuration_is_allowed(&module.attrs, allow_test_container) =>
            {
                if let Some((_, nested)) = &module.content {
                    modules.push(module.ident.to_string());
                    collect_discovered_test_functions(
                        nested,
                        modules,
                        allow_test_container,
                        discovered,
                    );
                    modules.pop();
                }
            }
            _ => {}
        }
    }
}

pub fn validate_test_evidence(root: &Path, file: &str, item: &str) -> Result<String> {
    let package = Path::new(file)
        .components()
        .next()
        .and_then(|component| component.as_os_str().to_str())
        .unwrap_or_default();
    validate_test_evidence_for_gate(root, file, item, package, "nextest")
}

pub fn validate_test_evidence_for_gate(
    root: &Path,
    file: &str,
    item: &str,
    package: &str,
    gate: &str,
) -> Result<String> {
    if gate != "nextest" {
        return Err(Error::message(format!(
            "{file}: evidence gate must be `nextest`"
        )));
    }
    let relative = Path::new(file);
    if relative.is_absolute()
        || !relative
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
    {
        return Err(Error::message(format!(
            "{file}: evidence path must be a safe repository-relative path"
        )));
    }
    let components = relative
        .components()
        .map(|component| component.as_os_str())
        .collect::<Vec<_>>();
    let Some(path_package) = components.first().and_then(|component| component.to_str()) else {
        return Err(Error::message(format!("{file}: invalid evidence package")));
    };
    let integration_test = components.len() == 3
        && matches!(path_package, "boxdd" | "xtask")
        && components[1] == "tests";
    let xtask_unit_module =
        components.len() >= 3 && path_package == "xtask" && components[1] == "src";
    if package != path_package
        || (!integration_test && !xtask_unit_module)
        || relative
            .extension()
            .and_then(|extension| extension.to_str())
            != Some("rs")
    {
        return Err(Error::message(format!(
            "{file}: evidence must belong to its declared package and be a boxdd/tests/*.rs, xtask/tests/*.rs, or reachable xtask/src module"
        )));
    }
    let canonical_root = fs::canonicalize(root).map_err(|source| Error::io(root, source))?;
    let path = fs::canonicalize(canonical_root.join(relative))
        .map_err(|source| Error::io(canonical_root.join(relative), source))?;
    if integration_test {
        let expected_tests = canonical_root.join(path_package).join("tests");
        let canonical_tests = fs::canonicalize(&expected_tests)
            .map_err(|source| Error::io(&expected_tests, source))?;
        if path.parent() != Some(canonical_tests.as_path()) || !path.starts_with(&canonical_root) {
            return Err(Error::message(format!(
                "{file}: evidence path escapes its package test directory"
            )));
        }
    } else {
        let source_root = canonical_root.join("xtask/src");
        let canonical_source_root =
            fs::canonicalize(&source_root).map_err(|source| Error::io(&source_root, source))?;
        if !path.starts_with(&canonical_source_root)
            || !source_file_is_test_reachable(&canonical_source_root, &path)?
        {
            return Err(Error::message(format!(
                "{file}: source evidence is not reachable from the xtask library test target"
            )));
        }
    }
    let source = fs::read_to_string(&path).map_err(|source| Error::io(&path, source))?;
    let syntax = syn::parse_file(&source).map_err(|error| {
        Error::message(format!("{}: invalid Rust syntax: {error}", path.display()))
    })?;
    let test_coordinate = RustIndexCoordinate::test();
    if has_conditional_attributes(&syntax.attrs)
        || !attributes_are_enabled(&syntax.attrs, &test_coordinate)
    {
        return Err(Error::message(format!(
            "{file}: evidence source cannot have crate-level conditional compilation"
        )));
    }
    let mut matches = Vec::new();
    collect_requested_functions(
        &syntax.items,
        &mut Vec::new(),
        item,
        &test_coordinate,
        xtask_unit_module,
        &mut matches,
    );
    if matches.len() != 1 {
        return Err(Error::message(format!(
            "{file}: expected exactly one function `{item}`, found {}",
            matches.len()
        )));
    }
    let matched = &matches[0];
    let function = matched.function;
    if !function.attrs.iter().any(is_exact_test_attribute) {
        return Err(Error::message(format!(
            "{file}: function `{item}` has no exact #[test] attribute"
        )));
    }
    if function.attrs.iter().any(|attribute| {
        attribute.path().is_ident("ignore")
            || attribute.path().is_ident("should_panic")
            || attribute.path().is_ident("cfg")
            || attribute.path().is_ident("cfg_attr")
    }) {
        return Err(Error::message(format!(
            "{file}: evidence test `{item}` cannot be ignored, should-panic, or conditionally compiled"
        )));
    }
    if function.block.stmts.is_empty() {
        return Err(Error::message(format!(
            "{file}: evidence test `{item}` has an empty body"
        )));
    }
    Ok(test_item_fingerprint(file, &matched.path, function))
}

pub fn index_test_evidence(
    root: &Path,
    file: &str,
    item: &str,
    rust_index: &RustIndex,
) -> Result<TestEvidenceIndex> {
    index_test_evidence_at_coordinate(root, file, item, rust_index, &RustIndexCoordinate::test())
}

pub fn index_test_evidence_at_coordinate(
    root: &Path,
    file: &str,
    item: &str,
    rust_index: &RustIndex,
    coordinate: &RustIndexCoordinate,
) -> Result<TestEvidenceIndex> {
    let package = Path::new(file)
        .components()
        .next()
        .and_then(|component| component.as_os_str().to_str())
        .unwrap_or_default();
    index_test_evidence_for_gate_at_coordinate(
        root, file, item, package, "nextest", rust_index, coordinate,
    )
}

pub fn index_test_evidence_for_gate(
    root: &Path,
    file: &str,
    item: &str,
    package: &str,
    gate: &str,
    rust_index: &RustIndex,
) -> Result<TestEvidenceIndex> {
    index_test_evidence_for_gate_at_coordinate(
        root,
        file,
        item,
        package,
        gate,
        rust_index,
        &RustIndexCoordinate::test(),
    )
}

pub fn index_test_evidence_for_gate_at_coordinate(
    root: &Path,
    file: &str,
    item: &str,
    package: &str,
    gate: &str,
    rust_index: &RustIndex,
    coordinate: &RustIndexCoordinate,
) -> Result<TestEvidenceIndex> {
    validate_test_evidence_for_gate(root, file, item, package, gate)?;
    let path = root.join(file);
    let source = fs::read_to_string(&path).map_err(|source| Error::io(&path, source))?;
    let syntax = syn::parse_file(&source).map_err(|error| {
        Error::message(format!("{}: invalid Rust syntax: {error}", path.display()))
    })?;
    let coordinate = coordinate.clone().with_cfg_flag("test", true);
    let mut matches = Vec::new();
    collect_requested_functions(
        &syntax.items,
        &mut Vec::new(),
        item,
        &coordinate,
        file.starts_with("xtask/src/"),
        &mut matches,
    );
    let [matched] = matches.as_slice() else {
        return Err(Error::message(format!(
            "{file}: evidence test `{item}` changed during indexing"
        )));
    };
    let model = EvidenceSourceModel::collect(&syntax.items, &coordinate);
    let calls = model.called_paths(&matched.path, rust_index);
    let fingerprint = model.evidence_fingerprint(file, &matched.path, &calls.local);
    let called_public_paths = calls.public;
    let dropped_public_types = calls.dropped;
    let implementation_reachable_symbols = rust_index
        .symbol_paths
        .iter()
        .filter(|(_, paths)| {
            paths.iter().any(|path| {
                called_public_paths.contains(path) || dropped_public_types.contains(path)
            })
        })
        .map(|(symbol, _)| symbol.clone())
        .collect();
    Ok(TestEvidenceIndex {
        fingerprint,
        called_public_paths,
        called_local_paths: calls.local,
        dropped_public_types,
        implementation_reachable_symbols,
        unresolved_calls: calls.gaps,
    })
}

struct TestItemMatch<'a> {
    path: String,
    function: &'a syn::ItemFn,
}

fn test_item_fingerprint(file: &str, item_path: &str, function: &syn::ItemFn) -> String {
    let normalized_file = Path::new(file)
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/");
    let normalized_tokens = function.to_token_stream().to_string();
    let mut hasher = blake3::Hasher::new();
    for component in [
        "boxdd-test-evidence-v1",
        normalized_file.as_str(),
        item_path,
        normalized_tokens.as_str(),
    ] {
        hasher.update(&(component.len() as u64).to_le_bytes());
        hasher.update(component.as_bytes());
    }
    format!("blake3:{}", hasher.finalize().to_hex())
}

fn collect_requested_functions<'a>(
    items: &'a [Item],
    modules: &mut Vec<String>,
    requested: &str,
    coordinate: &RustIndexCoordinate,
    allow_test_container: bool,
    matches: &mut Vec<TestItemMatch<'a>>,
) {
    for item in items {
        if !item_is_enabled(item, coordinate) {
            continue;
        }
        match item {
            Item::Fn(function) => {
                let mut path = modules.clone();
                path.push(function.sig.ident.to_string());
                let path = path.join("::");
                if function.sig.ident == requested || requested == path {
                    matches.push(TestItemMatch { path, function });
                }
            }
            Item::Mod(module) => {
                if !module_configuration_is_allowed(&module.attrs, allow_test_container) {
                    continue;
                }
                if let Some((_, nested)) = &module.content {
                    modules.push(module.ident.to_string());
                    collect_requested_functions(
                        nested,
                        modules,
                        requested,
                        coordinate,
                        allow_test_container,
                        matches,
                    );
                    modules.pop();
                }
            }
            _ => {}
        }
    }
}

fn is_exact_test_attribute(attribute: &Attribute) -> bool {
    matches!(&attribute.meta, Meta::Path(path) if path.is_ident("test"))
}

fn is_exact_cfg_test(attribute: &Attribute) -> bool {
    attribute.path().is_ident("cfg")
        && attribute
            .parse_args::<Meta>()
            .is_ok_and(|meta| matches!(meta, Meta::Path(path) if path.is_ident("test")))
}

fn has_conditional_attributes(attributes: &[Attribute]) -> bool {
    attributes
        .iter()
        .any(|attribute| attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr"))
}

fn module_configuration_is_allowed(attributes: &[Attribute], allow_test_container: bool) -> bool {
    attributes.iter().all(|attribute| {
        if !attribute.path().is_ident("cfg") && !attribute.path().is_ident("cfg_attr") {
            return true;
        }
        allow_test_container && is_exact_cfg_test(attribute)
    })
}

fn source_file_is_test_reachable(crate_src: &Path, target: &Path) -> Result<bool> {
    let lib_rs = crate_src.join("lib.rs");
    let source = fs::read_to_string(&lib_rs).map_err(|source| Error::io(&lib_rs, source))?;
    let syntax = syn::parse_file(&source).map_err(|error| {
        Error::message(format!(
            "{}: invalid Rust syntax: {error}",
            lib_rs.display()
        ))
    })?;
    source_module_reaches_file(
        crate_src,
        &lib_rs,
        &syntax,
        &[],
        target,
        &RustIndexCoordinate::test(),
        &mut BTreeSet::new(),
    )
}

fn source_module_reaches_file(
    crate_src: &Path,
    file: &Path,
    syntax: &File,
    modules: &[String],
    target: &Path,
    coordinate: &RustIndexCoordinate,
    visited: &mut BTreeSet<PathBuf>,
) -> Result<bool> {
    if has_conditional_attributes(&syntax.attrs)
        || !attributes_are_enabled(&syntax.attrs, coordinate)
    {
        return Ok(false);
    }
    let canonical = fs::canonicalize(file).map_err(|source| Error::io(file, source))?;
    if canonical == target {
        return Ok(true);
    }
    if !visited.insert(canonical) {
        return Ok(false);
    }
    source_items_reach_file(
        crate_src,
        &syntax.items,
        modules,
        target,
        coordinate,
        visited,
    )
}

fn source_items_reach_file(
    crate_src: &Path,
    items: &[Item],
    modules: &[String],
    target: &Path,
    coordinate: &RustIndexCoordinate,
    visited: &mut BTreeSet<PathBuf>,
) -> Result<bool> {
    for item in items {
        let Item::Mod(module) = item else {
            continue;
        };
        if !module_configuration_is_allowed(&module.attrs, true)
            || !attributes_are_enabled(&module.attrs, coordinate)
        {
            continue;
        }
        let mut child_modules = modules.to_vec();
        child_modules.push(module.ident.to_string());
        if let Some((_, nested)) = &module.content {
            if source_items_reach_file(
                crate_src,
                nested,
                &child_modules,
                target,
                coordinate,
                visited,
            )? {
                return Ok(true);
            }
            continue;
        }
        let child_file = resolve_module_file(crate_src, modules, &module.ident.to_string())?;
        let source =
            fs::read_to_string(&child_file).map_err(|source| Error::io(&child_file, source))?;
        let syntax = syn::parse_file(&source).map_err(|error| {
            Error::message(format!(
                "{}: invalid Rust syntax: {error}",
                child_file.display()
            ))
        })?;
        if source_module_reaches_file(
            crate_src,
            &child_file,
            &syntax,
            &child_modules,
            target,
            coordinate,
            visited,
        )? {
            return Ok(true);
        }
    }
    Ok(false)
}

#[derive(Default)]
struct EvidenceSourceModel<'a> {
    functions: BTreeMap<String, &'a syn::ItemFn>,
    imports: BTreeMap<String, BTreeMap<String, BTreeSet<String>>>,
    glob_imports: BTreeMap<String, BTreeSet<String>>,
    declared_macros: BTreeMap<String, BTreeSet<String>>,
    extern_aliases: BTreeMap<String, BTreeSet<String>>,
    modules: BTreeSet<String>,
    use_tokens: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Default)]
struct EvidenceCalls {
    public: BTreeSet<String>,
    local: BTreeSet<String>,
    dropped: BTreeSet<String>,
    gaps: BTreeSet<TestEvidenceGap>,
}

impl<'a> EvidenceSourceModel<'a> {
    fn collect(items: &'a [Item], coordinate: &RustIndexCoordinate) -> Self {
        let mut model = Self::default();
        model.modules.insert(String::new());
        model.collect_items(items, &mut Vec::new(), coordinate);
        model
    }

    fn collect_items(
        &mut self,
        items: &'a [Item],
        modules: &mut Vec<String>,
        coordinate: &RustIndexCoordinate,
    ) {
        for item in items {
            if !item_is_enabled(item, coordinate) {
                continue;
            }
            match item {
                Item::Fn(function) => {
                    self.functions
                        .insert(item_key(modules, &function.sig.ident.to_string()), function);
                }
                Item::Use(item) => {
                    let module = module_key(modules);
                    self.use_tokens
                        .entry(module.clone())
                        .or_default()
                        .insert(item.to_token_stream().to_string());
                    let mut entries = Vec::new();
                    flatten_use_tree(&item.tree, &mut Vec::new(), &mut entries);
                    for (alias, raw_path) in entries {
                        self.imports
                            .entry(module.clone())
                            .or_default()
                            .entry(alias)
                            .or_default()
                            .extend(normalize_import_paths(&module, &raw_path));
                    }
                    let mut globs = Vec::new();
                    flatten_use_globs(&item.tree, &mut Vec::new(), &mut globs);
                    for raw_path in globs {
                        self.glob_imports
                            .entry(module.clone())
                            .or_default()
                            .extend(normalize_import_paths(&module, &raw_path));
                    }
                }
                Item::Macro(item) if item.mac.path.is_ident("macro_rules") => {
                    if let Some(ident) = &item.ident {
                        self.declared_macros
                            .entry(module_key(modules))
                            .or_default()
                            .insert(ident.to_string());
                    }
                }
                Item::ExternCrate(item) => {
                    let alias = item
                        .rename
                        .as_ref()
                        .map_or(&item.ident, |(_, alias)| alias)
                        .to_string();
                    self.extern_aliases
                        .entry(module_key(modules))
                        .or_default()
                        .insert(alias);
                }
                Item::Mod(module) => {
                    let Some((_, nested)) = &module.content else {
                        continue;
                    };
                    modules.push(module.ident.to_string());
                    self.modules.insert(module_key(modules));
                    self.collect_items(nested, modules, coordinate);
                    modules.pop();
                }
                _ => {}
            }
        }
    }

    fn called_paths(&self, root: &str, rust_index: &RustIndex) -> EvidenceCalls {
        let mut called = EvidenceCalls::default();
        let mut visited = BTreeSet::new();
        let mut queue = VecDeque::from([root.to_owned()]);
        while let Some(function_path) = queue.pop_front() {
            if !visited.insert(function_path.clone()) {
                continue;
            }
            let Some(function) = self.functions.get(&function_path) else {
                continue;
            };
            let module = function_path
                .rsplit_once("::")
                .map_or("", |(module, _)| module);
            let mut visitor = EvidenceCallVisitor::new(self, rust_index, module, &function.sig);
            visitor.visit_block(&function.block);
            called.public.extend(visitor.called_public_paths);
            called.dropped.extend(visitor.dropped_public_types);
            called.gaps.extend(visitor.unresolved_calls);
            for local in visitor.called_local_functions {
                if local != root {
                    called.local.insert(local.clone());
                }
                queue.push_back(local);
            }
        }
        called
    }

    fn evidence_fingerprint(
        &self,
        file: &str,
        root: &str,
        called_local_paths: &BTreeSet<String>,
    ) -> String {
        let normalized_file = Path::new(file)
            .components()
            .filter_map(|component| component.as_os_str().to_str())
            .collect::<Vec<_>>()
            .join("/");
        let mut function_paths = called_local_paths.clone();
        function_paths.insert(root.to_owned());
        let mut components = vec![
            "boxdd-test-evidence-v2".to_owned(),
            normalized_file,
            root.to_owned(),
        ];
        let mut modules = BTreeSet::new();
        for function_path in function_paths {
            if let Some(function) = self.functions.get(&function_path) {
                components.push(format!("fn:{function_path}:{}", function.to_token_stream()));
            }
            modules.insert(
                function_path
                    .rsplit_once("::")
                    .map_or("", |(module, _)| module)
                    .to_owned(),
            );
        }
        for module in modules {
            if let Some(imports) = self.use_tokens.get(&module) {
                for import in imports {
                    components.push(format!("use:{module}:{import}"));
                }
            }
        }
        components.sort();
        let mut hasher = blake3::Hasher::new();
        for component in components {
            hasher.update(&(component.len() as u64).to_le_bytes());
            hasher.update(component.as_bytes());
        }
        format!("blake3:{}", hasher.finalize().to_hex())
    }

    fn path_candidates(&self, module: &str, segments: &[String]) -> BTreeSet<String> {
        let mut candidates = BTreeSet::new();
        let Some(first) = segments.first() else {
            return candidates;
        };
        if first == "crate" {
            if segments.len() > 1 {
                candidates.insert(segments[1..].join("::"));
            }
            return candidates;
        }
        if first == "self" {
            candidates.insert(join_path(module, &segments[1..]));
            return candidates;
        }
        if first == "super" {
            let mut base = split_path(module);
            let mut offset = 0;
            while segments
                .get(offset)
                .is_some_and(|segment| segment == "super")
            {
                if base.pop().is_none() {
                    return candidates;
                }
                offset += 1;
            }
            candidates.insert(join_segments(&base, &segments[offset..]));
            return candidates;
        }
        self.expand_scope_candidates(module, segments, &mut BTreeSet::new(), &mut candidates);
        if first == "boxdd" {
            candidates.insert(segments.join("::"));
        }
        candidates
    }

    fn expand_scope_candidates(
        &self,
        module: &str,
        segments: &[String],
        visited: &mut BTreeSet<(String, Vec<String>)>,
        candidates: &mut BTreeSet<String>,
    ) {
        if !visited.insert((module.to_owned(), segments.to_vec())) {
            return;
        }
        candidates.insert(join_path(module, segments));
        let Some(first) = segments.first() else {
            return;
        };
        if let Some(imports) = self
            .imports
            .get(module)
            .and_then(|imports| imports.get(first))
        {
            for imported in imports {
                candidates.insert(join_path(imported, &segments[1..]));
            }
        }
        if let Some(globs) = self.glob_imports.get(module) {
            for glob in globs {
                if glob == "boxdd" || glob.starts_with("boxdd::") {
                    candidates.insert(join_path(glob, segments));
                } else {
                    self.expand_scope_candidates(glob, segments, visited, candidates);
                }
            }
        }
    }

    fn builtin_macro_is_unshadowed(
        &self,
        rust_index: &RustIndex,
        module: &str,
        name: &str,
    ) -> bool {
        self.builtin_macro_is_unshadowed_inner(rust_index, module, name, &mut BTreeSet::new())
    }

    fn prelude_drop_is_unshadowed(&self, module: &str) -> bool {
        !self
            .functions
            .contains_key(&join_path(module, &["drop".to_owned()]))
            && !self
                .imports
                .get(module)
                .is_some_and(|imports| imports.contains_key("drop"))
            && self.glob_imports.get(module).is_none_or(BTreeSet::is_empty)
    }

    fn external_root_is_unshadowed(
        &self,
        rust_index: &RustIndex,
        module: &str,
        root: &str,
    ) -> bool {
        !self.modules.contains(root)
            && !self
                .imports
                .get(module)
                .is_some_and(|imports| imports.contains_key(root))
            && !self
                .extern_aliases
                .get(module)
                .is_some_and(|aliases| aliases.contains(root))
            && self.glob_imports.get(module).is_none_or(|globs| {
                globs.iter().all(|glob| {
                    (glob == "boxdd" || glob.starts_with("boxdd::"))
                        && !rust_index
                            .public_alias_paths
                            .contains_key(&format!("{glob}::{root}"))
                })
            })
    }

    fn builtin_macro_is_unshadowed_inner(
        &self,
        rust_index: &RustIndex,
        module: &str,
        name: &str,
        visited: &mut BTreeSet<String>,
    ) -> bool {
        if !visited.insert(module.to_owned())
            || self
                .declared_macros
                .get(module)
                .is_some_and(|macros| macros.contains(name))
            || self
                .imports
                .get(module)
                .is_some_and(|imports| imports.contains_key(name))
        {
            return false;
        }
        self.glob_imports.get(module).is_none_or(|globs| {
            globs.iter().all(|glob| {
                if glob == "boxdd" || glob.starts_with("boxdd::") {
                    return !rust_index
                        .public_alias_paths
                        .contains_key(&format!("{glob}::{name}"));
                }
                self.modules.contains(glob)
                    && self.builtin_macro_is_unshadowed_inner(rust_index, glob, name, visited)
            })
        })
    }

    fn resolve_local_function(&self, module: &str, segments: &[String]) -> Option<String> {
        let matches = self
            .path_candidates(module, segments)
            .into_iter()
            .filter(|candidate| self.functions.contains_key(candidate))
            .collect::<BTreeSet<_>>();
        (matches.len() == 1)
            .then(|| matches.into_iter().next())
            .flatten()
    }

    fn resolve_public_type(
        &self,
        rust_index: &RustIndex,
        module: &str,
        segments: &[String],
    ) -> Option<String> {
        let matches = self
            .path_candidates(module, segments)
            .into_iter()
            .filter_map(|candidate| canonical_public_type(rust_index, &candidate))
            .collect::<BTreeSet<_>>();
        (matches.len() == 1)
            .then(|| matches.into_iter().next())
            .flatten()
    }

    fn resolve_public_callable(
        &self,
        rust_index: &RustIndex,
        module: &str,
        segments: &[String],
    ) -> Option<String> {
        let matches = self
            .path_candidates(module, segments)
            .into_iter()
            .filter_map(|candidate| canonical_public_callable(rust_index, &candidate))
            .collect::<BTreeSet<_>>();
        (matches.len() == 1)
            .then(|| matches.into_iter().next())
            .flatten()
    }
}

fn canonical_public_type(index: &RustIndex, candidate: &str) -> Option<String> {
    let canonical = index
        .public_alias_paths
        .get(candidate)
        .map_or(candidate, String::as_str);
    index
        .public_type_paths
        .contains(canonical)
        .then(|| canonical.to_owned())
}

fn canonical_public_callable(index: &RustIndex, candidate: &str) -> Option<String> {
    let direct = index
        .public_alias_paths
        .get(candidate)
        .map_or(candidate, String::as_str);
    if index.public_safe_callable_paths.contains(direct) {
        return Some(direct.to_owned());
    }
    let (owner, method) = candidate.rsplit_once("::")?;
    let owner = canonical_public_type(index, owner)?;
    let method = format!("{owner}::{method}");
    index
        .public_safe_callable_paths
        .contains(&method)
        .then_some(method)
}

struct EvidenceCallVisitor<'model, 'syntax> {
    model: &'model EvidenceSourceModel<'syntax>,
    rust_index: &'model RustIndex,
    module: String,
    bindings: Vec<BTreeMap<String, Option<String>>>,
    owned_bindings: Vec<BTreeSet<String>>,
    called_public_paths: BTreeSet<String>,
    called_local_functions: BTreeSet<String>,
    dropped_public_types: BTreeSet<String>,
    unresolved_calls: BTreeSet<TestEvidenceGap>,
    witness_suppression: Vec<&'static str>,
    flow_stopped: bool,
    local_macro_scopes: Vec<LocalMacroScope>,
}

#[derive(Default)]
struct LocalMacroScope {
    names: BTreeSet<String>,
    has_glob_import: bool,
}

impl<'model, 'syntax> EvidenceCallVisitor<'model, 'syntax> {
    fn new(
        model: &'model EvidenceSourceModel<'syntax>,
        rust_index: &'model RustIndex,
        module: &str,
        signature: &Signature,
    ) -> Self {
        let mut visitor = Self {
            model,
            rust_index,
            module: module.to_owned(),
            bindings: vec![BTreeMap::new()],
            owned_bindings: vec![BTreeSet::new()],
            called_public_paths: BTreeSet::new(),
            called_local_functions: BTreeSet::new(),
            dropped_public_types: BTreeSet::new(),
            unresolved_calls: BTreeSet::new(),
            witness_suppression: Vec::new(),
            flow_stopped: false,
            local_macro_scopes: vec![LocalMacroScope::default()],
        };
        for argument in &signature.inputs {
            let FnArg::Typed(argument) = argument else {
                continue;
            };
            let Some(name) = simple_pattern_ident(&argument.pat) else {
                continue;
            };
            let owner = direct_value_type_path(&argument.ty).and_then(|segments| {
                visitor
                    .model
                    .resolve_public_type(visitor.rust_index, &visitor.module, &segments)
            });
            let owned = type_is_owned_value(&argument.ty) && owner.is_some();
            visitor.bindings[0].insert(name.clone(), owner);
            if owned {
                visitor.owned_bindings[0].insert(name);
            }
        }
        visitor
    }

    fn binding_owner(&self, name: &str) -> Option<String> {
        self.bindings
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))
            .cloned()
            .flatten()
    }

    fn binding_is_owned(&self, name: &str) -> bool {
        self.owned_bindings
            .iter()
            .rev()
            .zip(self.bindings.iter().rev())
            .find_map(|(owned, bindings)| bindings.contains_key(name).then(|| owned.contains(name)))
            .unwrap_or(false)
    }

    fn bind_owner(&mut self, name: String, owner: Option<String>, owned: bool) {
        self.bindings
            .last_mut()
            .expect("evidence visitor always has a scope")
            .insert(name.clone(), owner);
        let owned_bindings = self
            .owned_bindings
            .last_mut()
            .expect("evidence visitor always has an ownership scope");
        if owned {
            owned_bindings.insert(name);
        } else {
            owned_bindings.remove(&name);
        }
    }

    fn local_macro_is_shadowed(&self, name: &str) -> bool {
        self.local_macro_scopes
            .iter()
            .rev()
            .any(|scope| scope.has_glob_import || scope.names.contains(name))
    }

    fn builtin_macro_is_unshadowed(&self, name: &str) -> bool {
        !self.local_macro_is_shadowed(name)
            && self
                .model
                .builtin_macro_is_unshadowed(self.rust_index, &self.module, name)
    }

    fn with_suppressed_witness(&mut self, reason: &'static str, visit: impl FnOnce(&mut Self)) {
        let flow_stopped = self.flow_stopped;
        self.flow_stopped = false;
        self.witness_suppression.push(reason);
        visit(self);
        self.witness_suppression.pop();
        self.flow_stopped = flow_stopped;
    }

    fn record_public_call(&mut self, path: String, expression: String) {
        if let Some(reason) = self.witness_suppression.last() {
            self.unresolved_calls.insert(TestEvidenceGap {
                expression,
                reason: (*reason).to_owned(),
            });
        } else {
            self.called_public_paths.insert(path);
        }
    }

    fn record_local_call(&mut self, path: String, expression: String) {
        if let Some(reason) = self.witness_suppression.last() {
            self.unresolved_calls.insert(TestEvidenceGap {
                expression,
                reason: (*reason).to_owned(),
            });
        } else {
            self.called_local_functions.insert(path);
        }
    }

    fn register_block_item(&mut self, item: &Item) {
        match item {
            Item::Macro(item) if item.mac.path.is_ident("macro_rules") => {
                if let Some(ident) = &item.ident {
                    self.local_macro_scopes
                        .last_mut()
                        .expect("evidence visitor always has a macro scope")
                        .names
                        .insert(ident.to_string());
                }
            }
            Item::Use(item) => {
                let mut entries = Vec::new();
                flatten_use_tree(&item.tree, &mut Vec::new(), &mut entries);
                let aliases = entries
                    .into_iter()
                    .map(|(alias, _)| alias)
                    .collect::<BTreeSet<_>>();
                self.local_macro_scopes
                    .last_mut()
                    .expect("evidence visitor always has a macro scope")
                    .names
                    .extend(aliases.iter().cloned());
                self.bindings
                    .last_mut()
                    .expect("evidence visitor always has a binding scope")
                    .extend(aliases.into_iter().map(|alias| (alias, None)));
                let mut globs = Vec::new();
                flatten_use_globs(&item.tree, &mut Vec::new(), &mut globs);
                self.local_macro_scopes
                    .last_mut()
                    .expect("evidence visitor always has a macro scope")
                    .has_glob_import |= !globs.is_empty();
            }
            Item::Fn(item) => {
                self.bindings
                    .last_mut()
                    .expect("evidence visitor always has a binding scope")
                    .insert(item.sig.ident.to_string(), None);
            }
            Item::Mod(item) => {
                self.bindings
                    .last_mut()
                    .expect("evidence visitor always has a binding scope")
                    .insert(item.ident.to_string(), None);
            }
            Item::ExternCrate(item) => {
                let alias = item
                    .rename
                    .as_ref()
                    .map_or(&item.ident, |(_, alias)| alias)
                    .to_string();
                self.bindings
                    .last_mut()
                    .expect("evidence visitor always has a binding scope")
                    .insert(alias, None);
            }
            _ => {}
        }
    }

    fn expression_owner(&self, expression: &Expr) -> Option<String> {
        match expression {
            Expr::Path(path) if path.qself.is_none() && path.path.segments.len() == 1 => {
                self.binding_owner(&path.path.segments[0].ident.to_string())
            }
            Expr::Struct(structure) => self.model.resolve_public_type(
                self.rust_index,
                &self.module,
                &path_segments(&structure.path),
            ),
            Expr::Call(call) => {
                let Expr::Path(path) = call.func.as_ref() else {
                    return None;
                };
                let callable = self.model.resolve_public_callable(
                    self.rust_index,
                    &self.module,
                    &path_segments(&path.path),
                )?;
                self.rust_index
                    .callable_return_types
                    .get(&callable)
                    .cloned()
            }
            Expr::MethodCall(call) => {
                let owner = self.expression_owner(&call.receiver)?;
                if matches!(
                    call.method.to_string().as_str(),
                    "unwrap" | "expect" | "as_ref" | "as_mut" | "clone"
                ) {
                    return Some(owner);
                }
                self.rust_index
                    .callable_return_types
                    .get(&format!("{owner}::{}", call.method))
                    .cloned()
            }
            Expr::Group(group) => self.expression_owner(&group.expr),
            Expr::Paren(paren) => self.expression_owner(&paren.expr),
            Expr::Reference(reference) => self.expression_owner(&reference.expr),
            Expr::Try(expression) => self.expression_owner(&expression.expr),
            Expr::Await(expression) => self.expression_owner(&expression.base),
            Expr::Cast(cast) => direct_value_type_path(&cast.ty).and_then(|segments| {
                self.model
                    .resolve_public_type(self.rust_index, &self.module, &segments)
            }),
            _ => None,
        }
    }

    fn owned_expression_owner(&self, expression: &Expr) -> Option<String> {
        match expression {
            Expr::Path(path) if path.qself.is_none() => {
                if path.path.segments.len() == 1 {
                    let name = path.path.segments[0].ident.to_string();
                    if self
                        .bindings
                        .iter()
                        .rev()
                        .any(|scope| scope.contains_key(&name))
                    {
                        return self
                            .binding_is_owned(&name)
                            .then(|| self.binding_owner(&name))
                            .flatten();
                    }
                }
                self.model.resolve_public_type(
                    self.rust_index,
                    &self.module,
                    &path_segments(&path.path),
                )
            }
            Expr::Struct(_) | Expr::Call(_) | Expr::Cast(_) => self.expression_owner(expression),
            Expr::MethodCall(call)
                if matches!(
                    call.method.to_string().as_str(),
                    "unwrap" | "expect" | "clone"
                ) =>
            {
                self.expression_owner(expression)
            }
            Expr::Group(group) => self.owned_expression_owner(&group.expr),
            Expr::Paren(paren) => self.owned_expression_owner(&paren.expr),
            Expr::Try(expression) => self.owned_expression_owner(&expression.expr),
            Expr::Await(expression) => self.owned_expression_owner(&expression.base),
            Expr::Reference(_) | Expr::MethodCall(_) => None,
            _ => None,
        }
    }

    fn is_unshadowed_drop_path(&self, path: &syn::ExprPath) -> bool {
        if path.qself.is_some() {
            return false;
        }
        let segments = path_segments(&path.path);
        match segments.as_slice() {
            [drop] if drop == "drop" => {
                !self
                    .bindings
                    .iter()
                    .rev()
                    .any(|scope| scope.contains_key(drop))
                    && self.model.prelude_drop_is_unshadowed(&self.module)
            }
            [root, module, drop]
                if matches!(root.as_str(), "core" | "std") && module == "mem" && drop == "drop" =>
            {
                !self
                    .bindings
                    .iter()
                    .rev()
                    .any(|scope| scope.contains_key(root))
                    && self
                        .model
                        .external_root_is_unshadowed(self.rust_index, &self.module, root)
            }
            _ => false,
        }
    }

    fn record_drop(&mut self, owner: String, expression: String) {
        if let Some(reason) = self.witness_suppression.last() {
            self.unresolved_calls.insert(TestEvidenceGap {
                expression,
                reason: (*reason).to_owned(),
            });
        } else {
            self.dropped_public_types.insert(owner);
        }
    }
}

fn type_is_owned_value(ty: &Type) -> bool {
    match ty {
        Type::Group(group) => type_is_owned_value(&group.elem),
        Type::Paren(paren) => type_is_owned_value(&paren.elem),
        Type::Path(_) => true,
        _ => false,
    }
}

fn literal_bool(expression: &Expr) -> Option<bool> {
    let Expr::Lit(expression) = strip_expression_wrappers(expression) else {
        return None;
    };
    let syn::Lit::Bool(value) = &expression.lit else {
        return None;
    };
    Some(value.value)
}

fn constant_bool(expression: &Expr) -> Option<bool> {
    let expression = strip_expression_wrappers(expression);
    if let Some(value) = literal_bool(expression) {
        return Some(value);
    }
    match expression {
        Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Not(_)) => {
            constant_bool(&unary.expr).map(|value| !value)
        }
        Expr::Binary(binary) => match binary.op {
            syn::BinOp::And(_) => match constant_bool(&binary.left) {
                Some(false) => Some(false),
                Some(true) => constant_bool(&binary.right),
                None => None,
            },
            syn::BinOp::Or(_) => match constant_bool(&binary.left) {
                Some(true) => Some(true),
                Some(false) => constant_bool(&binary.right),
                None => None,
            },
            syn::BinOp::Eq(_) | syn::BinOp::Ne(_) => {
                let left = constant_scalar(&binary.left)?;
                let right = constant_scalar(&binary.right)?;
                let equal = left == right;
                Some(if matches!(binary.op, syn::BinOp::Eq(_)) {
                    equal
                } else {
                    !equal
                })
            }
            _ => None,
        },
        _ => None,
    }
}

fn bool_pattern_matches(pattern: &Pat, value: bool) -> Option<bool> {
    match pattern {
        Pat::Lit(pattern) => match &pattern.lit {
            syn::Lit::Bool(pattern) => Some(pattern.value == value),
            _ => Some(false),
        },
        Pat::Wild(_) => Some(true),
        Pat::Ident(pattern) => pattern.subpat.as_ref().map_or(Some(true), |(_, pattern)| {
            bool_pattern_matches(pattern, value)
        }),
        Pat::Or(pattern) => {
            let decisions = pattern
                .cases
                .iter()
                .map(|pattern| bool_pattern_matches(pattern, value))
                .collect::<Option<Vec<_>>>()?;
            Some(decisions.into_iter().any(|matches| matches))
        }
        Pat::Paren(pattern) => bool_pattern_matches(&pattern.pat, value),
        Pat::Reference(pattern) => bool_pattern_matches(&pattern.pat, value),
        _ => None,
    }
}

fn statement_definitely_stops(statement: &syn::Stmt) -> bool {
    match statement {
        syn::Stmt::Expr(expression, _) => expression_definitely_stops(expression),
        syn::Stmt::Macro(statement) => macro_is_known_diverging(&statement.mac),
        syn::Stmt::Local(_) | syn::Stmt::Item(_) => false,
    }
}

fn expression_definitely_stops(expression: &Expr) -> bool {
    match strip_expression_wrappers(expression) {
        Expr::Return(_) => true,
        Expr::Macro(expression) => macro_is_known_diverging(&expression.mac),
        Expr::Block(expression) => expression
            .block
            .stmts
            .last()
            .is_some_and(statement_definitely_stops),
        _ => false,
    }
}

fn macro_is_known_diverging(mac: &syn::Macro) -> bool {
    mac.path.segments.last().is_some_and(|segment| {
        matches!(
            segment.ident.to_string().as_str(),
            "panic" | "todo" | "unimplemented" | "unreachable"
        )
    })
}

fn constant_scalar(expression: &Expr) -> Option<String> {
    match strip_expression_wrappers(expression) {
        Expr::Lit(literal) => Some(literal.lit.to_token_stream().to_string()),
        _ => None,
    }
}

fn expression_may_return(expression: &Expr) -> bool {
    match strip_expression_wrappers(expression) {
        Expr::Return(_) => true,
        Expr::Block(block) => block_may_return(&block.block),
        Expr::If(expression) => {
            let then_returns = block_may_return(&expression.then_branch);
            let else_returns = expression
                .else_branch
                .as_ref()
                .is_some_and(|(_, expression)| expression_may_return(expression));
            match constant_bool(&expression.cond) {
                Some(true) => then_returns,
                Some(false) => else_returns,
                None => then_returns || else_returns,
            }
        }
        Expr::Match(expression) => expression
            .arms
            .iter()
            .any(|arm| expression_may_return(&arm.body)),
        Expr::While(expression) => block_may_return(&expression.body),
        Expr::ForLoop(expression) => block_may_return(&expression.body),
        Expr::Loop(expression) => block_may_return(&expression.body),
        Expr::Binary(expression)
            if matches!(expression.op, syn::BinOp::And(_) | syn::BinOp::Or(_)) =>
        {
            expression_may_return(&expression.left) || expression_may_return(&expression.right)
        }
        Expr::Closure(_) | Expr::Async(_) | Expr::Const(_) => false,
        expression => {
            let mut visitor = ReturnDetector { found: false };
            visitor.visit_expr(expression);
            visitor.found
        }
    }
}

fn block_may_return(block: &syn::Block) -> bool {
    block.stmts.iter().any(|statement| {
        let mut visitor = ReturnDetector { found: false };
        visitor.visit_stmt(statement);
        visitor.found
    })
}

struct ReturnDetector {
    found: bool,
}

impl<'ast> Visit<'ast> for ReturnDetector {
    fn visit_expr_return(&mut self, _expression: &'ast syn::ExprReturn) {
        self.found = true;
    }

    fn visit_expr_closure(&mut self, _expression: &'ast syn::ExprClosure) {}

    fn visit_expr_async(&mut self, _expression: &'ast syn::ExprAsync) {}

    fn visit_expr_const(&mut self, _expression: &'ast syn::ExprConst) {}

    fn visit_item(&mut self, _item: &'ast Item) {}
}

fn assertion_macro_expressions(mac: &syn::Macro, count: usize) -> Option<Vec<Expr>> {
    let parser = syn::punctuated::Punctuated::<Expr, syn::Token![,]>::parse_terminated;
    let expressions = parser.parse2(mac.tokens.clone()).ok()?;
    (expressions.len() >= count).then(|| expressions.into_iter().take(count).collect())
}

impl<'ast> Visit<'ast> for EvidenceCallVisitor<'_, '_> {
    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.bindings.push(BTreeMap::new());
        self.owned_bindings.push(BTreeSet::new());
        self.local_macro_scopes.push(LocalMacroScope::default());
        for statement in &block.stmts {
            if let syn::Stmt::Item(item) = statement {
                self.register_block_item(item);
            }
            if self.flow_stopped {
                self.with_suppressed_witness(
                    "call occurs after a control-flow path can return from the test",
                    |visitor| visitor.visit_stmt(statement),
                );
            } else {
                self.visit_stmt(statement);
            }
        }
        self.local_macro_scopes.pop();
        self.owned_bindings.pop();
        self.bindings.pop();
    }

    fn visit_expr_if(&mut self, expression: &'ast syn::ExprIf) {
        self.visit_expr(&expression.cond);
        let condition = constant_bool(&expression.cond);
        if condition != Some(false) {
            self.with_suppressed_witness("call is inside a conditional branch", |visitor| {
                visitor.visit_block(&expression.then_branch)
            });
        }
        if condition != Some(true)
            && let Some((_, alternate)) = &expression.else_branch
        {
            self.with_suppressed_witness("call is inside a conditional branch", |visitor| {
                visitor.visit_expr(alternate)
            });
        }
        let then_returns = block_may_return(&expression.then_branch);
        let else_returns = expression
            .else_branch
            .as_ref()
            .is_some_and(|(_, expression)| expression_may_return(expression));
        if match condition {
            Some(true) => then_returns,
            Some(false) => else_returns,
            None => then_returns || else_returns,
        } {
            self.flow_stopped = true;
        }
    }

    fn visit_expr_while(&mut self, expression: &'ast syn::ExprWhile) {
        self.visit_expr(&expression.cond);
        if constant_bool(&expression.cond) != Some(false) {
            self.with_suppressed_witness("call is inside a loop body", |visitor| {
                visitor.visit_block(&expression.body);
            });
            if block_may_return(&expression.body) {
                self.flow_stopped = true;
            }
        }
    }

    fn visit_expr_for_loop(&mut self, expression: &'ast syn::ExprForLoop) {
        self.visit_expr(&expression.expr);
        self.with_suppressed_witness("call is inside a loop body", |visitor| {
            visitor.visit_block(&expression.body);
        });
        if block_may_return(&expression.body) {
            self.flow_stopped = true;
        }
    }

    fn visit_expr_loop(&mut self, expression: &'ast syn::ExprLoop) {
        self.with_suppressed_witness("call is inside a loop body", |visitor| {
            visitor.visit_block(&expression.body);
        });
        if block_may_return(&expression.body) {
            self.flow_stopped = true;
        }
    }

    fn visit_expr_match(&mut self, expression: &'ast syn::ExprMatch) {
        self.visit_expr(&expression.expr);
        for arm in &expression.arms {
            if let Some((_, guard)) = &arm.guard {
                self.with_suppressed_witness("call is inside a match guard", |visitor| {
                    visitor.visit_expr(guard);
                });
            }
            self.with_suppressed_witness("call is inside a match arm", |visitor| {
                visitor.visit_expr(&arm.body);
            });
        }
        if expression
            .arms
            .iter()
            .any(|arm| expression_may_return(&arm.body))
        {
            self.flow_stopped = true;
        }
    }

    fn visit_expr_binary(&mut self, expression: &'ast syn::ExprBinary) {
        if !matches!(expression.op, syn::BinOp::And(_) | syn::BinOp::Or(_)) {
            syn::visit::visit_expr_binary(self, expression);
            return;
        }
        self.visit_expr(&expression.left);
        let left = constant_bool(&expression.left);
        let right_executes = match expression.op {
            syn::BinOp::And(_) => left != Some(false),
            syn::BinOp::Or(_) => left != Some(true),
            _ => unreachable!(),
        };
        if !right_executes {
            return;
        }
        let right_is_unconditional = match expression.op {
            syn::BinOp::And(_) => left == Some(true),
            syn::BinOp::Or(_) => left == Some(false),
            _ => unreachable!(),
        };
        if right_is_unconditional {
            self.visit_expr(&expression.right);
        } else {
            self.with_suppressed_witness("call is inside a short-circuit operand", |visitor| {
                visitor.visit_expr(&expression.right)
            });
        }
        if expression_may_return(&expression.right) {
            self.flow_stopped = true;
        }
    }

    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if let Expr::Path(path) = call.func.as_ref()
            && self.is_unshadowed_drop_path(path)
            && call.args.len() == 1
        {
            let argument = &call.args[0];
            let owner = self.owned_expression_owner(argument);
            self.visit_expr(argument);
            if let Some(owner) = owner {
                self.record_drop(owner, call.to_token_stream().to_string());
            } else {
                self.unresolved_calls.insert(TestEvidenceGap {
                    expression: call.to_token_stream().to_string(),
                    reason: "drop argument is not a proven owned public value".to_owned(),
                });
            }
            return;
        }
        if let Expr::Path(path) = call.func.as_ref()
            && path.qself.is_none()
        {
            let segments = path_segments(&path.path);
            if let Some(public_path) =
                self.model
                    .resolve_public_callable(self.rust_index, &self.module, &segments)
            {
                self.record_public_call(public_path, call.to_token_stream().to_string());
            } else if let Some(local) = self.model.resolve_local_function(&self.module, &segments) {
                self.record_local_call(local, call.to_token_stream().to_string());
            }
        }
        syn::visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        if let Some(owner) = self.expression_owner(&call.receiver) {
            let public_path = format!("{owner}::{}", call.method);
            if self
                .rust_index
                .public_safe_callable_paths
                .contains(&public_path)
            {
                self.record_public_call(public_path, call.to_token_stream().to_string());
            }
        }
        syn::visit::visit_expr_method_call(self, call);
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        if let Some(init) = &local.init {
            self.visit_expr(&init.expr);
            if let Some((_, diverge)) = &init.diverge {
                self.visit_expr(diverge);
            }
        }
        if !self.witness_suppression.is_empty() {
            return;
        }
        let explicit = pattern_type(&local.pat)
            .and_then(direct_value_type_path)
            .and_then(|segments| {
                self.model
                    .resolve_public_type(self.rust_index, &self.module, &segments)
            });
        let inferred_owned = local
            .init
            .as_ref()
            .and_then(|init| self.owned_expression_owner(&init.expr));
        let inferred = local
            .init
            .as_ref()
            .and_then(|init| self.expression_owner(&init.expr));
        let explicit_owned = pattern_type(&local.pat).is_some_and(type_is_owned_value);
        let owner = explicit.clone().or(inferred);
        let owner_is_owned = (explicit.is_some() && explicit_owned) || inferred_owned.is_some();
        let mut names = BTreeSet::new();
        collect_pattern_idents(&local.pat, &mut names);
        let simple = simple_local_pattern_ident(&local.pat);
        for name in names {
            self.bind_owner(
                name.clone(),
                (simple.as_deref() == Some(&name))
                    .then(|| owner.clone())
                    .flatten(),
                simple.as_deref() == Some(&name) && owner_is_owned,
            );
        }
    }

    fn visit_expr_assign(&mut self, assign: &'ast syn::ExprAssign) {
        self.visit_expr(&assign.left);
        self.visit_expr(&assign.right);
        if self.witness_suppression.is_empty()
            && let Expr::Path(path) = assign.left.as_ref()
            && path.qself.is_none()
            && path.path.segments.len() == 1
        {
            let owner = self.expression_owner(&assign.right);
            let owned = self.owned_expression_owner(&assign.right).is_some();
            let name = path.path.segments[0].ident.to_string();
            if let Some(index) = (0..self.bindings.len())
                .rev()
                .find(|index| self.bindings[*index].contains_key(&name))
            {
                self.bindings[index].insert(name.clone(), owner);
                if owned {
                    self.owned_bindings[index].insert(name);
                } else {
                    self.owned_bindings[index].remove(&name);
                }
            }
        }
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        let Some(name) =
            (mac.path.segments.len() == 1).then(|| mac.path.segments[0].ident.to_string())
        else {
            self.unresolved_calls.insert(TestEvidenceGap {
                expression: mac.path.to_token_stream().to_string(),
                reason: "opaque macro invocation".to_owned(),
            });
            return;
        };
        let expression_count = match name.as_str() {
            "assert" | "debug_assert" => Some(1),
            "assert_eq" | "assert_ne" | "debug_assert_eq" | "debug_assert_ne" => Some(2),
            "dbg" => Some(0),
            _ => None,
        };
        if matches!(
            name.as_str(),
            "panic" | "todo" | "unimplemented" | "unreachable"
        ) && self.builtin_macro_is_unshadowed(&name)
        {
            self.flow_stopped = true;
            return;
        }
        let Some(expression_count) = expression_count else {
            self.unresolved_calls.insert(TestEvidenceGap {
                expression: format!("{name}!(...)"),
                reason: "opaque macro invocation".to_owned(),
            });
            let known_non_control_macro = matches!(
                name.as_str(),
                "concat"
                    | "env"
                    | "eprint"
                    | "eprintln"
                    | "format"
                    | "format_args"
                    | "include_bytes"
                    | "include_str"
                    | "matches"
                    | "option_env"
                    | "print"
                    | "println"
                    | "stringify"
                    | "vec"
            ) && self.builtin_macro_is_unshadowed(&name);
            if !known_non_control_macro {
                self.flow_stopped = true;
            }
            return;
        };
        if !self.builtin_macro_is_unshadowed(&name) {
            self.unresolved_calls.insert(TestEvidenceGap {
                expression: format!("{name}!(...)"),
                reason: "assertion macro name is shadowed or ambiguous".to_owned(),
            });
            self.flow_stopped = true;
            return;
        }
        let parsed = if name == "dbg" {
            let parser = syn::punctuated::Punctuated::<Expr, syn::Token![,]>::parse_terminated;
            parser
                .parse2(mac.tokens.clone())
                .ok()
                .map(|expressions| expressions.into_iter().collect::<Vec<_>>())
        } else {
            assertion_macro_expressions(mac, expression_count)
        };
        let Some(expressions) = parsed else {
            self.unresolved_calls.insert(TestEvidenceGap {
                expression: format!("{name}!(...)"),
                reason: "standard assertion macro arguments could not be parsed".to_owned(),
            });
            return;
        };
        for expression in &expressions {
            self.visit_expr(expression);
        }
    }

    fn visit_item(&mut self, _item: &'ast Item) {}

    fn visit_expr_return(&mut self, expression: &'ast syn::ExprReturn) {
        if let Some(expression) = &expression.expr {
            self.visit_expr(expression);
        }
        self.flow_stopped = true;
    }

    fn visit_expr_closure(&mut self, _closure: &'ast syn::ExprClosure) {}

    fn visit_expr_async(&mut self, _async: &'ast syn::ExprAsync) {}

    fn visit_expr_const(&mut self, _constant: &'ast syn::ExprConst) {}
}

#[derive(Default)]
struct PatternIdentVisitor {
    idents: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for PatternIdentVisitor {
    fn visit_pat_ident(&mut self, pattern: &'ast syn::PatIdent) {
        self.idents.insert(pattern.ident.to_string());
        syn::visit::visit_pat_ident(self, pattern);
    }
}

fn collect_pattern_idents(pattern: &Pat, names: &mut BTreeSet<String>) {
    let mut visitor = PatternIdentVisitor::default();
    visitor.visit_pat(pattern);
    names.extend(visitor.idents);
}

fn simple_pattern_ident(pattern: &Pat) -> Option<String> {
    let Pat::Ident(pattern) = pattern else {
        return None;
    };
    pattern.subpat.is_none().then(|| pattern.ident.to_string())
}

fn flatten_use_tree(
    tree: &UseTree,
    prefix: &mut Vec<String>,
    entries: &mut Vec<(String, Vec<String>)>,
) {
    match tree {
        UseTree::Name(name) => {
            let ident = name.ident.to_string();
            if ident == "self" {
                if let Some(alias) = prefix.last() {
                    entries.push((alias.clone(), prefix.clone()));
                }
            } else {
                let mut path = prefix.clone();
                path.push(ident.clone());
                entries.push((ident, path));
            }
        }
        UseTree::Rename(rename) => {
            let mut path = prefix.clone();
            if rename.ident != "self" {
                path.push(rename.ident.to_string());
            }
            entries.push((rename.rename.to_string(), path));
        }
        UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            flatten_use_tree(&path.tree, prefix, entries);
            prefix.pop();
        }
        UseTree::Group(group) => {
            for item in &group.items {
                flatten_use_tree(item, prefix, entries);
            }
        }
        UseTree::Glob(_) => {}
    }
}

fn flatten_use_globs(tree: &UseTree, prefix: &mut Vec<String>, globs: &mut Vec<Vec<String>>) {
    match tree {
        UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            flatten_use_globs(&path.tree, prefix, globs);
            prefix.pop();
        }
        UseTree::Group(group) => {
            for item in &group.items {
                flatten_use_globs(item, prefix, globs);
            }
        }
        UseTree::Glob(_) => globs.push(prefix.clone()),
        UseTree::Name(_) | UseTree::Rename(_) => {}
    }
}

fn normalize_import_paths(module: &str, raw_path: &[String]) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    let Some(first) = raw_path.first() else {
        return paths;
    };
    if first == "crate" {
        if raw_path.len() > 1 {
            paths.insert(raw_path[1..].join("::"));
        }
        return paths;
    }
    if first == "self" {
        paths.insert(join_path(module, &raw_path[1..]));
        return paths;
    }
    if first == "super" {
        let mut base = split_path(module);
        let mut offset = 0;
        while raw_path
            .get(offset)
            .is_some_and(|segment| segment == "super")
        {
            if base.pop().is_none() {
                return BTreeSet::new();
            }
            offset += 1;
        }
        paths.insert(join_segments(&base, &raw_path[offset..]));
        return paths;
    }
    paths.insert(raw_path.join("::"));
    paths.insert(join_path(module, raw_path));
    paths
}

fn collect_owned_type_refs(ty: &Type, module: &str, owned: &mut BTreeSet<TypeRef>) {
    match ty {
        Type::Array(array) => collect_owned_type_refs(&array.elem, module, owned),
        Type::Group(group) => collect_owned_type_refs(&group.elem, module, owned),
        Type::Paren(paren) => collect_owned_type_refs(&paren.elem, module, owned),
        Type::Path(path) if path.qself.is_none() => {
            let segments = path_segments(&path.path);
            if !segments.is_empty() {
                owned.insert(TypeRef {
                    module: module.to_owned(),
                    segments: segments.clone(),
                });
            }
            let non_owning = segments.last().is_some_and(|ident| {
                matches!(
                    ident.as_str(),
                    "ManuallyDrop" | "MaybeUninit" | "PhantomData" | "Weak"
                )
            });
            if !non_owning {
                for segment in &path.path.segments {
                    if let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments {
                        for argument in &arguments.args {
                            if let syn::GenericArgument::Type(ty) = argument {
                                collect_owned_type_refs(ty, module, owned);
                            }
                        }
                    }
                }
            }
        }
        Type::Slice(slice) => collect_owned_type_refs(&slice.elem, module, owned),
        Type::Tuple(tuple) => {
            for element in &tuple.elems {
                collect_owned_type_refs(element, module, owned);
            }
        }
        Type::BareFn(_)
        | Type::ImplTrait(_)
        | Type::Infer(_)
        | Type::Macro(_)
        | Type::Never(_)
        | Type::Ptr(_)
        | Type::Reference(_)
        | Type::TraitObject(_)
        | Type::Verbatim(_) => {}
        _ => {}
    }
}

fn type_name(ty: &Type) -> Option<String> {
    let Type::Path(path) = ty else {
        return None;
    };
    path.path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
}

fn type_path_segments(ty: &Type) -> Option<Vec<String>> {
    let Type::Path(path) = ty else {
        return None;
    };
    path.qself.is_none().then(|| path_segments(&path.path))
}

fn direct_value_type_path(ty: &Type) -> Option<Vec<String>> {
    match ty {
        Type::Path(path) if path.qself.is_none() => Some(path_segments(&path.path)),
        Type::Group(group) => direct_value_type_path(&group.elem),
        Type::Paren(paren) => direct_value_type_path(&paren.elem),
        Type::Ptr(pointer) => direct_value_type_path(&pointer.elem),
        Type::Reference(reference) => direct_value_type_path(&reference.elem),
        _ => None,
    }
}

fn pattern_type(pattern: &Pat) -> Option<&Type> {
    match pattern {
        Pat::Type(pattern) => Some(&pattern.ty),
        _ => None,
    }
}

fn simple_local_pattern_ident(pattern: &Pat) -> Option<String> {
    match pattern {
        Pat::Type(pattern) => simple_pattern_ident(&pattern.pat),
        _ => simple_pattern_ident(pattern),
    }
}

fn resolve_module_file(crate_src: &Path, modules: &[String], module: &str) -> Result<PathBuf> {
    let base = modules
        .iter()
        .fold(crate_src.to_owned(), |path, module| path.join(module));
    let flat = base.join(format!("{module}.rs"));
    if flat.is_file() {
        return Ok(flat);
    }
    let nested = base.join(module).join("mod.rs");
    if nested.is_file() {
        return Ok(nested);
    }
    Err(Error::message(format!(
        "could not resolve module `{module}` below {}",
        base.display()
    )))
}

fn path_with(crate_name: &str, modules: &[String], item: &str) -> String {
    if modules.is_empty() {
        format!("{crate_name}::{item}")
    } else {
        format!("{crate_name}::{}::{item}", modules.join("::"))
    }
}

fn module_key(modules: &[String]) -> String {
    modules.join("::")
}

fn item_key(modules: &[String], item: &str) -> String {
    join_path(&module_key(modules), &[item.to_owned()])
}

fn split_path(path: &str) -> Vec<String> {
    if path.is_empty() {
        Vec::new()
    } else {
        path.split("::").map(str::to_owned).collect()
    }
}

fn join_path(base: &str, tail: &[String]) -> String {
    if base.is_empty() {
        tail.join("::")
    } else if tail.is_empty() {
        base.to_owned()
    } else {
        format!("{base}::{}", tail.join("::"))
    }
}

fn join_segments(base: &[String], tail: &[String]) -> String {
    match (base.is_empty(), tail.is_empty()) {
        (true, _) => tail.join("::"),
        (_, true) => base.join("::"),
        (false, false) => format!("{}::{}", base.join("::"), tail.join("::")),
    }
}

fn is_public(visibility: &Visibility) -> bool {
    matches!(visibility, Visibility::Public(_))
}

fn item_is_enabled(item: &Item, coordinate: &RustIndexCoordinate) -> bool {
    let attributes = match item {
        Item::Const(item) => &item.attrs,
        Item::Enum(item) => &item.attrs,
        Item::ExternCrate(item) => &item.attrs,
        Item::Fn(item) => &item.attrs,
        Item::Impl(item) => &item.attrs,
        Item::Mod(item) => &item.attrs,
        Item::Static(item) => &item.attrs,
        Item::Struct(item) => &item.attrs,
        Item::Trait(item) => &item.attrs,
        Item::Type(item) => &item.attrs,
        Item::Union(item) => &item.attrs,
        Item::Use(item) => &item.attrs,
        _ => return true,
    };
    attributes_are_enabled(attributes, coordinate)
}

fn attributes_are_enabled(attributes: &[Attribute], coordinate: &RustIndexCoordinate) -> bool {
    attributes
        .iter()
        .all(|attribute| attribute_cfg_decision(attribute, coordinate) == CfgDecision::Enabled)
}

fn attribute_cfg_decision(attribute: &Attribute, coordinate: &RustIndexCoordinate) -> CfgDecision {
    if attribute.path().is_ident("cfg") {
        return attribute
            .parse_args::<Meta>()
            .map_or(CfgDecision::Unknown, |meta| {
                evaluate_cfg_meta(&meta, coordinate)
            });
    }
    if attribute.path().is_ident("cfg_attr") {
        let parser = syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated;
        return attribute
            .parse_args_with(parser)
            .map_or(CfgDecision::Unknown, |metas| {
                evaluate_cfg_attr(metas.iter(), coordinate)
            });
    }
    CfgDecision::Enabled
}

fn evaluate_cfg_attr<'a>(
    mut metas: impl Iterator<Item = &'a Meta>,
    coordinate: &RustIndexCoordinate,
) -> CfgDecision {
    let Some(predicate) = metas.next() else {
        return CfgDecision::Unknown;
    };
    match evaluate_cfg_meta(predicate, coordinate) {
        CfgDecision::Disabled => CfgDecision::Enabled,
        CfgDecision::Unknown => CfgDecision::Unknown,
        CfgDecision::Enabled => {
            for meta in metas {
                let decision = nested_attribute_cfg_decision(meta, coordinate);
                if decision != CfgDecision::Enabled {
                    return decision;
                }
            }
            CfgDecision::Enabled
        }
    }
}

fn nested_attribute_cfg_decision(meta: &Meta, coordinate: &RustIndexCoordinate) -> CfgDecision {
    let Meta::List(list) = meta else {
        return CfgDecision::Enabled;
    };
    if list.path.is_ident("cfg") {
        return parse_meta_arguments(list)
            .and_then(|metas| (metas.len() == 1).then_some(metas))
            .map_or(CfgDecision::Unknown, |metas| {
                evaluate_cfg_meta(&metas[0], coordinate)
            });
    }
    if list.path.is_ident("cfg_attr") {
        return parse_meta_arguments(list).map_or(CfgDecision::Unknown, |metas| {
            evaluate_cfg_attr(metas.iter(), coordinate)
        });
    }
    CfgDecision::Enabled
}

fn evaluate_cfg_meta(meta: &Meta, coordinate: &RustIndexCoordinate) -> CfgDecision {
    match meta {
        Meta::Path(path) => {
            let Some(name) = path.get_ident().map(ToString::to_string) else {
                return CfgDecision::Unknown;
            };
            if !coordinate.known_flags.contains(&name) {
                CfgDecision::Unknown
            } else if coordinate.enabled_flags.contains(&name) {
                CfgDecision::Enabled
            } else {
                CfgDecision::Disabled
            }
        }
        Meta::NameValue(name_value) => {
            let Some(name) = name_value.path.get_ident().map(ToString::to_string) else {
                return CfgDecision::Unknown;
            };
            let Expr::Lit(expression) = &name_value.value else {
                return CfgDecision::Unknown;
            };
            let syn::Lit::Str(value) = &expression.lit else {
                return CfgDecision::Unknown;
            };
            let Some(values) = coordinate.cfg_values.get(&name) else {
                return CfgDecision::Unknown;
            };
            if values.contains(&value.value()) {
                CfgDecision::Enabled
            } else {
                CfgDecision::Disabled
            }
        }
        Meta::List(list) => {
            let Some(arguments) = parse_meta_arguments(list) else {
                return CfgDecision::Unknown;
            };
            if list.path.is_ident("all") {
                return combine_all(
                    arguments
                        .iter()
                        .map(|argument| evaluate_cfg_meta(argument, coordinate)),
                );
            }
            if list.path.is_ident("any") {
                return combine_any(
                    arguments
                        .iter()
                        .map(|argument| evaluate_cfg_meta(argument, coordinate)),
                );
            }
            if list.path.is_ident("not") && arguments.len() == 1 {
                return match evaluate_cfg_meta(&arguments[0], coordinate) {
                    CfgDecision::Enabled => CfgDecision::Disabled,
                    CfgDecision::Disabled => CfgDecision::Enabled,
                    CfgDecision::Unknown => CfgDecision::Unknown,
                };
            }
            CfgDecision::Unknown
        }
    }
}

fn parse_meta_arguments(list: &syn::MetaList) -> Option<Vec<Meta>> {
    let parser = syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated;
    parser
        .parse2(list.tokens.clone())
        .ok()
        .map(|arguments| arguments.into_iter().collect())
}

fn combine_all(decisions: impl IntoIterator<Item = CfgDecision>) -> CfgDecision {
    let mut unknown = false;
    for decision in decisions {
        match decision {
            CfgDecision::Disabled => return CfgDecision::Disabled,
            CfgDecision::Unknown => unknown = true,
            CfgDecision::Enabled => {}
        }
    }
    if unknown {
        CfgDecision::Unknown
    } else {
        CfgDecision::Enabled
    }
}

fn combine_any(decisions: impl IntoIterator<Item = CfgDecision>) -> CfgDecision {
    let mut unknown = false;
    for decision in decisions {
        match decision {
            CfgDecision::Enabled => return CfgDecision::Enabled,
            CfgDecision::Unknown => unknown = true,
            CfgDecision::Disabled => {}
        }
    }
    if unknown {
        CfgDecision::Unknown
    } else {
        CfgDecision::Disabled
    }
}

fn standard_trait_identity(path: &str) -> Option<ProvenTrait> {
    match path {
        "core::ops::Drop" | "std::ops::Drop" => Some(ProvenTrait::Drop),
        "core::default::Default"
        | "std::default::Default"
        | "core::convert::From"
        | "std::convert::From"
        | "core::convert::Into"
        | "std::convert::Into"
        | "core::convert::TryFrom"
        | "std::convert::TryFrom"
        | "core::convert::TryInto"
        | "std::convert::TryInto"
        | "core::convert::AsRef"
        | "std::convert::AsRef" => Some(ProvenTrait::Public),
        _ => None,
    }
}

fn prelude_trait_identity(name: &str) -> Option<ProvenTrait> {
    match name {
        "Drop" => Some(ProvenTrait::Drop),
        "Default" | "From" | "Into" | "TryFrom" | "TryInto" | "AsRef" => Some(ProvenTrait::Public),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    fn fixture_root(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("test clock")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "boxdd-rust-index-{name}-{}-{nonce}",
            std::process::id()
        ))
    }

    fn index_fixture_with_manifest_and_coordinate(
        name: &str,
        source: &str,
        manifest: &str,
        coordinate: &RustIndexCoordinate,
    ) -> RustIndex {
        let root = fixture_root(name);
        fs::create_dir_all(&root).expect("fixture root");
        fs::write(root.join("Cargo.toml"), manifest).expect("fixture manifest");
        let lib = root.join("lib.rs");
        fs::write(&lib, source).expect("fixture source");
        let index =
            index_crate_for_coordinate(&lib, "fixture", coordinate).expect("fixture should index");
        fs::remove_dir_all(root).expect("fixture cleanup");
        index
    }

    fn index_fixture_with_manifest(name: &str, source: &str, manifest: &str) -> RustIndex {
        index_fixture_with_manifest_and_coordinate(
            name,
            source,
            manifest,
            &RustIndexCoordinate::source_single(),
        )
    }

    fn index_fixture(name: &str, source: &str) -> RustIndex {
        index_fixture_with_manifest(
            name,
            source,
            "[package]\nname = \"rust-index-fixture\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[dependencies]\nboxdd-sys = \"0\"\n",
        )
    }

    fn index_fixture_without_boxdd_sys(name: &str, source: &str) -> RustIndex {
        index_fixture_with_manifest(
            name,
            source,
            "[package]\nname = \"rust-index-fixture\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
        )
    }

    fn write_local_dependency(root: &Path, directory: &str, package: &str) {
        let dependency_root = root.join(directory);
        fs::create_dir_all(&dependency_root).expect("dependency root");
        fs::write(
            dependency_root.join("Cargo.toml"),
            format!(
                "[package]\nname = \"{package}\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[lib]\npath = \"lib.rs\"\n"
            ),
        )
        .expect("dependency manifest");
        fs::write(
            dependency_root.join("lib.rs"),
            "pub mod ffi { #[allow(non_snake_case)] pub unsafe fn b2World_Step() {} }\n",
        )
        .expect("dependency source");
    }

    #[test]
    fn only_reachable_public_items_cover_c_symbols() {
        let index = index_fixture(
            "reachability",
            r#"
                pub struct World;
                impl World {
                    pub fn step(&self) { helper(); }
                }
                fn helper() { unsafe { boxdd_sys::ffi::b2World_Step() }; }
                fn private_only() { unsafe { boxdd_sys::ffi::b2CommentCannotCover() }; }
                const TEXT: &str = "b2StringCannotCover";
            "#,
        );
        assert!(index.path_reaches_symbol("fixture::World::step", "b2World_Step"));
        assert!(
            index
                .paths_for_symbol("b2CommentCannotCover")
                .next()
                .is_none()
        );
        assert!(
            index
                .paths_for_symbol("b2StringCannotCover")
                .next()
                .is_none()
        );
    }

    #[test]
    fn crate_level_cfg_disables_the_entire_source_index() {
        let index = index_fixture(
            "crate-cfg-disabled",
            r#"
                #![cfg(any())]
                pub fn fake() { unsafe { boxdd_sys::ffi::b2CrateCfgDisabled() }; }
            "#,
        );
        assert!(!index.contains_public_path("fixture::fake"));
        assert!(
            index
                .paths_for_symbol("b2CrateCfgDisabled")
                .next()
                .is_none()
        );
    }

    #[test]
    fn item_module_and_impl_cfg_are_applied_fail_closed() {
        let index = index_fixture(
            "nested-cfg-disabled",
            r#"
                #[cfg(any())]
                pub fn disabled_item() {
                    unsafe { boxdd_sys::ffi::b2ItemCfgDisabled() };
                }

                #[cfg(any())]
                pub mod disabled_module {
                    pub fn fake() {
                        unsafe { boxdd_sys::ffi::b2ModuleCfgDisabled() };
                    }
                }

                pub struct Public;
                #[cfg(any())]
                impl Public {
                    pub fn disabled_impl() {
                        unsafe { boxdd_sys::ffi::b2ImplCfgDisabled() };
                    }
                }

                #[cfg_attr(all(), cfg(any()))]
                pub fn disabled_cfg_attr() {
                    unsafe { boxdd_sys::ffi::b2CfgAttrDisabled() };
                }
            "#,
        );
        for symbol in [
            "b2ItemCfgDisabled",
            "b2ModuleCfgDisabled",
            "b2ImplCfgDisabled",
            "b2CfgAttrDisabled",
        ] {
            assert!(index.paths_for_symbol(symbol).next().is_none(), "{symbol}");
        }
        assert!(!index.contains_public_path("fixture::disabled_item"));
        assert!(!index.contains_public_path("fixture::disabled_module"));
        assert!(!index.contains_public_path("fixture::Public::disabled_impl"));
        assert!(!index.contains_public_path("fixture::disabled_cfg_attr"));
    }

    #[test]
    fn explicit_coordinate_controls_feature_gated_source() {
        let source = r#"
            #[cfg(feature = "double")]
            pub fn double_only() { unsafe { boxdd_sys::ffi::b2DoubleOnly() }; }
        "#;
        let manifest = "[package]\nname = \"rust-index-fixture\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[dependencies]\nboxdd-sys = \"0\"\n";
        let single = index_fixture_with_manifest("single-coordinate", source, manifest);
        assert!(!single.contains_public_path("fixture::double_only"));

        let double = index_fixture_with_manifest_and_coordinate(
            "double-coordinate",
            source,
            manifest,
            &RustIndexCoordinate::source_single().with_cfg_value("feature", "double"),
        );
        assert!(double.path_reaches_symbol("fixture::double_only", "b2DoubleOnly"));
    }

    #[test]
    fn route_indexes_require_explicit_coordinates() {
        let root = fixture_root("route-coordinates");
        let crate_root = root.join("boxdd");
        let source_root = crate_root.join("src");
        fs::create_dir_all(&source_root).expect("fixture source root");
        fs::write(
            crate_root.join("Cargo.toml"),
            "[package]\nname = \"boxdd\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[dependencies]\nboxdd-sys = \"0\"\n",
        )
        .expect("fixture manifest");
        fs::write(
            source_root.join("lib.rs"),
            r#"
                pub fn single() { unsafe { boxdd_sys::ffi::b2Single() }; }
                #[cfg(feature = "double-precision")]
                pub fn double() { unsafe { boxdd_sys::ffi::b2Double() }; }
            "#,
        )
        .expect("fixture source");
        let mut coordinates = BTreeMap::new();
        coordinates.insert(
            ("single".to_owned(), "source".to_owned()),
            RustIndexCoordinate::source_single(),
        );
        coordinates.insert(
            ("single".to_owned(), "mirror".to_owned()),
            RustIndexCoordinate::source_single(),
        );
        coordinates.insert(
            ("double".to_owned(), "source".to_owned()),
            RustIndexCoordinate::source_single().with_cfg_value("feature", "double-precision"),
        );
        let indexes = index_boxdd_routes(&root, &coordinates).expect("route indexes");
        let single_key = ("single".to_owned(), "source".to_owned());
        let mirror_key = ("single".to_owned(), "mirror".to_owned());
        let double_key = ("double".to_owned(), "source".to_owned());
        assert!(indexes[&single_key].path_reaches_symbol("boxdd::single", "b2Single"));
        assert!(indexes[&mirror_key].path_reaches_symbol("boxdd::single", "b2Single"));
        assert!(!indexes[&single_key].contains_public_path("boxdd::double"));
        assert!(indexes[&double_key].path_reaches_symbol("boxdd::double", "b2Double"));
        assert_eq!(indexes.len(), coordinates.len());
        fs::remove_dir_all(root).expect("fixture cleanup");
    }

    #[test]
    fn wasm_coordinates_replace_host_target_cfg_values() {
        let source = r#"
            #[cfg(all(target_arch = "wasm32", target_os = "unknown", target_family = "wasm"))]
            pub fn browser() { unsafe { boxdd_sys::ffi::b2Browser() }; }
            #[cfg(all(target_arch = "wasm32", target_os = "wasi", target_env = "p1"))]
            pub fn wasi() { unsafe { boxdd_sys::ffi::b2Wasi() }; }
        "#;
        let manifest = "[package]\nname = \"rust-index-fixture\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[dependencies]\nboxdd-sys = \"0\"\n";
        let browser = index_fixture_with_manifest_and_coordinate(
            "wasm-browser-coordinate",
            source,
            manifest,
            &RustIndexCoordinate::wasm32_unknown_unknown(),
        );
        let wasi = index_fixture_with_manifest_and_coordinate(
            "wasip1-coordinate",
            source,
            manifest,
            &RustIndexCoordinate::wasm32_wasip1(),
        );
        assert!(browser.path_reaches_symbol("fixture::browser", "b2Browser"));
        assert!(!browser.contains_public_path("fixture::wasi"));
        assert!(wasi.path_reaches_symbol("fixture::wasi", "b2Wasi"));
        assert!(!wasi.contains_public_path("fixture::browser"));
    }

    #[test]
    fn default_source_index_does_not_assume_a_build_profile() {
        let source = r#"
            #[cfg(debug_assertions)]
            pub fn debug_only() { unsafe { boxdd_sys::ffi::b2DebugOnly() }; }
            #[cfg(not(debug_assertions))]
            pub fn release_only() { unsafe { boxdd_sys::ffi::b2ReleaseOnly() }; }
        "#;
        let manifest = "[package]\nname = \"rust-index-fixture\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[dependencies]\nboxdd-sys = \"0\"\n";
        let always = index_fixture_with_manifest("profile-independent", source, manifest);
        assert!(!always.contains_public_path("fixture::debug_only"));
        assert!(!always.contains_public_path("fixture::release_only"));

        let debug = index_fixture_with_manifest_and_coordinate(
            "debug-coordinate",
            source,
            manifest,
            &RustIndexCoordinate::source_single().with_cfg_flag("debug_assertions", true),
        );
        assert!(debug.path_reaches_symbol("fixture::debug_only", "b2DebugOnly"));
        assert!(!debug.contains_public_path("fixture::release_only"));

        let release = index_fixture_with_manifest_and_coordinate(
            "release-coordinate",
            source,
            manifest,
            &RustIndexCoordinate::source_single().with_cfg_flag("debug_assertions", false),
        );
        assert!(release.path_reaches_symbol("fixture::release_only", "b2ReleaseOnly"));
        assert!(!release.contains_public_path("fixture::debug_only"));
    }

    #[test]
    fn local_b2_named_function_does_not_fake_ffi_coverage() {
        let index = index_fixture(
            "local-b2-function",
            r#"
                fn b2World_Step() {}
                pub fn safe_api() { b2World_Step(); }
            "#,
        );
        assert!(index.paths_for_symbol("b2World_Step").next().is_none());
    }

    #[test]
    fn crate_qualified_b2_named_function_does_not_fake_ffi_coverage() {
        let index = index_fixture(
            "crate-qualified-b2-function",
            r#"
                fn b2World_Step() {}
                pub fn safe_api() { crate::b2World_Step(); }
            "#,
        );
        assert!(index.paths_for_symbol("b2World_Step").next().is_none());
    }

    #[test]
    fn ffi_paths_require_a_declared_boxdd_sys_dependency() {
        let index = index_fixture_without_boxdd_sys(
            "undeclared-boxdd-sys",
            r#"
                pub fn safe_api() { unsafe { boxdd_sys::ffi::b2World_Step() }; }
                pub fn forged_type(raw: boxdd_sys::ffi::b2Vec2) -> f32 { raw.x }
            "#,
        );
        assert!(index.paths_for_symbol("b2World_Step").next().is_none());
        assert!(!index.path_has_ffi_type_witness("fixture::forged_type", "boxdd_sys::ffi::b2Vec2"));
        assert!(!index.path_has_ffi_field_witness(
            "fixture::forged_type",
            "boxdd_sys::ffi::b2Vec2",
            "x"
        ));
    }

    #[test]
    fn inactive_target_dependency_does_not_prove_ffi_provenance() {
        let index = index_fixture_with_manifest(
            "inactive-target-boxdd-sys",
            r#"
                pub fn safe_api() { unsafe { boxdd_sys::ffi::b2World_Step() }; }
            "#,
            "[package]\nname = \"rust-index-fixture\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[target.'cfg(any())'.dependencies]\nboxdd-sys = \"0\"\n",
        );
        assert!(index.paths_for_symbol("b2World_Step").next().is_none());
    }

    #[test]
    fn workspace_dependency_resolves_the_actual_package_identity() {
        let root = fixture_root("workspace-boxdd-sys");
        fs::create_dir_all(&root).expect("fixture root");
        write_local_dependency(&root, "native", "boxdd-sys");
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"rust-index-fixture\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[lib]\npath = \"lib.rs\"\n\n[workspace]\nmembers = [\"native\"]\n\n[workspace.dependencies]\nboxdd-sys = { path = \"native\" }\n\n[dependencies]\nboxdd-sys = { workspace = true }\n",
        )
        .expect("fixture manifest");
        let lib = root.join("lib.rs");
        fs::write(
            &lib,
            "pub fn safe_api() { unsafe { boxdd_sys::ffi::b2World_Step() }; }\n",
        )
        .expect("fixture source");
        let index = index_crate(&lib, "fixture").expect("fixture should index");
        assert!(index.path_reaches_symbol("fixture::safe_api", "b2World_Step"));
        fs::remove_dir_all(root).expect("fixture cleanup");
    }

    #[test]
    fn workspace_dependency_alias_cannot_fake_boxdd_sys_provenance() {
        let root = fixture_root("workspace-fake-boxdd-sys");
        fs::create_dir_all(&root).expect("fixture root");
        write_local_dependency(&root, "fake", "fake");
        let manifest = root.join("Cargo.toml");
        fs::write(
            &manifest,
            "[package]\nname = \"rust-index-fixture\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[lib]\npath = \"lib.rs\"\n\n[workspace]\nmembers = [\"fake\"]\n\n[workspace.dependencies]\nboxdd-sys = { package = \"fake\", path = \"fake\" }\n\n[dependencies]\nboxdd-sys = { workspace = true }\n",
        )
        .expect("fixture manifest");
        let lib = root.join("lib.rs");
        fs::write(
            &lib,
            "pub fn safe_api() { unsafe { boxdd_sys::ffi::b2World_Step() }; }\n",
        )
        .expect("fixture source");

        let output = Command::new(env!("CARGO"))
            .args(["check", "--quiet", "--manifest-path"])
            .arg(&manifest)
            .env("CARGO_TARGET_DIR", root.join("target"))
            .output()
            .expect("fixture cargo check");
        assert!(
            output.status.success(),
            "the adversarial fixture must compile: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let index = index_crate(&lib, "fixture").expect("fixture should index");
        assert!(index.paths_for_symbol("b2World_Step").next().is_none());
        fs::remove_dir_all(root).expect("fixture cleanup");
    }

    #[test]
    fn local_boxdd_sys_module_does_not_fake_ffi_coverage() {
        let index = index_fixture(
            "local-boxdd-sys-module",
            r#"
                mod boxdd_sys {
                    pub mod ffi {
                        pub fn b2World_Step() {}
                    }
                }
                pub fn relative_api() { boxdd_sys::ffi::b2World_Step(); }
                pub fn crate_api() { crate::boxdd_sys::ffi::b2World_Step(); }
            "#,
        );
        assert!(index.paths_for_symbol("b2World_Step").next().is_none());
    }

    #[test]
    fn extern_crate_self_alias_does_not_fake_ffi_coverage() {
        let index = index_fixture(
            "extern-self-boxdd-sys",
            r#"
                extern crate self as boxdd_sys;
                pub mod ffi {
                    pub fn b2World_Step() {}
                }
                pub fn safe_api() { boxdd_sys::ffi::b2World_Step(); }
            "#,
        );
        assert!(index.paths_for_symbol("b2World_Step").next().is_none());
    }

    #[test]
    fn local_crate_alias_does_not_fake_ffi_coverage() {
        let index = index_fixture(
            "local-boxdd-sys-use-alias",
            r#"
                mod fake {
                    pub mod ffi {
                        pub fn b2World_Step() {}
                    }
                }
                use crate::fake as boxdd_sys;
                pub fn aliased_crate_api() { boxdd_sys::ffi::b2World_Step(); }
            "#,
        );
        assert!(index.paths_for_symbol("b2World_Step").next().is_none());
    }

    #[test]
    fn local_ffi_import_does_not_fake_ffi_coverage() {
        let index = index_fixture(
            "local-ffi-use-alias",
            r#"
                mod fake {
                    pub mod ffi {
                        pub fn b2World_Step() {}
                    }
                }
                use crate::fake::ffi;
                pub fn imported_ffi_api() { ffi::b2World_Step(); }
            "#,
        );
        assert!(index.paths_for_symbol("b2World_Step").next().is_none());
    }

    #[test]
    fn renamed_boxdd_ffi_import_retains_provenance() {
        let index = index_fixture(
            "renamed-boxdd-ffi-import",
            r#"
                use boxdd_sys::ffi::b2World_Step as native_step;
                pub fn safe_api() { unsafe { native_step() }; }
            "#,
        );
        assert!(index.path_reaches_symbol("fixture::safe_api", "b2World_Step"));
    }

    #[test]
    fn public_method_on_private_type_is_not_a_public_api() {
        let index = index_fixture(
            "private-owner-public-method",
            r#"
                struct Hidden;
                impl Hidden {
                    pub fn exposed() { unsafe { boxdd_sys::ffi::b2Hidden() }; }
                }
            "#,
        );
        assert!(!index.contains_public_path("fixture::Hidden::exposed"));
        assert!(index.paths_for_symbol("b2Hidden").next().is_none());
    }

    #[test]
    fn child_impl_before_parent_type_uses_the_exact_module_namespace() {
        let index = index_fixture(
            "child-impl-before-parent-type",
            r#"
                mod unrelated {
                    pub struct Shared;
                }

                mod implementation {
                    mod methods {
                        use super::*;

                        impl Shared {
                            pub fn call() {
                                unsafe { boxdd_sys::ffi::b2ExactShared() };
                            }
                        }
                    }

                    pub struct Shared;
                }

                pub use implementation::Shared;
            "#,
        );
        assert!(index.path_reaches_symbol("fixture::Shared::call", "b2ExactShared"));
    }

    #[test]
    fn ambiguous_glob_imported_impl_owner_fails_closed() {
        let index = index_fixture(
            "ambiguous-glob-impl-owner",
            r#"
                mod left {
                    pub struct Shared;
                }
                mod right {
                    pub struct Shared;
                }
                mod methods {
                    use crate::left::*;
                    use crate::right::*;

                    impl Shared {
                        pub fn forged() {
                            unsafe { boxdd_sys::ffi::b2AmbiguousShared() };
                        }
                    }
                }

                pub use left::Shared;
            "#,
        );
        assert!(!index.contains_public_path("fixture::Shared::forged"));
        assert!(index.paths_for_symbol("b2AmbiguousShared").next().is_none());
    }

    #[test]
    fn root_exports_resolve_to_their_exact_source_items() {
        let index = index_fixture(
            "exact-root-reexports",
            r#"
                mod internals {
                    pub struct Exposed;
                    impl Exposed {
                        pub fn call() { unsafe { boxdd_sys::ffi::b2Exposed() }; }
                    }
                    pub fn helper() { unsafe { boxdd_sys::ffi::b2Helper() }; }
                }
                pub use internals::{Exposed, helper};
            "#,
        );
        assert!(index.path_reaches_symbol("fixture::Exposed::call", "b2Exposed"));
        assert!(index.path_reaches_symbol("fixture::helper", "b2Helper"));
    }

    #[test]
    fn transitive_public_reexports_resolve_to_the_unique_declaration() {
        let index = index_fixture(
            "transitive-root-reexports",
            r#"
                mod implementation {
                    pub struct Actual {
                        pub exposed: i32,
                        hidden: i32,
                    }
                    impl Actual {
                        pub fn call() { unsafe { boxdd_sys::ffi::b2TransitiveType() }; }
                    }
                    pub fn actual_helper() {
                        unsafe { boxdd_sys::ffi::b2TransitiveFunction() };
                    }
                }
                mod facade {
                    pub use crate::implementation::Actual as Intermediate;
                    pub use crate::implementation::actual_helper as intermediate_helper;
                }
                pub use facade::Intermediate as Exposed;
                pub use facade::intermediate_helper as exposed_helper;
            "#,
        );
        assert!(index.path_reaches_symbol("fixture::Exposed::call", "b2TransitiveType"));
        assert!(index.path_reaches_symbol("fixture::exposed_helper", "b2TransitiveFunction"));
        assert!(index.contains_public_type_path("fixture::Exposed"));
        assert!(!index.contains_public_type_path("fixture::exposed_helper"));
        assert!(index.contains_public_callable_path("fixture::Exposed::call"));
        assert!(index.contains_public_callable_path("fixture::exposed_helper"));
        assert!(index.contains_public_field_path("fixture::Exposed::exposed"));
        assert!(!index.contains_public_field_path("fixture::Exposed::hidden"));
    }

    #[test]
    fn cyclic_public_reexports_fail_closed() {
        let index = index_fixture(
            "cyclic-root-reexports",
            r#"
                mod facade {
                    pub use crate::Cyclic as Intermediate;
                }
                pub use facade::Intermediate as Cyclic;
            "#,
        );
        assert!(!index.contains_public_path("fixture::Cyclic"));
    }

    #[test]
    fn ambiguous_public_reexports_fail_closed() {
        let index = index_fixture(
            "ambiguous-root-reexports",
            r#"
                mod alpha {
                    pub struct Thing { pub value: i32 }
                    pub fn helper() { unsafe { boxdd_sys::ffi::b2AliasAlpha() }; }
                }
                mod beta {
                    pub struct Thing { pub value: i32 }
                    pub fn helper() { unsafe { boxdd_sys::ffi::b2AliasBeta() }; }
                }
                mod facade {
                    pub use crate::alpha::helper as ambiguous;
                    pub use crate::beta::helper as ambiguous;
                    pub use crate::alpha::Thing as AmbiguousThing;
                    pub use crate::beta::Thing as AmbiguousThing;
                }
                pub use facade::ambiguous;
                pub use facade::AmbiguousThing as ExposedThing;
            "#,
        );
        assert!(!index.contains_public_path("fixture::ambiguous"));
        assert!(!index.contains_public_type_path("fixture::ExposedThing"));
        assert!(!index.contains_public_field_path("fixture::ExposedThing::value"));
        assert!(index.paths_for_symbol("b2AliasAlpha").next().is_none());
        assert!(index.paths_for_symbol("b2AliasBeta").next().is_none());
    }

    #[test]
    fn same_named_private_items_do_not_inherit_root_visibility() {
        let index = index_fixture(
            "private-root-name-collision",
            r#"
                pub struct Same { pub root_field: i32 }
                pub fn same() {}
                mod hidden {
                    pub struct Same { pub forged_field: i32 }
                    impl Same {
                        pub fn fake() { unsafe { boxdd_sys::ffi::b2PrivateType() }; }
                    }
                    pub fn same() { unsafe { boxdd_sys::ffi::b2PrivateFunction() }; }
                }
            "#,
        );
        assert!(!index.contains_public_path("fixture::Same::fake"));
        assert!(index.contains_public_field_path("fixture::Same::root_field"));
        assert!(!index.contains_public_field_path("fixture::Same::forged_field"));
        assert!(index.paths_for_symbol("b2PrivateType").next().is_none());
        assert!(index.paths_for_symbol("b2PrivateFunction").next().is_none());
    }

    #[test]
    fn ffi_witnesses_require_exact_type_field_and_public_path_relations() {
        let index = index_fixture(
            "ffi-witness-exactness",
            r#"
                mod implementation {
                    use boxdd_sys::ffi as raw;

                    pub struct Vec2 {
                        pub x: f32,
                        pub y: f32,
                    }

                    impl Vec2 {
                        pub fn from_raw(value: raw::b2Vec2) -> Self {
                            Self { x: value.x, y: value.y }
                        }

                        pub fn into_raw(self) -> raw::b2Vec2 {
                            raw::b2Vec2 { x: self.x, y: self.y }
                        }
                    }
                }

                pub use implementation::Vec2;
                pub fn unrelated() {}
            "#,
        );
        let raw = "boxdd_sys::ffi::b2Vec2";
        assert!(index.path_has_ffi_type_witness("fixture::Vec2", raw));
        assert!(index.path_has_ffi_type_witness("fixture::Vec2::from_raw", raw));
        assert!(index.path_has_safe_ffi_type_witness("fixture::Vec2", raw));
        assert!(index.path_has_safe_ffi_type_witness("fixture::Vec2::from_raw", raw));
        assert!(index.path_has_ffi_field_witness("fixture::Vec2::x", raw, "x"));
        assert!(index.path_has_ffi_field_witness("fixture::Vec2::y", raw, "y"));
        assert!(index.path_has_safe_ffi_field_witness("fixture::Vec2::x", raw, "x"));
        assert!(index.path_has_safe_ffi_field_witness("fixture::Vec2::y", raw, "y"));
        assert!(!index.path_has_ffi_field_witness("fixture::Vec2", raw, "x"));
        assert!(!index.path_has_ffi_field_witness("fixture::Vec2::y", raw, "x"));
        assert!(!index.path_has_ffi_type_witness("fixture::unrelated", raw));
        assert!(!index.path_has_ffi_field_witness("fixture::unrelated", raw, "x"));
        assert!(!index.path_has_ffi_type_witness("fixture::Missing", raw));
        assert_eq!(
            index.paths_with_ffi_type_witness(raw).collect::<Vec<_>>(),
            vec![
                "fixture::Vec2",
                "fixture::Vec2::from_raw",
                "fixture::Vec2::into_raw"
            ]
        );
        assert_eq!(
            index
                .paths_with_ffi_field_witness(raw, "x")
                .collect::<Vec<_>>(),
            vec![
                "fixture::Vec2::from_raw",
                "fixture::Vec2::into_raw",
                "fixture::Vec2::x"
            ]
        );
        assert!(
            index
                .paths_with_ffi_type_witness("boxdd_sys::ffi::Missing")
                .next()
                .is_none()
        );
        assert!(
            index
                .paths_with_ffi_field_witness(raw, "missing")
                .next()
                .is_none()
        );
    }

    #[test]
    fn unsafe_public_callables_do_not_create_safe_callable_or_abi_witnesses() {
        let index = index_fixture(
            "unsafe-public-witness",
            r#"
                use boxdd_sys::ffi;

                pub struct SafeValue(i32);
                impl SafeValue {
                    pub unsafe fn from_raw(raw: ffi::b2RawValue) -> Self {
                        unsafe { ffi::b2ConsumeRawValue(raw) };
                        Self(0)
                    }
                }
            "#,
        );
        let callable = "fixture::SafeValue::from_raw";
        let raw = "boxdd_sys::ffi::b2RawValue";
        assert!(index.contains_public_callable_path(callable));
        assert!(!index.contains_public_safe_callable_path(callable));
        assert!(index.path_has_ffi_type_witness("fixture::SafeValue", raw));
        assert!(!index.path_has_safe_ffi_type_witness("fixture::SafeValue", raw));
        assert!(!index.path_has_safe_ffi_type_witness(callable, raw));
    }

    #[test]
    fn public_type_and_callable_witness_kinds_are_disjoint() {
        let index = index_fixture(
            "ffi-witness-kinds",
            r#"
                use boxdd_sys::ffi;

                pub struct Wrapper;
                impl Wrapper {
                    pub fn from_raw(_: ffi::b2Thing) -> Self { Self }
                }

                pub fn adapter(_: ffi::b2Thing) -> Wrapper { Wrapper }
            "#,
        );
        assert!(index.contains_public_type_path("fixture::Wrapper"));
        assert!(!index.contains_public_callable_path("fixture::Wrapper"));
        assert!(index.contains_public_callable_path("fixture::Wrapper::from_raw"));
        assert!(!index.contains_public_type_path("fixture::Wrapper::from_raw"));
        assert!(index.contains_public_callable_path("fixture::adapter"));
        assert!(!index.contains_public_type_path("fixture::adapter"));
        assert!(index.path_has_ffi_type_witness("fixture::Wrapper", "boxdd_sys::ffi::b2Thing"));
        assert!(index.path_has_ffi_type_witness("fixture::adapter", "boxdd_sys::ffi::b2Thing"));
    }

    #[test]
    fn ffi_witnesses_propagate_through_helpers_and_preserve_deep_field_chains() {
        let index = index_fixture(
            "ffi-witness-helper",
            r#"
                use boxdd_sys::ffi;

                pub struct Safe {
                    pub value: f32,
                }

                fn decode(raw: ffi::b2Nested) -> Safe {
                    Safe { value: raw.outer.value }
                }

                impl Safe {
                    pub fn from_raw(raw: ffi::b2Nested) -> Self {
                        decode(raw)
                    }
                }
            "#,
        );
        let raw = "boxdd_sys::ffi::b2Nested";
        assert!(index.path_has_ffi_type_witness("fixture::Safe", raw));
        assert!(index.path_has_ffi_type_witness("fixture::Safe::from_raw", raw));
        assert!(index.path_has_ffi_field_witness("fixture::Safe::from_raw", raw, "outer::value"));
        assert!(index.path_has_ffi_field_witness("fixture::Safe::value", raw, "outer::value"));
        assert!(!index.path_has_ffi_field_witness("fixture::Safe::value", raw, "value"));
    }

    #[test]
    fn ffi_witnesses_reject_private_cfg_disabled_and_shadowed_sources() {
        let private = index_fixture(
            "ffi-witness-private",
            r#"
                use boxdd_sys::ffi;

                struct Hidden { pub value: f32 }
                impl Hidden {
                    pub fn from_raw(raw: ffi::b2Hidden) -> Self {
                        Self { value: raw.value }
                    }
                }

                pub struct Visible { pub value: f32 }
                #[cfg(any())]
                impl Visible {
                    pub fn from_raw(raw: ffi::b2Disabled) -> Self {
                        Self { value: raw.value }
                    }
                }
            "#,
        );
        assert!(!private.path_has_ffi_type_witness("fixture::Hidden", "boxdd_sys::ffi::b2Hidden"));
        assert!(!private.path_has_ffi_field_witness(
            "fixture::Hidden::value",
            "boxdd_sys::ffi::b2Hidden",
            "value"
        ));
        assert!(
            !private.path_has_ffi_type_witness("fixture::Visible", "boxdd_sys::ffi::b2Disabled")
        );

        let shadowed = index_fixture(
            "ffi-witness-shadowed",
            r#"
                mod boxdd_sys {
                    pub mod ffi {
                        pub struct b2Forged { pub value: f32 }
                    }
                }

                pub struct Safe { pub value: f32 }
                impl Safe {
                    pub fn from_raw(raw: boxdd_sys::ffi::b2Forged) -> Self {
                        Self { value: raw.value }
                    }
                }
            "#,
        );
        assert!(!shadowed.path_has_ffi_type_witness("fixture::Safe", "boxdd_sys::ffi::b2Forged"));
    }

    #[test]
    fn ambiguous_raw_type_aliases_fail_closed() {
        let index = index_fixture(
            "ffi-witness-ambiguous-alias",
            r#"
                mod fake {
                    pub mod ffi {
                        pub struct b2Vec2 { pub x: f32 }
                    }
                }
                use boxdd_sys::ffi::b2Vec2 as Raw;
                use crate::fake::ffi::b2Vec2 as Raw;

                pub fn forged(raw: Raw) -> f32 { raw.x }
            "#,
        );
        assert!(!index.path_has_ffi_type_witness("fixture::forged", "boxdd_sys::ffi::b2Vec2"));
        assert!(!index.path_has_ffi_field_witness(
            "fixture::forged",
            "boxdd_sys::ffi::b2Vec2",
            "x"
        ));
    }

    #[test]
    fn ambiguous_external_imports_cannot_fake_ffi_provenance() {
        let index = index_fixture(
            "ffi-witness-external-alias-ambiguity",
            r#"
                use another_crate::ffi::b2Vec2 as Raw;
                use boxdd_sys::ffi::b2Vec2 as Raw;
                use another_crate::ffi::b2Step as native_step;
                use boxdd_sys::ffi::b2Step as native_step;

                pub fn forged(raw: Raw) -> f32 {
                    unsafe { native_step() };
                    raw.x
                }
            "#,
        );
        assert!(!index.path_has_ffi_type_witness("fixture::forged", "boxdd_sys::ffi::b2Vec2"));
        assert!(!index.path_has_ffi_field_witness(
            "fixture::forged",
            "boxdd_sys::ffi::b2Vec2",
            "x"
        ));
        assert!(!index.path_reaches_symbol("fixture::forged", "b2Step"));
    }

    #[test]
    fn production_index_proves_canonical_types_nested_impls_and_field_witnesses() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root");
        let index = index_boxdd(root).expect("production boxdd index");
        let raw = "boxdd_sys::ffi::b2Vec2";
        assert!(index.contains_public_type_path("boxdd::Vec2"));
        assert!(index.contains_public_callable_path("boxdd::Vec2::from_raw"));
        assert!(index.contains_public_field_path("boxdd::Vec2::x"));
        assert!(index.path_has_ffi_type_witness("boxdd::Vec2", raw));
        assert!(index.path_has_ffi_field_witness("boxdd::Vec2::x", raw, "x"));
        assert!(!index.path_has_ffi_field_witness("boxdd::Vec2::y", raw, "x"));
        assert!(!index.path_has_ffi_type_witness("boxdd::Rot", raw));

        assert!(index.contains_public_type_path("boxdd::World"));
        assert!(index.contains_public_callable_path("boxdd::World::step"));
        assert!(index.contains_public_type_path("boxdd::Capsule"));
        assert!(index.contains_public_callable_path("boxdd::Capsule::aabb"));
        assert!(index.contains_public_type_path("boxdd::ContactId"));
        assert!(index.contains_public_callable_path("boxdd::ContactId::data"));
    }

    #[test]
    fn ffi_witnesses_follow_typed_locals_struct_literals_and_raw_receiver_storage() {
        let index = index_fixture(
            "ffi-witness-body-forms",
            r#"
                use boxdd_sys::ffi;

                pub struct Definition(ffi::b2Definition);

                impl Definition {
                    pub fn set_limit(&mut self, value: f32) {
                        self.0.base.limit = value;
                    }
                }

                pub fn make_point(x: f32) -> ffi::b2Point {
                    let mut raw: ffi::b2Point = ffi::b2Point { x };
                    raw.x = x;
                    raw
                }
            "#,
        );
        assert!(
            index.path_has_ffi_type_witness("fixture::Definition", "boxdd_sys::ffi::b2Definition")
        );
        assert!(index.path_has_ffi_field_witness(
            "fixture::Definition::set_limit",
            "boxdd_sys::ffi::b2Definition",
            "base::limit"
        ));
        assert!(index.path_has_ffi_type_witness("fixture::make_point", "boxdd_sys::ffi::b2Point"));
        assert!(index.path_has_ffi_field_witness(
            "fixture::make_point",
            "boxdd_sys::ffi::b2Point",
            "x"
        ));
    }

    #[test]
    fn function_pointer_flow_requires_an_observed_call() {
        let index = index_fixture(
            "function-pointer",
            r#"
                pub fn invoked() {
                    let fp = boxdd_sys::ffi::b2InvokedThroughPointer;
                    unsafe { fp() };
                }

                pub fn mentioned_only() {
                    let fp = boxdd_sys::ffi::b2MentionedOnly;
                    let _ = fp;
                }

                pub fn reassigned() {
                    let mut fp = boxdd_sys::ffi::b2StaleBinding;
                    fp = local_callback;
                    fp();
                }

                fn local_callback() {}
            "#,
        );
        assert!(index.path_reaches_symbol("fixture::invoked", "b2InvokedThroughPointer"));
        assert!(index.paths_for_symbol("b2MentionedOnly").next().is_none());
        assert!(index.paths_for_symbol("b2StaleBinding").next().is_none());
    }

    #[test]
    fn function_pointer_flow_propagates_through_called_helpers() {
        let index = index_fixture(
            "function-pointer-helper",
            r#"
                fn invoke(callback: unsafe extern "C" fn()) {
                    unsafe { callback() };
                }
                fn adapter(callback: unsafe extern "C" fn()) {
                    invoke(callback);
                }
                pub fn api() {
                    adapter(boxdd_sys::ffi::b2InvokedThroughHelper);
                }
            "#,
        );
        assert!(index.path_reaches_symbol("fixture::api", "b2InvokedThroughHelper"));
    }

    #[test]
    fn callable_argument_witnesses_distinguish_trampolines_none_and_argument_slots() {
        let index = index_fixture(
            "callable-argument-witness",
            r#"
                unsafe extern "C" fn trampoline() {}

                fn install(callback: unsafe extern "C" fn()) {
                    unsafe { boxdd_sys::ffi::b2Install(Some(callback)) };
                }

                pub fn installed() {
                    install(trampoline);
                }

                pub fn absent() {
                    unsafe { boxdd_sys::ffi::b2Install(None) };
                }

                pub fn second_slot() {
                    unsafe { boxdd_sys::ffi::b2InstallAt(7, Some(trampoline)) };
                }
            "#,
        );
        assert!(index.path_reaches_symbol("fixture::installed", "b2Install"));
        assert!(index.path_reaches_symbol_with_callable_argument(
            "fixture::installed",
            "b2Install",
            0
        ));
        assert!(!index.path_reaches_symbol_with_callable_argument(
            "fixture::installed",
            "b2Install",
            1
        ));
        assert!(index.path_reaches_symbol("fixture::absent", "b2Install"));
        assert!(!index.path_reaches_symbol_with_callable_argument(
            "fixture::absent",
            "b2Install",
            0
        ));
        assert!(index.path_reaches_symbol_with_callable_argument(
            "fixture::second_slot",
            "b2InstallAt",
            1
        ));
        assert!(!index.path_reaches_symbol_with_callable_argument(
            "fixture::second_slot",
            "b2InstallAt",
            0
        ));
    }

    #[test]
    fn locally_shadowed_some_cannot_forge_a_callable_argument_witness() {
        let index = index_fixture(
            "shadowed-option-some",
            r#"
                unsafe extern "C" fn trampoline() {}
                #[allow(non_snake_case)]
                fn Some<T>(_: T) -> Option<T> { None }

                pub fn forged() {
                    unsafe { boxdd_sys::ffi::b2Install(Some(trampoline)) };
                }
            "#,
        );
        assert!(index.path_reaches_symbol("fixture::forged", "b2Install"));
        assert!(!index.path_reaches_symbol_with_callable_argument(
            "fixture::forged",
            "b2Install",
            0
        ));
    }

    #[test]
    fn callable_field_argument_witnesses_require_the_same_live_binding() {
        let index = index_fixture(
            "callable-field-argument-witness",
            r#"
                unsafe extern "C" fn trampoline() {}

                fn unknown_mutation(_: &mut boxdd_sys::ffi::b2Owner) {}

                pub fn installed() {
                    let mut owner: boxdd_sys::ffi::b2Owner = unsafe { core::mem::zeroed() };
                    owner.callback = Some(trampoline);
                    unsafe { boxdd_sys::ffi::b2ConsumeOwner(7, &mut owner) };
                }

                pub fn different_binding() {
                    let mut configured: boxdd_sys::ffi::b2Owner = unsafe { core::mem::zeroed() };
                    configured.callback = Some(trampoline);
                    let mut consumed: boxdd_sys::ffi::b2Owner = unsafe { core::mem::zeroed() };
                    unsafe { boxdd_sys::ffi::b2ConsumeOwner(7, &mut consumed) };
                }

                pub fn overwritten_with_none() {
                    let mut owner: boxdd_sys::ffi::b2Owner = unsafe { core::mem::zeroed() };
                    owner.callback = Some(trampoline);
                    owner.callback = None;
                    unsafe { boxdd_sys::ffi::b2ConsumeOwner(7, &mut owner) };
                }

                pub fn escaped_to_unknown_mutation() {
                    let mut owner: boxdd_sys::ffi::b2Owner = unsafe { core::mem::zeroed() };
                    owner.callback = Some(trampoline);
                    unknown_mutation(&mut owner);
                    unsafe { boxdd_sys::ffi::b2ConsumeOwner(7, &mut owner) };
                }

                pub fn constructed() {
                    let owner = boxdd_sys::ffi::b2Owner {
                        callback: Some(trampoline),
                    };
                    unsafe { boxdd_sys::ffi::b2ConsumeOwner(7, &owner) };
                }
            "#,
        );
        for path in ["fixture::installed", "fixture::constructed"] {
            assert!(index.path_reaches_symbol_with_callable_field_argument(
                path,
                "b2ConsumeOwner",
                1,
                "callback"
            ));
            assert!(!index.path_reaches_symbol_with_callable_field_argument(
                path,
                "b2ConsumeOwner",
                0,
                "callback"
            ));
        }
        for path in [
            "fixture::different_binding",
            "fixture::overwritten_with_none",
            "fixture::escaped_to_unknown_mutation",
        ] {
            assert!(index.path_reaches_symbol(path, "b2ConsumeOwner"));
            assert!(!index.path_reaches_symbol_with_callable_field_argument(
                path,
                "b2ConsumeOwner",
                1,
                "callback"
            ));
        }
    }

    #[test]
    fn closure_bodies_require_a_proven_invocation() {
        let index = index_fixture(
            "closure-invocation",
            r#"
                fn invoke(callback: impl FnOnce()) { callback(); }
                pub fn invoked() { invoke(|| unsafe { boxdd_sys::ffi::b2InvokedClosure() }); }
                pub fn mentioned_only() {
                    let callback = || unsafe { boxdd_sys::ffi::b2UncalledClosure() };
                    let _ = callback;
                }
            "#,
        );
        assert!(index.path_reaches_symbol("fixture::invoked", "b2InvokedClosure"));
        assert!(index.paths_for_symbol("b2UncalledClosure").next().is_none());
    }

    #[test]
    fn type_constant_and_struct_paths_are_not_calls() {
        let index = index_fixture(
            "non-call-paths",
            r#"
                pub fn mentions() {
                    let _: Option<boxdd_sys::ffi::b2MentionedType> = None;
                    let _ = boxdd_sys::ffi::b2MentionedConstant;
                    let _ = boxdd_sys::ffi::b2MentionedStruct { value: 1 };
                }
            "#,
        );
        assert!(index.paths_for_symbol("b2MentionedType").next().is_none());
        assert!(
            index
                .paths_for_symbol("b2MentionedConstant")
                .next()
                .is_none()
        );
        assert!(index.paths_for_symbol("b2MentionedStruct").next().is_none());
    }

    #[test]
    fn full_module_qualifiers_distinguish_same_named_helpers() {
        let index = index_fixture(
            "qualified-helpers",
            r#"
                pub mod alpha {
                    pub fn helper() { unsafe { boxdd_sys::ffi::b2Alpha() }; }
                    pub fn local_api() { helper(); }
                }
                pub mod beta {
                    pub fn helper() { unsafe { boxdd_sys::ffi::b2Beta() }; }
                    pub fn local_api() { helper(); }
                }
                pub fn qualified_api() { crate::alpha::helper(); }
            "#,
        );
        assert!(index.path_reaches_symbol("fixture::alpha::local_api", "b2Alpha"));
        assert!(!index.path_reaches_symbol("fixture::alpha::local_api", "b2Beta"));
        assert!(index.path_reaches_symbol("fixture::beta::local_api", "b2Beta"));
        assert!(!index.path_reaches_symbol("fixture::beta::local_api", "b2Alpha"));
        assert!(index.path_reaches_symbol("fixture::qualified_api", "b2Alpha"));
        assert!(!index.path_reaches_symbol("fixture::qualified_api", "b2Beta"));
    }

    #[test]
    fn ambiguous_imported_helpers_fail_closed() {
        let index = index_fixture(
            "ambiguous-helper",
            r#"
                mod alpha { pub fn helper() { unsafe { boxdd_sys::ffi::b2Alpha() }; } }
                mod beta { pub fn helper() { unsafe { boxdd_sys::ffi::b2Beta() }; } }
                use alpha::helper;
                use beta::helper;
                pub fn ambiguous_api() { helper(); }
            "#,
        );
        assert!(!index.path_reaches_symbol("fixture::ambiguous_api", "b2Alpha"));
        assert!(!index.path_reaches_symbol("fixture::ambiguous_api", "b2Beta"));
    }

    #[test]
    fn structural_call_graph_skips_statically_dead_control_flow() {
        let index = index_fixture(
            "dead-control-flow",
            r#"
                use boxdd_sys::ffi;

                pub fn dead_if() {
                    if 1 == 2 { unsafe { ffi::b2DeadIf() }; }
                }
                pub fn dead_match() {
                    match false {
                        true => unsafe { ffi::b2DeadMatch() },
                        false => (),
                    }
                }
                pub fn dead_short_circuit() {
                    false && { unsafe { ffi::b2DeadShortCircuit() }; true };
                }
                pub fn after_return() {
                    return;
                    unsafe { ffi::b2AfterReturn() };
                }
                pub fn after_panic() {
                    panic!("stop");
                    unsafe { ffi::b2AfterPanic() };
                }
                pub fn conditional(flag: bool) {
                    if flag { unsafe { ffi::b2Conditional() }; }
                }
            "#,
        );
        for (path, symbol) in [
            ("fixture::dead_if", "b2DeadIf"),
            ("fixture::dead_match", "b2DeadMatch"),
            ("fixture::dead_short_circuit", "b2DeadShortCircuit"),
            ("fixture::after_return", "b2AfterReturn"),
            ("fixture::after_panic", "b2AfterPanic"),
        ] {
            assert!(!index.path_reaches_symbol(path, symbol));
        }
        assert!(index.path_reaches_symbol("fixture::conditional", "b2Conditional"));
    }

    #[test]
    fn method_edges_require_a_proven_receiver() {
        let index = index_fixture(
            "method-receivers",
            r#"
                pub struct Wrapper;
                impl Wrapper {
                    fn helper(&self) { unsafe { boxdd_sys::ffi::b2MethodHelper() }; }
                    pub fn via_self(&self) { self.helper(); }
                    pub fn via_other(&self, other: &Self) { other.helper(); }
                }
                pub fn via_explicit_type() { Wrapper::helper(&Wrapper); }
            "#,
        );
        assert!(index.path_reaches_symbol("fixture::Wrapper::via_self", "b2MethodHelper"));
        assert!(!index.path_reaches_symbol("fixture::Wrapper::via_other", "b2MethodHelper"));
        assert!(index.path_reaches_symbol("fixture::via_explicit_type", "b2MethodHelper"));
    }

    #[test]
    fn private_trait_defaults_are_not_guessed_as_inherent_methods() {
        let index = index_fixture(
            "private-trait-default",
            r#"
                pub struct PublicType;
                trait HiddenBehavior {
                    fn hidden(&self) { unsafe { boxdd_sys::ffi::b2HiddenDefault() }; }
                }
                impl HiddenBehavior for PublicType {}
                impl PublicType {
                    pub fn call_hidden(&self) { self.hidden(); }
                }
            "#,
        );
        assert!(!index.path_reaches_symbol("fixture::PublicType::call_hidden", "b2HiddenDefault"));
    }

    #[test]
    fn private_standard_named_traits_do_not_create_public_api_paths() {
        let index = index_fixture(
            "private-standard-trait-names",
            r#"
                trait Default { fn default() -> Self; }
                trait Serialize { fn serialize(&self); }

                pub struct DefaultVictim;
                impl Default for DefaultVictim {
                    fn default() -> Self {
                        unsafe { boxdd_sys::ffi::b2FakeDefault() };
                        Self
                    }
                }

                pub struct SerializeVictim;
                impl Serialize for SerializeVictim {
                    fn serialize(&self) {
                        unsafe { boxdd_sys::ffi::b2FakeSerialize() };
                    }
                }
            "#,
        );
        assert!(!index.contains_public_path("fixture::DefaultVictim::default"));
        assert!(!index.contains_public_path("fixture::SerializeVictim::serialize"));
        assert!(index.paths_for_symbol("b2FakeDefault").next().is_none());
        assert!(index.paths_for_symbol("b2FakeSerialize").next().is_none());
    }

    #[test]
    fn private_drop_named_trait_does_not_create_raii_coverage() {
        let index = index_fixture(
            "private-drop-trait-name",
            r#"
                mod internals {
                    pub(crate) struct Resource;
                    trait Drop { fn drop(&mut self); }
                    impl Drop for Resource {
                        fn drop(&mut self) {
                            unsafe { boxdd_sys::ffi::b2FakeDrop() };
                        }
                    }
                }
                pub struct Owner { resource: internals::Resource }
            "#,
        );
        assert!(!index.path_reaches_symbol("fixture::Owner", "b2FakeDrop"));
    }

    #[test]
    fn drop_coverage_requires_an_owned_field() {
        let index = index_fixture(
            "drop-ownership",
            r#"
                mod internals {
                    pub(crate) struct Resource;
                    impl Drop for Resource {
                        fn drop(&mut self) { destroy_resource(); }
                    }
                    fn destroy_resource() { unsafe { boxdd_sys::ffi::b2DestroyResource() }; }
                }
                use internals::Resource;
                pub struct Owner { resource: Resource }
                pub struct Borrower<'a> { resource: &'a Resource }
                pub struct ForeignOwner { resource: external_crate::Resource }
            "#,
        );
        assert!(index.path_reaches_symbol("fixture::Owner", "b2DestroyResource"));
        assert!(!index.path_reaches_symbol("fixture::Borrower", "b2DestroyResource"));
        assert!(!index.path_reaches_symbol("fixture::ForeignOwner", "b2DestroyResource"));
    }

    #[test]
    fn evidence_requires_test_attribute() {
        let root = fixture_root("evidence");
        let tests = root.join("boxdd/tests");
        fs::create_dir_all(&tests).expect("fixture tests");
        fs::write(
            tests.join("proof.rs"),
            "fn helper() {}\n#[test]\nfn actual_proof() { assert!(true); }\n",
        )
        .expect("fixture source");
        validate_test_evidence(&root, "boxdd/tests/proof.rs", "actual_proof")
            .expect("test evidence should resolve");
        assert!(validate_test_evidence(&root, "boxdd/tests/proof.rs", "helper").is_err());
        fs::remove_dir_all(root).expect("fixture cleanup");
    }

    #[test]
    fn evidence_fingerprint_is_ast_normalized_and_item_specific() {
        let first_root = fixture_root("evidence-fingerprint-first");
        let second_root = fixture_root("evidence-fingerprint-second");
        for root in [&first_root, &second_root] {
            fs::create_dir_all(root.join("boxdd/tests")).expect("fixture tests");
        }
        fs::write(
            first_root.join("boxdd/tests/proof.rs"),
            r#"
                #[test]
                fn actual() {
                    // Formatting and comments do not carry test semantics.
                    assert!(1 + 1 == 2);
                }

                #[test]
                fn unrelated() { assert!(2 + 2 == 4); }
            "#,
        )
        .expect("first evidence");
        fs::write(
            second_root.join("boxdd/tests/proof.rs"),
            "#[test] fn actual(){assert!(1+1==2);}\n#[test] fn unrelated(){assert!(2+2==4);}\n",
        )
        .expect("second evidence");

        let reviewed = validate_test_evidence(&first_root, "boxdd/tests/proof.rs", "actual")
            .expect("reviewed evidence");
        let reformatted = validate_test_evidence(&second_root, "boxdd/tests/proof.rs", "actual")
            .expect("reformatted evidence");
        let wrong_existing =
            validate_test_evidence(&first_root, "boxdd/tests/proof.rs", "unrelated")
                .expect("other existing evidence");
        assert_eq!(reviewed, reformatted);
        assert_ne!(reviewed, wrong_existing);
        assert!(reviewed.starts_with("blake3:"));

        fs::write(
            second_root.join("boxdd/tests/proof.rs"),
            "#[test] fn actual(){assert!(1+1==3);}\n",
        )
        .expect("changed evidence");
        let changed = validate_test_evidence(&second_root, "boxdd/tests/proof.rs", "actual")
            .expect("changed existing evidence");
        assert_ne!(reviewed, changed);
        fs::remove_dir_all(first_root).expect("first fixture cleanup");
        fs::remove_dir_all(second_root).expect("second fixture cleanup");
    }

    #[test]
    fn evidence_call_index_requires_reachable_calls_and_exact_method_owners() {
        let root = fixture_root("typed-evidence-call-index");
        fs::create_dir_all(root.join("boxdd/src")).expect("fixture source");
        fs::create_dir_all(root.join("boxdd/tests")).expect("fixture tests");
        fs::write(
            root.join("boxdd/Cargo.toml"),
            "[package]\nname = \"boxdd\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[dependencies]\nboxdd-sys = \"0\"\n",
        )
        .expect("fixture manifest");
        fs::write(
            root.join("boxdd/src/lib.rs"),
            r#"
                mod implementation {
                    pub struct World;
                    impl World {
                        pub fn new() -> Result<Self, ()> {
                            unsafe { boxdd_sys::ffi::b2CreateWorld() };
                            Ok(Self)
                        }
                        pub fn step(&mut self) {
                            unsafe { boxdd_sys::ffi::b2WorldStep() };
                        }
                        pub unsafe fn unsafe_step(&mut self) {
                            unsafe { boxdd_sys::ffi::b2UnsafeWorldStep() };
                        }
                    }
                }
                pub use implementation::World;
                pub mod prelude { pub use crate::World; }

                pub struct Other;
                impl Other {
                    pub fn step(&mut self) {
                        unsafe { boxdd_sys::ffi::b2WrongOwner() };
                    }
                }

                struct Guard;
                impl Drop for Guard {
                    fn drop(&mut self) {
                        unsafe { boxdd_sys::ffi::b2DestroyHandle() };
                    }
                }
                pub struct Handle { guard: Guard }
                impl Handle {
                    pub fn new() -> Self { Self { guard: Guard } }
                }
            "#,
        )
        .expect("fixture library");
        fs::write(
            root.join("boxdd/tests/proof.rs"),
            r#"
                use boxdd::prelude::*;
                use boxdd::{Handle, Other};

                fn exercise(world: &mut World) { world.step(); }
                fn unrelated(other: &mut Other) { other.step(); }
                fn route_condition() -> bool { true }
                fn drop(_: Handle) {}
                #[cfg(feature = "single")]
                fn route_helper() { let mut world = World; World::step(&mut world); }
                #[cfg(feature = "double")]
                fn route_helper() { let mut other = Other; Other::step(&mut other); }

                #[test]
                fn proof() {
                    let mut world = World::new().unwrap();
                    assert_eq!({ exercise(&mut world); 1 }, 1);
                    World::step(&mut world);
                    let _mentioned_only = Other::step;
                    let _text = "boxdd::Other::step";
                }

                #[test]
                fn dead_if() {
                    let mut world = World;
                    if 1 == 2 { World::step(&mut world); }
                }

                #[test]
                fn dead_match() {
                    let mut world = World;
                    match false {
                        true => World::step(&mut world),
                        false => (),
                    }
                }

                #[test]
                fn short_circuit() {
                    let mut world = World;
                    false && { World::step(&mut world); true };
                }

                #[test]
                fn conditional_return() {
                    let mut world = World;
                    if route_condition() { return; }
                    World::step(&mut world);
                }

                #[test]
                fn function_local_shadowed_assert() {
                    let mut world = World;
                    macro_rules! assert {
                        ($expression:expr) => {{ let _ = stringify!($expression); }}
                    }
                    assert!(World::step(&mut world));
                }

                #[test]
                fn route_proof() { route_helper(); }

                #[test]
                fn unsafe_callable() {
                    let mut world = World;
                    unsafe { world.unsafe_step(); }
                }

                #[test]
                fn explicit_raii_drop() {
                    let handle = Handle::new();
                    core::mem::drop(handle);
                }

                #[test]
                fn borrowed_drop_is_not_raii() {
                    let handle = Handle::new();
                    std::mem::drop(&handle);
                }

                #[test]
                fn implicit_scope_drop_is_not_evidence() {
                    let _handle = Handle::new();
                }

                #[test]
                fn shadowed_drop_is_not_raii() {
                    let handle = Handle::new();
                    drop(handle);
                }

                #[test]
                fn conditional_drop_is_not_raii() {
                    let handle = Handle::new();
                    if true { core::mem::drop(handle); }
                }

                #[test]
                fn shadowed_core_drop_is_not_raii() {
                    mod core { pub mod mem { pub fn drop<T>(_: T) {} } }
                    let handle = Handle::new();
                    core::mem::drop(handle);
                }

                #[test]
                #[should_panic]
                fn after_guaranteed_panic() {
                    let mut world = World;
                    panic!("expected");
                    World::step(&mut world);
                }
            "#,
        )
        .expect("fixture evidence");

        let rust_index = index_boxdd(&root).expect("fixture Rust index");
        let evidence = index_test_evidence(&root, "boxdd/tests/proof.rs", "proof", &rust_index)
            .expect("typed evidence index");
        assert_eq!(
            evidence.called_public_paths,
            BTreeSet::from([
                "boxdd::World::new".to_owned(),
                "boxdd::World::step".to_owned(),
            ])
        );
        assert_eq!(
            evidence.implementation_reachable_symbols,
            BTreeSet::from(["b2CreateWorld".to_owned(), "b2WorldStep".to_owned()])
        );
        assert_eq!(
            evidence.called_local_paths,
            BTreeSet::from(["exercise".to_owned()])
        );
        assert!(!evidence.called_public_paths.contains("boxdd::Other::step"));
        for item in [
            "dead_if",
            "dead_match",
            "short_circuit",
            "conditional_return",
            "function_local_shadowed_assert",
        ] {
            let forged = index_test_evidence(&root, "boxdd/tests/proof.rs", item, &rust_index)
                .expect("forged evidence remains syntactically indexable");
            assert!(
                !forged.called_public_paths.contains("boxdd::World::step"),
                "conditional call in `{item}` must not become evidence"
            );
            if item == "function_local_shadowed_assert" {
                assert!(
                    forged.unresolved_calls.iter().any(|gap| {
                        gap.reason == "assertion macro name is shadowed or ambiguous"
                    })
                );
            }
        }
        assert!(
            index_test_evidence(
                &root,
                "boxdd/tests/proof.rs",
                "after_guaranteed_panic",
                &rust_index,
            )
            .is_err()
        );
        let single_coordinate =
            RustIndexCoordinate::source_single().with_cfg_values("feature", ["single"]);
        let double_coordinate =
            RustIndexCoordinate::source_single().with_cfg_values("feature", ["double"]);
        let single = index_test_evidence_at_coordinate(
            &root,
            "boxdd/tests/proof.rs",
            "route_proof",
            &rust_index,
            &single_coordinate,
        )
        .expect("single-route evidence");
        let double = index_test_evidence_at_coordinate(
            &root,
            "boxdd/tests/proof.rs",
            "route_proof",
            &rust_index,
            &double_coordinate,
        )
        .expect("double-route evidence");
        assert_eq!(
            single.called_public_paths,
            BTreeSet::from(["boxdd::World::step".to_owned()])
        );
        assert_eq!(
            double.called_public_paths,
            BTreeSet::from(["boxdd::Other::step".to_owned()])
        );
        assert_ne!(single.fingerprint, double.fingerprint);
        let unsafe_evidence = index_test_evidence(
            &root,
            "boxdd/tests/proof.rs",
            "unsafe_callable",
            &rust_index,
        )
        .expect("unsafe callable test is indexable but not Safe evidence");
        assert!(
            !unsafe_evidence
                .called_public_paths
                .contains("boxdd::World::unsafe_step")
        );
        let raii_evidence = index_test_evidence(
            &root,
            "boxdd/tests/proof.rs",
            "explicit_raii_drop",
            &rust_index,
        )
        .expect("explicit RAII drop evidence");
        assert_eq!(
            raii_evidence.dropped_public_types,
            BTreeSet::from(["boxdd::Handle".to_owned()])
        );
        assert!(
            raii_evidence
                .implementation_reachable_symbols
                .contains("b2DestroyHandle")
        );
        for item in [
            "borrowed_drop_is_not_raii",
            "implicit_scope_drop_is_not_evidence",
            "shadowed_drop_is_not_raii",
            "conditional_drop_is_not_raii",
            "shadowed_core_drop_is_not_raii",
        ] {
            let evidence = index_test_evidence(&root, "boxdd/tests/proof.rs", item, &rust_index)
                .expect("negative RAII evidence remains indexable");
            assert!(
                evidence.dropped_public_types.is_empty(),
                "`{item}` must not prove an owned RAII drop"
            );
        }
        let evidence_source_path = root.join("boxdd/tests/proof.rs");
        let changed_source = fs::read_to_string(&evidence_source_path)
            .expect("read evidence for helper drift")
            .replace(
                "fn exercise(world: &mut World) { world.step(); }",
                "fn exercise(world: &mut World) { World::step(world); }",
            );
        fs::write(&evidence_source_path, changed_source).expect("write helper drift");
        let changed_evidence =
            index_test_evidence(&root, "boxdd/tests/proof.rs", "proof", &rust_index)
                .expect("evidence after helper drift");
        assert_eq!(
            changed_evidence.called_public_paths,
            evidence.called_public_paths
        );
        assert_ne!(changed_evidence.fingerprint, evidence.fingerprint);
        fs::remove_dir_all(root).expect("fixture cleanup");
    }

    #[test]
    fn evidence_rejects_empty_ignored_conditional_fake_ambiguous_and_escaping_tests() {
        let root = fixture_root("forged-evidence");
        let tests = root.join("boxdd/tests");
        fs::create_dir_all(&tests).expect("fixture tests");
        fs::write(
            tests.join("proof.rs"),
            r#"
                #[test]
                fn empty() {}
                #[test]
                #[ignore]
                fn ignored() { assert!(true); }
                #[test]
                #[cfg(any())]
                fn conditional() { assert!(true); }
                #[fake::test]
                fn fake_attribute() { assert!(true); }
                #[test()]
                fn test_list() { assert!(true); }
                #[test = "forged"]
                fn test_name_value() { assert!(true); }
                mod first { #[test] fn duplicate() { assert!(true); } }
                mod second { #[test] fn duplicate() { assert!(true); } }
            "#,
        )
        .expect("forged evidence source");

        for item in [
            "empty",
            "ignored",
            "conditional",
            "fake_attribute",
            "test_list",
            "test_name_value",
            "duplicate",
        ] {
            assert!(
                validate_test_evidence(&root, "boxdd/tests/proof.rs", item).is_err(),
                "forged evidence `{item}` must fail"
            );
        }
        assert!(validate_test_evidence(&root, "../proof.rs", "proof").is_err());
        assert!(
            validate_test_evidence(
                &root,
                tests.join("proof.rs").to_string_lossy().as_ref(),
                "proof",
            )
            .is_err()
        );
        fs::remove_dir_all(root).expect("fixture cleanup");
    }

    #[test]
    fn evidence_rejects_disabled_crate_and_ancestor_module_cfg() {
        let root = fixture_root("ancestor-cfg-evidence");
        let tests = root.join("boxdd/tests");
        fs::create_dir_all(&tests).expect("fixture tests");
        fs::write(
            tests.join("crate_disabled.rs"),
            "#![cfg(any())]\n#[test]\nfn proof() { assert!(true); }\n",
        )
        .expect("crate-disabled evidence");
        fs::write(
            tests.join("module_disabled.rs"),
            r#"
                #[cfg(any())]
                mod disabled {
                    #[test]
                    fn proof() { assert!(true); }
                }
            "#,
        )
        .expect("module-disabled evidence");
        fs::write(
            tests.join("module_platform.rs"),
            r#"
                #[cfg(unix)]
                mod platform {
                    #[test]
                    fn proof() { assert!(true); }
                }
            "#,
        )
        .expect("platform evidence");
        fs::write(
            tests.join("module_unknown.rs"),
            r#"
                #[cfg(unknown_evidence_target)]
                mod platform {
                    #[test]
                    fn proof() { assert!(true); }
                }
            "#,
        )
        .expect("unknown evidence");
        fs::write(
            tests.join("module_cfg_attr.rs"),
            r#"
                #[cfg_attr(any(), allow(dead_code))]
                mod platform {
                    #[test]
                    fn proof() { assert!(true); }
                }
            "#,
        )
        .expect("cfg-attr evidence");
        fs::write(
            tests.join("crate_platform.rs"),
            "#![cfg(unix)]\n#[test]\nfn proof() { assert!(true); }\n",
        )
        .expect("platform crate evidence");

        assert!(validate_test_evidence(&root, "boxdd/tests/crate_disabled.rs", "proof").is_err());
        assert!(
            validate_test_evidence(&root, "boxdd/tests/module_disabled.rs", "disabled::proof",)
                .is_err()
        );
        for file in [
            "boxdd/tests/module_platform.rs",
            "boxdd/tests/module_unknown.rs",
            "boxdd/tests/module_cfg_attr.rs",
        ] {
            assert!(
                validate_test_evidence(&root, file, "platform::proof").is_err(),
                "conditionally compiled ancestor `{file}` must fail",
            );
        }
        assert!(validate_test_evidence(&root, "boxdd/tests/crate_platform.rs", "proof").is_err());
        fs::remove_dir_all(root).expect("fixture cleanup");
    }

    #[test]
    fn xtask_source_evidence_must_be_reachable_from_the_library_test_target() {
        let root = fixture_root("xtask-source-evidence");
        let source = root.join("xtask/src");
        fs::create_dir_all(&source).expect("fixture source");
        fs::write(source.join("lib.rs"), "#[cfg(test)]\nmod abi;\n").expect("fixture lib");
        let proof = r#"
            #[cfg(test)]
            mod tests {
                #[test]
                fn proof() { assert!(true); }
            }
        "#;
        fs::write(source.join("abi.rs"), proof).expect("reachable source evidence");
        fs::write(source.join("orphan.rs"), proof).expect("orphan source evidence");

        validate_test_evidence(&root, "xtask/src/abi.rs", "tests::proof")
            .expect("reachable unit test evidence");
        assert!(validate_test_evidence(&root, "xtask/src/orphan.rs", "tests::proof").is_err());
        assert!(
            validate_test_evidence_for_gate(
                &root,
                "xtask/src/abi.rs",
                "tests::proof",
                "boxdd",
                "nextest",
            )
            .is_err()
        );
        assert!(
            validate_test_evidence_for_gate(
                &root,
                "xtask/src/abi.rs",
                "tests::proof",
                "xtask",
                "manual",
            )
            .is_err()
        );
        let discovered = discover_test_evidence_items(&root).expect("discover evidence tests");
        assert_eq!(
            discovered,
            vec![DiscoveredTestItem {
                file: "xtask/src/abi.rs".to_owned(),
                item: "tests::proof".to_owned(),
                package: "xtask".to_owned(),
                gate: "nextest".to_owned(),
            }]
        );
        fs::remove_dir_all(root).expect("fixture cleanup");
    }
}
