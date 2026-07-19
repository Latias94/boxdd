use std::{fmt::Write as _, path::Path};

use crate::{
    Error, Result,
    c_api::{
        AbiFieldShape, AbiTypeShape, CAbiPrecision, CApiInventory, PrecisionCApiInventory,
        parse_headers, parse_headers_for_precision,
    },
    sys_abi_index::{SysAbiAccessProjection, SysAbiIndex, index_bindings},
};

/// Native Box2D ABI precision selected for one probe build.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AbiProbePrecision {
    Single,
    Double,
}

impl AbiProbePrecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Double => "double",
        }
    }

    const fn c_precision(self) -> CAbiPrecision {
        match self {
            Self::Single => CAbiPrecision::Single,
            Self::Double => CAbiPrecision::Double,
        }
    }

    const fn bindings_file(self) -> &'static str {
        match self {
            Self::Single => "bindings_pregenerated.rs",
            Self::Double => "bindings_double.rs",
        }
    }
}

/// Generated, target-independent source for one native ABI probe route.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedAbiProbe {
    pub c_source: String,
    pub mixed_precision_c_source: String,
    pub rust_source: String,
    pub struct_count: usize,
    pub field_count: usize,
    pub layout_case_count: usize,
    pub symbol_count: usize,
    pub callback_count: usize,
    pub callable_callback_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LayoutCase {
    label: String,
    c_expression: String,
    rust_expression: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ProbeSpec {
    precision: AbiProbePrecision,
    layouts: Vec<LayoutCase>,
    struct_count: usize,
    field_count: usize,
    physical_symbols: Vec<String>,
    callbacks: Vec<CallbackProbeCase>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CallbackProbeCase {
    name: String,
    kind: CallbackProbeKind,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum CallbackProbeKind {
    Alloc,
    Free,
    Assert,
    Log,
    Task,
    EnqueueTask,
    FinishTask,
    Friction,
    Restitution,
    TreeQuery,
    TreeRayCast,
    TreeBoxCast,
    CustomFilter,
    PreSolve,
    OverlapResult,
    CastResult,
    PlaneResult,
}

impl CallbackProbeKind {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "b2AllocFcn" => Some(Self::Alloc),
            "b2FreeFcn" => Some(Self::Free),
            "b2AssertFcn" => Some(Self::Assert),
            "b2LogFcn" => Some(Self::Log),
            "b2TaskCallback" => Some(Self::Task),
            "b2EnqueueTaskCallback" => Some(Self::EnqueueTask),
            "b2FinishTaskCallback" => Some(Self::FinishTask),
            "b2FrictionCallback" => Some(Self::Friction),
            "b2RestitutionCallback" => Some(Self::Restitution),
            "b2TreeQueryCallbackFcn" => Some(Self::TreeQuery),
            "b2TreeRayCastCallbackFcn" => Some(Self::TreeRayCast),
            "b2TreeBoxCastCallbackFcn" => Some(Self::TreeBoxCast),
            "b2CustomFilterFcn" => Some(Self::CustomFilter),
            "b2PreSolveFcn" => Some(Self::PreSolve),
            "b2OverlapResultFcn" => Some(Self::OverlapResult),
            "b2CastResultFcn" => Some(Self::CastResult),
            "b2PlaneResultFcn" => Some(Self::PlaneResult),
            _ => None,
        }
    }

    const fn suffix(self) -> &'static str {
        match self {
            Self::Alloc => "alloc",
            Self::Free => "free",
            Self::Assert => "assert",
            Self::Log => "log",
            Self::Task => "task",
            Self::EnqueueTask => "enqueue_task",
            Self::FinishTask => "finish_task",
            Self::Friction => "friction",
            Self::Restitution => "restitution",
            Self::TreeQuery => "tree_query",
            Self::TreeRayCast => "tree_ray_cast",
            Self::TreeBoxCast => "tree_box_cast",
            Self::CustomFilter => "custom_filter",
            Self::PreSolve => "pre_solve",
            Self::OverlapResult => "overlap_result",
            Self::CastResult => "cast_result",
            Self::PlaneResult => "plane_result",
        }
    }

    const fn rust_probe_function(self) -> &'static str {
        match self {
            Self::Alloc => "probe_alloc_callback",
            Self::Free => "probe_free_callback",
            Self::Assert => "probe_assert_callback",
            Self::Log => "probe_log_callback",
            Self::Task => "probe_task_callback",
            Self::EnqueueTask => "probe_enqueue_task_callback",
            Self::FinishTask => "probe_finish_task_callback",
            Self::Friction => "probe_friction_callback",
            Self::Restitution => "probe_restitution_callback",
            Self::TreeQuery => "probe_tree_query_callback",
            Self::TreeRayCast => "probe_tree_ray_cast_callback",
            Self::TreeBoxCast => "probe_tree_box_cast_callback",
            Self::CustomFilter => "probe_custom_filter_callback",
            Self::PreSolve => "probe_pre_solve_callback",
            Self::OverlapResult => "probe_overlap_result_callback",
            Self::CastResult => "probe_cast_result_callback",
            Self::PlaneResult => "probe_plane_result_callback",
        }
    }

    const fn rust_ffi_declaration(self) -> &'static str {
        match self {
            Self::Alloc => {
                "    fn boxdd_abi_probe_invoke_alloc(callback: ffi::b2AllocFcn) -> bool;"
            }
            Self::Free => "    fn boxdd_abi_probe_invoke_free(callback: ffi::b2FreeFcn) -> bool;",
            Self::Assert => {
                "    fn boxdd_abi_probe_invoke_assert(callback: ffi::b2AssertFcn) -> i32;"
            }
            Self::Log => "    fn boxdd_abi_probe_invoke_log(callback: ffi::b2LogFcn) -> bool;",
            Self::Task => {
                "    fn boxdd_abi_probe_invoke_task(callback: ffi::b2TaskCallback, context: *mut c_void) -> bool;"
            }
            Self::EnqueueTask => {
                "    fn boxdd_abi_probe_invoke_enqueue_task(callback: ffi::b2EnqueueTaskCallback, context: *mut c_void) -> u32;"
            }
            Self::FinishTask => {
                "    fn boxdd_abi_probe_invoke_finish_task(callback: ffi::b2FinishTaskCallback, context: *mut c_void) -> bool;"
            }
            Self::Friction => {
                "    fn boxdd_abi_probe_invoke_friction(callback: ffi::b2FrictionCallback) -> f32;"
            }
            Self::Restitution => {
                "    fn boxdd_abi_probe_invoke_restitution(callback: ffi::b2RestitutionCallback) -> f32;"
            }
            Self::TreeQuery => {
                "    fn boxdd_abi_probe_invoke_tree_query(callback: ffi::b2TreeQueryCallbackFcn, context: *mut c_void) -> bool;"
            }
            Self::TreeRayCast => {
                "    fn boxdd_abi_probe_invoke_tree_ray_cast(callback: ffi::b2TreeRayCastCallbackFcn, context: *mut c_void) -> f32;"
            }
            Self::TreeBoxCast => {
                "    fn boxdd_abi_probe_invoke_tree_box_cast(callback: ffi::b2TreeBoxCastCallbackFcn, context: *mut c_void) -> f32;"
            }
            Self::CustomFilter => {
                "    fn boxdd_abi_probe_invoke_custom_filter(callback: ffi::b2CustomFilterFcn, context: *mut c_void) -> bool;"
            }
            Self::PreSolve => {
                "    fn boxdd_abi_probe_invoke_pre_solve(callback: ffi::b2PreSolveFcn, context: *mut c_void) -> bool;"
            }
            Self::OverlapResult => {
                "    fn boxdd_abi_probe_invoke_overlap_result(callback: ffi::b2OverlapResultFcn, context: *mut c_void) -> bool;"
            }
            Self::CastResult => {
                "    fn boxdd_abi_probe_invoke_cast_result(callback: ffi::b2CastResultFcn, context: *mut c_void) -> f32;"
            }
            Self::PlaneResult => {
                "    fn boxdd_abi_probe_invoke_plane_result(callback: ffi::b2PlaneResultFcn, context: *mut c_void) -> bool;"
            }
        }
    }
}

