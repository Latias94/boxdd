use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    Error, Result,
    c_api::{CApiInventory, OverlayDecl},
    commands::api_coverage::Classification,
    commands::upstream_sync::{ArtifactProvider, ArtifactTarget, Precision, RustTarget},
    rust_index::RustIndex,
    sys_abi_index::{SysAbiAccessProjection, SysAbiAccessStep, SysAbiIndex},
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AbiBindingIndex {
    pub artifact: String,
    pub precision: Precision,
    pub target: ArtifactTarget,
    pub provider: ArtifactProvider,
    pub index: SysAbiIndex,
}

impl AbiBindingIndex {
    pub fn new(
        artifact: impl Into<String>,
        precision: Precision,
        target: ArtifactTarget,
        provider: ArtifactProvider,
        index: SysAbiIndex,
    ) -> Self {
        Self {
            artifact: artifact.into(),
            precision,
            target,
            provider,
            index,
        }
    }
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
                Ok(AbiTypeMapping {
                    mode: mode.clone(),
                    provider: provider.clone(),
                    path: path.clone(),
                    resolved_path: require_resolved_type(&binding.index, &path, &declaration.name)?,
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
                    Ok(field_mapping(mode, provider, &projection))
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
                    Ok(AbiTypeMapping {
                        mode: mode.clone(),
                        provider: provider.clone(),
                        path: path.clone(),
                        resolved_path: require_resolved_type(
                            &binding.index,
                            &path,
                            &declaration.name,
                        )?,
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
    let mut policies = generated
        .policies
        .drain(..)
        .map(|policy| (policy.id.clone(), policy))
        .collect::<BTreeMap<_, _>>();
    for policy in &previous.policies {
        policies.insert(policy.id.clone(), policy.clone());
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
        {
            copy_struct_exposure(structure, &previous_structure);
        } else {
            structure.rationale = format!(
                "The declaration fingerprint or header for `{}` changed, so the previous Safe review was not inherited and this refreshed capability is conservatively raw.",
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
                {
                    copy_field_exposure(field, &previous_field);
                } else {
                    field.rationale = format!(
                        "The declaration or overlay contract for `{}::{}` changed, so the previous Safe review was not inherited and this refreshed field is conservatively raw.",
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
            {
                callback.rationale = previous_callback.rationale;
                callback.policy = previous_callback.policy;
                callback.safe_paths = previous_callback.safe_paths;
                callback.safe_witnesses = previous_callback.safe_witnesses;
            } else {
                callback.rationale = format!(
                    "The declaration fingerprint, signature, or header for `{}` changed, so the previous Safe review was not inherited and this refreshed callback is conservatively raw.",
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
            binding_routes,
            binding_indexes,
            function_symbols,
            rust_indexes,
            evidence_ids,
        }
    }
}

pub fn validate(
    contract: &AbiContract,
    context: &AbiValidationContext<'_>,
    errors: &mut Vec<String>,
) {
    validate_binding_routes(context.binding_routes, context.binding_indexes, errors);
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
    validate_structs(contract, context, &policies, &mut used_policies, errors);
    validate_callbacks(contract, context, &policies, &mut used_policies, errors);
    for policy in policies.keys() {
        if !used_policies.contains(*policy) {
            errors.push(format!("ABI policy `{policy}` is unused"));
        }
    }
    validate_referenced_binding_functions(
        context.inventory,
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
        validate_type_mappings(
            &format!("ABI struct `{}`", structure.name),
            &structure.name,
            &structure.raw_mappings,
            policy,
            context.binding_routes,
            context.binding_indexes,
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
        validate_type_mappings(
            &subject,
            &callback.name,
            &callback.raw_mappings,
            policy,
            context.binding_routes,
            context.binding_indexes,
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

fn validate_type_mappings(
    subject: &str,
    c_name: &str,
    mappings: &[AbiTypeMapping],
    policy: Option<&AbiCapabilityPolicy>,
    binding_routes: &AbiBindingRoutes,
    binding_indexes: &AbiBindingIndexes,
    errors: &mut Vec<String>,
) {
    validate_mapping_coordinates(subject, mappings, policy, errors);
    let expected_path = type_path(c_name);
    for mapping in mappings {
        let Some(index) = mapping_binding_index(
            subject,
            &mapping.mode,
            &mapping.provider,
            binding_routes,
            binding_indexes,
            errors,
        ) else {
            continue;
        };
        let expected_resolved = match index.resolved_type_path(&expected_path) {
            Ok(Some(path)) => Some(path),
            Ok(None) => {
                errors.push(format!(
                    "{subject} expected generated type path `{expected_path}` is absent from binding artifact `{}`",
                    route_artifact(&mapping.mode, &mapping.provider, binding_routes)
                ));
                None
            }
            Err(error) => {
                errors.push(format!(
                    "{subject} in binding artifact `{}`: {error}",
                    route_artifact(&mapping.mode, &mapping.provider, binding_routes)
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
            Ok(projection) => Some(field_mapping("", "", &projection)),
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
}

impl MappingCoordinate for AbiTypeMapping {
    fn mode(&self) -> &str {
        &self.mode
    }

    fn provider(&self) -> &str {
        &self.provider
    }
}

impl MappingCoordinate for AbiFieldMapping {
    fn mode(&self) -> &str {
        &self.mode
    }

    fn provider(&self) -> &str {
        &self.provider
    }
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
    }
}

fn validate_referenced_binding_functions(
    inventory: &CApiInventory,
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
    use super::declaration_mentions_identifier;

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
}