/// Generate the active workspace probe for one precision route.
pub fn generate_workspace_probe(
    workspace_root: &Path,
    precision: AbiProbePrecision,
) -> Result<GeneratedAbiProbe> {
    let include_dir = workspace_root.join("boxdd-sys/third-party/box2d/include/box2d");
    let bindings = workspace_root
        .join("boxdd-sys/src")
        .join(precision.bindings_file());
    generate_probe(&include_dir, &bindings, precision)
}

fn generate_probe(
    include_dir: &Path,
    bindings: &Path,
    precision: AbiProbePrecision,
) -> Result<GeneratedAbiProbe> {
    let inventory = parse_headers(include_dir)?;
    let precision_inventory = parse_headers_for_precision(include_dir, precision.c_precision())?;
    let rust_index = index_bindings(bindings)?;
    let spec = build_probe_spec(&inventory, &precision_inventory, &rust_index, precision)?;
    let c_source = render_c_source(&spec)?;
    let mixed_precision_c_source = render_mixed_precision_c_source(precision);
    let rust_source = render_rust_source(&spec)?;
    Ok(GeneratedAbiProbe {
        c_source,
        mixed_precision_c_source,
        rust_source,
        struct_count: spec.struct_count,
        field_count: spec.field_count,
        layout_case_count: spec.layouts.len(),
        symbol_count: spec.physical_symbols.len(),
        callback_count: spec.callbacks.len(),
        callable_callback_count: spec.callbacks.len(),
    })
}

fn build_probe_spec(
    inventory: &CApiInventory,
    precision_inventory: &PrecisionCApiInventory,
    rust_index: &SysAbiIndex,
    precision: AbiProbePrecision,
) -> Result<ProbeSpec> {
    if precision_inventory.precision != precision.c_precision() {
        return Err(Error::message(format!(
            "ABI probe requested {} precision but the effective C inventory reports {}",
            precision.as_str(),
            precision_inventory.precision.as_str()
        )));
    }

    let mut layouts = Vec::new();
    let mut field_count = 0;
    for structure in &inventory.structs {
        let c_shape = precision_inventory
            .type_shape(&structure.name)
            .ok_or_else(|| {
                Error::message(format!(
                    "ABI probe structure `{}` has no effective {} C type",
                    structure.name,
                    precision.as_str()
                ))
            })?;
        let AbiTypeShape::Aggregate { fields } = c_shape else {
            return Err(Error::message(format!(
                "ABI probe structure `{}` does not resolve to an aggregate in {} precision",
                structure.name,
                precision.as_str()
            )));
        };
        let rust_path = format!("boxdd_sys::ffi::{}", structure.name);
        let rust_shape = rust_index.type_abi_shape(&rust_path)?.ok_or_else(|| {
            Error::message(format!(
                "ABI probe structure `{}` is absent from {} bindings",
                structure.name,
                precision.as_str()
            ))
        })?;
        let direct_shape_matches = c_shape.fingerprint() == rust_shape.fingerprint();

        layouts.push(LayoutCase {
            label: format!("sizeof({})", structure.name),
            c_expression: format!("sizeof({})", structure.name),
            rust_expression: format!("::std::mem::size_of::<{rust_path}>() as u64"),
        });
        layouts.push(LayoutCase {
            label: format!("alignof({})", structure.name),
            c_expression: format!("BOXDD_ABI_ALIGNOF({})", structure.name),
            rust_expression: format!("::std::mem::align_of::<{rust_path}>() as u64"),
        });

        let mut projected_c_fields = Vec::with_capacity(structure.fields.len());
        let mut projected_rust_fields = Vec::with_capacity(structure.fields.len());
        for field in &structure.fields {
            let c_field = require_effective_field(&structure.name, fields, &field.name, precision)?;
            let projection = require_field_projection(rust_index, &rust_path, &field.name)?;
            let rust_field_shape =
                rust_index
                    .field_access_abi_shape(&projection)?
                    .ok_or_else(|| {
                        Error::message(format!(
                            "ABI probe field `{}::{}` has no generated Rust ABI shape",
                            structure.name, field.name
                        ))
                    })?;
            require_matching_fingerprint(
                &format!("field `{}::{}`", structure.name, field.name),
                &c_field.shape,
                &rust_field_shape,
                precision,
            )?;
            projected_c_fields.push(AbiFieldShape {
                name: field.name.clone(),
                shape: c_field.shape.clone(),
                overlays: field.overlays.clone(),
            });
            projected_rust_fields.push(AbiFieldShape {
                name: field.name.clone(),
                shape: rust_field_shape,
                overlays: field.overlays.clone(),
            });
            layouts.push(LayoutCase {
                label: format!("offsetof({},{})", structure.name, field.name),
                c_expression: format!("offsetof({}, {})", structure.name, field.name),
                rust_expression: rust_offset_expression(&projection)?,
            });
            field_count += 1;
        }
        if !direct_shape_matches {
            require_matching_fingerprint(
                &format!("projected structure `{}`", structure.name),
                &AbiTypeShape::Aggregate {
                    fields: projected_c_fields,
                },
                &AbiTypeShape::Aggregate {
                    fields: projected_rust_fields,
                },
                precision,
            )?;
        }
    }

    let mut physical_symbols = Vec::with_capacity(inventory.functions.len());
    for function in &inventory.functions {
        let symbol = function
            .physical_symbols
            .get(precision.as_str())
            .ok_or_else(|| {
                Error::message(format!(
                    "ABI probe function `{}` has no {} physical symbol",
                    function.name,
                    precision.as_str()
                ))
            })?;
        let c_function = precision_inventory
            .function(&function.name)
            .ok_or_else(|| {
                Error::message(format!(
                    "ABI probe function `{}` is absent from the effective {} C inventory",
                    function.name,
                    precision.as_str()
                ))
            })?;
        let rust_path = format!("boxdd_sys::ffi::{symbol}");
        let rust_fingerprint = rust_index
            .function_abi_fingerprint(&rust_path)?
            .ok_or_else(|| {
                Error::message(format!(
                    "ABI probe physical symbol `{symbol}` is absent from {} bindings",
                    precision.as_str()
                ))
            })?;
        if rust_fingerprint != c_function.fingerprint {
            return Err(Error::message(format!(
                "ABI probe function `{}` has mismatched {} C/Rust fingerprints: C `{}`, Rust `{rust_fingerprint}`",
                function.name,
                precision.as_str(),
                c_function.fingerprint
            )));
        }
        physical_symbols.push(symbol.clone());
    }

    let mut callbacks = Vec::with_capacity(inventory.callbacks.len());
    for callback in &inventory.callbacks {
        let c_callback = precision_inventory
            .callback(&callback.name)
            .ok_or_else(|| {
                Error::message(format!(
                    "ABI probe callback `{}` is absent from the effective {} C inventory",
                    callback.name,
                    precision.as_str()
                ))
            })?;
        let rust_path = format!("boxdd_sys::ffi::{}", callback.name);
        let rust_fingerprint = rust_index
            .callback_abi_fingerprint(&rust_path)?
            .ok_or_else(|| {
                Error::message(format!(
                    "ABI probe callback `{}` is absent from {} bindings",
                    callback.name,
                    precision.as_str()
                ))
            })?;
        if rust_fingerprint != c_callback.fingerprint {
            return Err(Error::message(format!(
                "ABI probe callback `{}` has mismatched {} C/Rust fingerprints: C `{}`, Rust `{rust_fingerprint}`",
                callback.name,
                precision.as_str(),
                c_callback.fingerprint
            )));
        }
        let kind = CallbackProbeKind::from_name(&callback.name).ok_or_else(|| {
            Error::message(format!(
                "ABI probe callback `{}` has no executable C-to-Rust probe route",
                callback.name
            ))
        })?;
        callbacks.push(CallbackProbeCase {
            name: callback.name.clone(),
            kind,
        });
    }

    Ok(ProbeSpec {
        precision,
        layouts,
        struct_count: inventory.structs.len(),
        field_count,
        physical_symbols,
        callbacks,
    })
}

fn require_effective_field<'a>(
    structure: &str,
    fields: &'a [AbiFieldShape],
    field_name: &str,
    precision: AbiProbePrecision,
) -> Result<&'a AbiFieldShape> {
    fields
        .iter()
        .find(|field| field.name == field_name)
        .ok_or_else(|| {
            Error::message(format!(
                "ABI probe field `{structure}::{field_name}` is absent from the effective {} C aggregate",
                precision.as_str()
            ))
        })
}

fn require_matching_fingerprint(
    subject: &str,
    c_shape: &AbiTypeShape,
    rust_shape: &AbiTypeShape,
    precision: AbiProbePrecision,
) -> Result<()> {
    let c_fingerprint = c_shape.fingerprint();
    let rust_fingerprint = rust_shape.fingerprint();
    if c_fingerprint == rust_fingerprint {
        return Ok(());
    }
    Err(Error::message(format!(
        "ABI probe {subject} has mismatched {} C/Rust fingerprints: C `{c_fingerprint}`, Rust `{rust_fingerprint}`",
        precision.as_str()
    )))
}

fn require_field_projection(
    index: &SysAbiIndex,
    root_path: &str,
    c_field: &str,
) -> Result<SysAbiAccessProjection> {
    let segments = c_field.split('.').collect::<Vec<_>>();
    if let Some(projection) = index.project_field_access(root_path, &segments)? {
        return Ok(projection);
    }
    let Some((last, prefix)) = segments.split_last() else {
        return Err(Error::message("ABI probe field path cannot be empty"));
    };
    let escaped_last = format!("{last}_");
    let mut escaped = prefix.to_vec();
    escaped.push(&escaped_last);
    index
        .project_field_access(root_path, &escaped)?
        .ok_or_else(|| {
            Error::message(format!(
                "ABI probe field `{root_path}::{c_field}` has no unique generated Rust access chain"
            ))
        })
}

fn rust_offset_expression(projection: &SysAbiAccessProjection) -> Result<String> {
    if projection.steps.is_empty() {
        return Err(Error::message(format!(
            "ABI probe projection for `{}` has no field steps",
            projection.root_type
        )));
    }
    Ok(projection
        .steps
        .iter()
        .map(|step| {
            format!(
                "::std::mem::offset_of!({}, {}) as u64",
                step.owner_type, step.field
            )
        })
        .collect::<Vec<_>>()
        .join(" + "))
}

fn render_c_source(spec: &ProbeSpec) -> Result<String> {
    if spec.layouts.is_empty() || spec.physical_symbols.is_empty() {
        return Err(Error::message(
            "ABI probe cannot render an empty layout or symbol inventory",
        ));
    }
    let mut output = String::new();
    output.push_str(&precision_prelude(spec.precision));
    output.push_str("#include <box2d/box2d.h>\n#include <stddef.h>\n#include <stdint.h>\n\n");
    output.push_str(
        "#if defined(_MSC_VER)\n#define BOXDD_ABI_ALIGNOF(type) __alignof(type)\n#else\n#define BOXDD_ABI_ALIGNOF(type) _Alignof(type)\n#endif\n\n",
    );
    output.push_str(
        "_Static_assert(sizeof(uint32_t) == 4, \"ABI probe requires 32-bit uint32_t\");\n_Static_assert(sizeof(uint64_t) == 8, \"ABI probe requires 64-bit uint64_t\");\n\n",
    );

    output.push_str(
        "uint64_t boxdd_abi_probe_layout_value(uint32_t index)\n{\n\tswitch (index)\n\t{\n",
    );
    for (index, case) in spec.layouts.iter().enumerate() {
        writeln!(
            output,
            "\t\tcase {index}u: return (uint64_t)({});",
            case.c_expression
        )
        .expect("write to string");
    }
    output.push_str("\t\tdefault: return UINT64_MAX;\n\t}\n}\n\n");

    for (index, symbol) in spec.physical_symbols.iter().enumerate() {
        writeln!(
            output,
            "static void (*volatile boxdd_abi_symbol_{index})(void) = (void (*)(void))&{symbol};"
        )
        .expect("write to string");
    }
    output.push_str("\nuint32_t boxdd_abi_probe_link_all(void)\n{\n\tuint32_t count = 0;\n");
    for index in 0..spec.physical_symbols.len() {
        writeln!(
            output,
            "\tcount += boxdd_abi_symbol_{index} != (void (*)(void))0;"
        )
        .expect("write to string");
    }
    output.push_str("\treturn count;\n}\n\n");

    for (index, callback) in spec.callbacks.iter().enumerate() {
        let callback_name = &callback.name;
        writeln!(
            output,
            "static {callback_name}* volatile boxdd_abi_callback_{index} = ({callback_name}*)0;"
        )
        .expect("write to string");
    }
    output.push_str(
        "\nuint32_t boxdd_abi_probe_callback_type_count(void)\n{\n\tuint32_t count = 0;\n",
    );
    for (index, callback) in spec.callbacks.iter().enumerate() {
        let callback_name = &callback.name;
        writeln!(
            output,
            "\tcount += boxdd_abi_callback_{index} == ({callback_name}*)0;"
        )
        .expect("write to string");
    }
    output.push_str("\treturn count;\n}\n\n");

    for callback in &spec.callbacks {
        writeln!(
            output,
            "/* executable callback probe: {} ({}) */",
            callback.name,
            callback.kind.suffix()
        )
        .expect("write to string");
        render_c_callback_invoker(&mut output, callback.kind);
    }
    output.push_str(
        "b2Version boxdd_abi_probe_get_version(void)\n{\n\treturn b2GetVersion();\n}\n\n",
    );
    writeln!(
        output,
        "bool boxdd_abi_probe_precision_matches(void)\n{{\n\treturn b2IsDoublePrecision() == {};\n}}",
        c_bool(spec.precision == AbiProbePrecision::Double)
    )
    .expect("write to string");
    Ok(output)
}

fn render_c_callback_invoker(output: &mut String, kind: CallbackProbeKind) {
    let source = match kind {
        CallbackProbeKind::Alloc => {
            r#"bool boxdd_abi_probe_invoke_alloc(b2AllocFcn* callback)
{
	if (callback == NULL)
	{
		return false;
	}
	void* memory = callback((size_t)37, 16);
	return memory != NULL && ((uintptr_t)memory % (uintptr_t)16) == (uintptr_t)0;
}

"#
        }
        CallbackProbeKind::Free => {
            r#"bool boxdd_abi_probe_invoke_free(b2FreeFcn* callback)
{
	if (callback == NULL)
	{
		return false;
	}
	uint64_t memory = UINT64_C(0x123456789ABCDEF0);
	callback(&memory, (size_t)41);
	return memory == UINT64_C(0x123456789ABCDEF0);
}

"#
        }
        CallbackProbeKind::Assert => {
            r#"int boxdd_abi_probe_invoke_assert(b2AssertFcn* callback)
{
	if (callback == NULL)
	{
		return -1;
	}
	return callback("boxdd-condition", "boxdd-file.c", 73);
}

"#
        }
        CallbackProbeKind::Log => {
            r#"bool boxdd_abi_probe_invoke_log(b2LogFcn* callback)
{
	if (callback == NULL)
	{
		return false;
	}
	callback("boxdd-log-message");
	return true;
}

"#
        }
        CallbackProbeKind::Task => {
            r#"bool boxdd_abi_probe_invoke_task(b2TaskCallback* callback, void* context)
{
	if (callback == NULL || context == NULL)
	{
		return false;
	}
	callback(context);
	return true;
}

"#
        }
        CallbackProbeKind::EnqueueTask => {
            r#"static void boxdd_abi_probe_nested_task(void* taskContext)
{
	uint32_t* calls = (uint32_t*)taskContext;
	*calls += 1u;
}

uint32_t boxdd_abi_probe_invoke_enqueue_task(b2EnqueueTaskCallback* callback, void* context)
{
	if (callback == NULL || context == NULL)
	{
		return 0u;
	}
	uint32_t nestedCalls = 0u;
	void* result = callback(boxdd_abi_probe_nested_task, &nestedCalls, context);
	uint32_t status = 0u;
	status |= nestedCalls == 1u ? 1u : 0u;
	status |= result == NULL ? 2u : 0u;
	return status;
}

"#
        }
        CallbackProbeKind::FinishTask => {
            r#"bool boxdd_abi_probe_invoke_finish_task(b2FinishTaskCallback* callback, void* context)
{
	if (callback == NULL || context == NULL)
	{
		return false;
	}
	uint64_t userTask = UINT64_C(0xA5A55A5AF0F00F0F);
	callback(&userTask, context);
	return userTask == UINT64_C(0xA5A55A5AF0F00F0F);
}

"#
        }
        CallbackProbeKind::Friction => {
            r#"float boxdd_abi_probe_invoke_friction(b2FrictionCallback* callback)
{
	if (callback == NULL)
	{
		return -1.0f;
	}
	return callback(0.25f, UINT64_C(101), 0.75f, UINT64_C(202));
}

"#
        }
        CallbackProbeKind::Restitution => {
            r#"float boxdd_abi_probe_invoke_restitution(b2RestitutionCallback* callback)
{
	if (callback == NULL)
	{
		return -1.0f;
	}
	return callback(0.125f, UINT64_C(303), 0.875f, UINT64_C(404));
}

"#
        }
        CallbackProbeKind::TreeQuery => {
            r#"bool boxdd_abi_probe_invoke_tree_query(b2TreeQueryCallbackFcn* callback, void* context)
{
	if (callback == NULL || context == NULL)
	{
		return false;
	}
	return callback(7, UINT64_C(11), context);
}

"#
        }
        CallbackProbeKind::TreeRayCast => {
            r#"float boxdd_abi_probe_invoke_tree_ray_cast(b2TreeRayCastCallbackFcn* callback, void* context)
{
	if (callback == NULL || context == NULL)
	{
		return -1.0f;
	}
	b2RayCastInput input = {0};
	input.origin.x = 1.25f;
	input.origin.y = -2.5f;
	input.translation.x = 3.5f;
	input.translation.y = -4.5f;
	input.maxFraction = 0.75f;
	return callback(&input, 13, UINT64_C(17), context);
}

"#
        }
        CallbackProbeKind::TreeBoxCast => {
            r#"float boxdd_abi_probe_invoke_tree_box_cast(b2TreeBoxCastCallbackFcn* callback, void* context)
{
	if (callback == NULL || context == NULL)
	{
		return -1.0f;
	}
	b2BoxCastInput input = {0};
	input.box.lowerBound.x = -1.25f;
	input.box.lowerBound.y = -2.25f;
	input.box.upperBound.x = 3.25f;
	input.box.upperBound.y = 4.25f;
	input.translation.x = 5.5f;
	input.translation.y = -6.5f;
	input.maxFraction = 0.875f;
	return callback(&input, 19, UINT64_C(23), context);
}

"#
        }
        CallbackProbeKind::CustomFilter => {
            r#"bool boxdd_abi_probe_invoke_custom_filter(b2CustomFilterFcn* callback, void* context)
{
	if (callback == NULL || context == NULL)
	{
		return false;
	}
	b2ShapeId shapeA = {0};
	shapeA.index1 = 29;
	shapeA.world0 = 31;
	shapeA.generation = 37;
	b2ShapeId shapeB = {0};
	shapeB.index1 = 41;
	shapeB.world0 = 43;
	shapeB.generation = 47;
	return callback(shapeA, shapeB, context);
}

"#
        }
        CallbackProbeKind::PreSolve => {
            r#"bool boxdd_abi_probe_invoke_pre_solve(b2PreSolveFcn* callback, void* context)
{
	if (callback == NULL || context == NULL)
	{
		return false;
	}
	b2ShapeId shapeA = {0};
	shapeA.index1 = 53;
	shapeA.world0 = 59;
	shapeA.generation = 61;
	b2ShapeId shapeB = {0};
	shapeB.index1 = 67;
	shapeB.world0 = 71;
	shapeB.generation = 73;
	b2Pos point = {0};
	point.x = 1.25;
	point.y = -2.5;
	b2Vec2 normal = {0};
	normal.x = 0.0f;
	normal.y = 1.0f;
	return callback(shapeA, shapeB, point, normal, context);
}

"#
        }
        CallbackProbeKind::OverlapResult => {
            r#"bool boxdd_abi_probe_invoke_overlap_result(b2OverlapResultFcn* callback, void* context)
{
	if (callback == NULL || context == NULL)
	{
		return false;
	}
	b2ShapeId shape = {0};
	shape.index1 = 79;
	shape.world0 = 83;
	shape.generation = 89;
	return callback(shape, context);
}

"#
        }
        CallbackProbeKind::CastResult => {
            r#"float boxdd_abi_probe_invoke_cast_result(b2CastResultFcn* callback, void* context)
{
	if (callback == NULL || context == NULL)
	{
		return -1.0f;
	}
	b2ShapeId shape = {0};
	shape.index1 = 97;
	shape.world0 = 101;
	shape.generation = 103;
	b2Pos point = {0};
	point.x = 2.25;
	point.y = -3.5;
	b2Vec2 normal = {0};
	normal.x = -0.5f;
	normal.y = 1.0f;
	return callback(shape, point, normal, 0.375f, context);
}

"#
        }
        CallbackProbeKind::PlaneResult => {
            r#"bool boxdd_abi_probe_invoke_plane_result(b2PlaneResultFcn* callback, void* context)
{
	if (callback == NULL || context == NULL)
	{
		return false;
	}
	b2ShapeId shape = {0};
	shape.index1 = 107;
	shape.world0 = 109;
	shape.generation = 113;
	b2PlaneResult plane = {0};
	plane.plane.normal.x = 0.0f;
	plane.plane.normal.y = 1.0f;
	plane.plane.offset = 1.25f;
	plane.point.x = -2.5f;
	plane.point.y = 3.75f;
	plane.hit = true;
	return callback(shape, &plane, context);
}

"#
        }
    };
    output.push_str(source);
}

fn render_mixed_precision_c_source(precision: AbiProbePrecision) -> String {
    let opposite = match precision {
        AbiProbePrecision::Single => AbiProbePrecision::Double,
        AbiProbePrecision::Double => AbiProbePrecision::Single,
    };
    format!(
        "{}#include <box2d/box2d.h>\n\nbool boxdd_abi_probe_mixed_precision_matches(void)\n{{\n\treturn b2IsDoublePrecision() == {};\n}}\n",
        precision_prelude(opposite),
        c_bool(opposite == AbiProbePrecision::Double)
    )
}

fn render_rust_source(spec: &ProbeSpec) -> Result<String> {
    if spec.layouts.is_empty() {
        return Err(Error::message("ABI probe has no Rust layout cases"));
    }
    let mut output = String::new();
    output.push_str("pub(crate) const ABI_PROBE_LAYOUT_CASES: &[(&str, u64)] = &[\n");
    for case in &spec.layouts {
        writeln!(output, "    ({:?}, {}),", case.label, case.rust_expression)
            .expect("write to string");
    }
    output.push_str("];\n");
    writeln!(
        output,
        "pub(crate) const ABI_PROBE_STRUCT_COUNT: usize = {};",
        spec.struct_count
    )
    .expect("write to string");
    writeln!(
        output,
        "pub(crate) const ABI_PROBE_FIELD_COUNT: usize = {};",
        spec.field_count
    )
    .expect("write to string");
    writeln!(
        output,
        "pub(crate) const ABI_PROBE_SYMBOL_COUNT: u32 = {};",
        spec.physical_symbols.len()
    )
    .expect("write to string");
    writeln!(
        output,
        "pub(crate) const ABI_PROBE_CALLBACK_COUNT: u32 = {};",
        spec.callbacks.len()
    )
    .expect("write to string");
    writeln!(
        output,
        "pub(crate) const ABI_PROBE_CALLABLE_CALLBACK_COUNT: u32 = {};",
        spec.callbacks.len()
    )
    .expect("write to string");
    output.push_str("pub(crate) const ABI_PROBE_CALLBACK_NAMES: &[&str] = &[\n");
    for callback in &spec.callbacks {
        writeln!(output, "    {:?},", callback.name).expect("write to string");
    }
    output.push_str("];\n");
    output.push_str("unsafe extern \"C\" {\n");
    for callback in &spec.callbacks {
        writeln!(output, "{}", callback.kind.rust_ffi_declaration()).expect("write to string");
    }
    output.push_str("}\n");
    output.push_str(
        "pub(crate) fn generated_callback_probe_results() -> Vec<CallbackProbeResult> {\n    vec![\n",
    );
    for callback in &spec.callbacks {
        writeln!(output, "        {}(),", callback.kind.rust_probe_function())
            .expect("write to string");
    }
    output.push_str("    ]\n}\n");
    writeln!(
        output,
        "pub(crate) const ABI_PROBE_IS_DOUBLE: bool = {};",
        spec.precision == AbiProbePrecision::Double
    )
    .expect("write to string");
    Ok(output)
}

fn precision_prelude(precision: AbiProbePrecision) -> String {
    let mut output = String::from(
        "#if defined(BOX2D_DOUBLE_PRECISION)\n#undef BOX2D_DOUBLE_PRECISION\n#endif\n",
    );
    if precision == AbiProbePrecision::Double {
        output.push_str("#define BOX2D_DOUBLE_PRECISION 1\n");
    }
    output
}

const fn c_bool(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, fs, path::PathBuf};

    use super::{AbiProbePrecision, CallbackProbeKind, generate_probe};

    const HEADER: &str = r#"
typedef struct b2Sample
{
    int type;
    float value;
} b2Sample;
typedef bool b2TreeQueryCallbackFcn(int proxyId, uint64_t userData, void* context);
B2_API int b2Touch(b2Sample sample);
"#;

    const BINDINGS: &str = r#"
#[repr(C)]
pub struct b2Sample {
    pub type_: ::std::os::raw::c_int,
    pub value: f32,
}
pub type b2TreeQueryCallbackFcn = ::std::option::Option<
    unsafe extern "C" fn(
        proxyId: ::std::os::raw::c_int,
        userData: u64,
        context: *mut ::std::os::raw::c_void,
    ) -> bool,
>;
unsafe extern "C" {
    pub fn b2Touch(sample: b2Sample) -> ::std::os::raw::c_int;
}
"#;

    fn fixture(header: &str, bindings: &str) -> (tempfile::TempDir, PathBuf, PathBuf) {
        let root = tempfile::tempdir().expect("temporary ABI probe fixture");
        let include = root.path().join("include");
        fs::create_dir(&include).expect("fixture include directory");
        fs::write(include.join("box2d.h"), header).expect("fixture C header");
        let bindings_path = root.path().join("bindings.rs");
        fs::write(&bindings_path, bindings).expect("fixture Rust bindings");
        (root, include, bindings_path)
    }

    #[test]
    fn generator_covers_every_structure_field_symbol_and_callback() {
        let (_root, include, bindings) = fixture(HEADER, BINDINGS);
        let generated = generate_probe(&include, &bindings, AbiProbePrecision::Single)
            .expect("valid ABI probe fixture");

        assert_eq!(generated.struct_count, 1);
        assert_eq!(generated.field_count, 2);
        assert_eq!(generated.layout_case_count, 4);
        assert_eq!(generated.symbol_count, 1);
        assert_eq!(generated.callback_count, 1);
        assert_eq!(generated.callable_callback_count, 1);
        assert!(generated.c_source.contains("offsetof(b2Sample, type)"));
        assert!(
            generated
                .rust_source
                .contains("offset_of!(boxdd_sys::ffi::b2Sample, type_)")
        );
        assert!(generated.c_source.contains("&b2Touch"));
        assert!(
            generated
                .c_source
                .contains("b2Version boxdd_abi_probe_get_version(void)")
        );
        assert!(generated.c_source.contains("return b2GetVersion();"));
        assert!(
            generated
                .c_source
                .contains("boxdd_abi_probe_invoke_tree_query")
        );
        assert!(
            generated
                .rust_source
                .contains("ABI_PROBE_CALLABLE_CALLBACK_COUNT: u32 = 1")
        );
        assert!(
            generated
                .rust_source
                .contains("probe_tree_query_callback()")
        );
    }

    #[test]
    fn generator_fails_closed_when_a_public_field_is_missing_from_bindings() {
        let bindings = BINDINGS.replace("    pub value: f32,\n", "");
        let (_root, include, bindings) = fixture(HEADER, &bindings);
        let error = generate_probe(&include, &bindings, AbiProbePrecision::Single)
            .expect_err("missing public field must fail");
        assert!(error.to_string().contains("b2Sample::value"));
        assert!(
            error
                .to_string()
                .contains("no unique generated Rust access chain")
        );
    }

    #[test]
    fn generator_fails_closed_when_a_callback_signature_drifts() {
        let bindings = BINDINGS.replace("userData: u64", "userData: u32");
        let (_root, include, bindings) = fixture(HEADER, &bindings);
        let error = generate_probe(&include, &bindings, AbiProbePrecision::Single)
            .expect_err("callback signature drift must fail");
        assert!(
            error
                .to_string()
                .contains("callback `b2TreeQueryCallbackFcn`")
        );
        assert!(
            error
                .to_string()
                .contains("mismatched single C/Rust fingerprints")
        );
    }

    #[test]
    fn generator_fails_closed_when_a_callback_has_no_executable_route() {
        let header = HEADER.replace(
            "B2_API int b2Touch",
            "typedef bool b2UnknownCallback(void);\nB2_API int b2Touch",
        );
        let bindings = BINDINGS.replace(
            "unsafe extern \"C\" {",
            "pub type b2UnknownCallback = ::std::option::Option<unsafe extern \"C\" fn() -> bool>;\nunsafe extern \"C\" {",
        );
        let (_root, include, bindings) = fixture(&header, &bindings);
        let error = generate_probe(&include, &bindings, AbiProbePrecision::Single)
            .expect_err("an uncallable callback must fail closed");
        assert!(error.to_string().contains("b2UnknownCallback"));
        assert!(
            error
                .to_string()
                .contains("no executable C-to-Rust probe route")
        );
    }

    #[test]
    fn executable_callback_routes_are_unique_and_cover_box2d_32() {
        const CALLBACKS: [&str; 17] = [
            "b2AllocFcn",
            "b2FreeFcn",
            "b2AssertFcn",
            "b2LogFcn",
            "b2TaskCallback",
            "b2EnqueueTaskCallback",
            "b2FinishTaskCallback",
            "b2FrictionCallback",
            "b2RestitutionCallback",
            "b2TreeQueryCallbackFcn",
            "b2TreeRayCastCallbackFcn",
            "b2TreeBoxCastCallbackFcn",
            "b2CustomFilterFcn",
            "b2PreSolveFcn",
            "b2OverlapResultFcn",
            "b2CastResultFcn",
            "b2PlaneResultFcn",
        ];

        let routes = CALLBACKS
            .iter()
            .map(|name| CallbackProbeKind::from_name(name).expect("known callback route"))
            .collect::<BTreeSet<_>>();
        assert_eq!(routes.len(), CALLBACKS.len());
    }

    #[test]
    fn mixed_precision_source_always_selects_the_opposite_header_mode() {
        let (_root, include, bindings) = fixture(HEADER, BINDINGS);
        let single = generate_probe(&include, &bindings, AbiProbePrecision::Single)
            .expect("single ABI probe fixture");
        assert!(
            single
                .mixed_precision_c_source
                .contains("#define BOX2D_DOUBLE_PRECISION 1")
        );

        let double = generate_probe(&include, &bindings, AbiProbePrecision::Double)
            .expect("double ABI probe fixture");
        assert!(
            !double
                .mixed_precision_c_source
                .contains("#define BOX2D_DOUBLE_PRECISION 1")
        );
    }
}
