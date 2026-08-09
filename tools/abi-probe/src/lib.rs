//! Native, C-backed ABI verification fixture for `boxdd-sys`.
//!
//! This crate is workspace-only and is not part of the published binding surface.

use std::{
    ffi::{CStr, c_void},
    os::raw::c_char,
    ptr,
    sync::{
        Mutex,
        atomic::{AtomicU32, Ordering},
    },
};

use boxdd_sys::ffi;

unsafe extern "C" {
    fn boxdd_abi_probe_get_version() -> ffi::b2Version;
    fn boxdd_abi_probe_precision_matches() -> bool;
    fn boxdd_abi_probe_mixed_precision_matches() -> bool;
    fn boxdd_abi_probe_tree_node_size() -> usize;
    fn boxdd_abi_probe_tree_node_alignment() -> usize;
    fn boxdd_abi_probe_tree_node_aabb_offset() -> usize;
    fn boxdd_abi_probe_tree_node_category_bits_offset() -> usize;
    fn boxdd_abi_probe_tree_node_children_offset() -> usize;
    fn boxdd_abi_probe_tree_node_user_data_offset() -> usize;
    fn boxdd_abi_probe_tree_node_parent_offset() -> usize;
    fn boxdd_abi_probe_tree_node_next_offset() -> usize;
    fn boxdd_abi_probe_tree_node_height_offset() -> usize;
    fn boxdd_abi_probe_tree_node_flags_offset() -> usize;
    fn boxdd_abi_probe_invoke_alloc(callback: ffi::b2AllocFcn) -> bool;
    fn boxdd_abi_probe_invoke_assert(callback: ffi::b2AssertFcn) -> i32;
    fn boxdd_abi_probe_invoke_cast_result(
        callback: ffi::b2CastResultFcn,
        context: *mut c_void,
    ) -> f32;
    fn boxdd_abi_probe_invoke_custom_filter(
        callback: ffi::b2CustomFilterFcn,
        context: *mut c_void,
    ) -> bool;
    fn boxdd_abi_probe_invoke_enqueue_task(
        callback: ffi::b2EnqueueTaskCallback,
        context: *mut c_void,
    ) -> u32;
    fn boxdd_abi_probe_invoke_finish_task(
        callback: ffi::b2FinishTaskCallback,
        context: *mut c_void,
    ) -> bool;
    fn boxdd_abi_probe_invoke_free(callback: ffi::b2FreeFcn) -> bool;
    fn boxdd_abi_probe_invoke_friction(callback: ffi::b2FrictionCallback) -> f32;
    fn boxdd_abi_probe_invoke_log(callback: ffi::b2LogFcn) -> bool;
    fn boxdd_abi_probe_invoke_overlap_result(
        callback: ffi::b2OverlapResultFcn,
        context: *mut c_void,
    ) -> bool;
    fn boxdd_abi_probe_invoke_plane_result(
        callback: ffi::b2PlaneResultFcn,
        context: *mut c_void,
    ) -> bool;
    fn boxdd_abi_probe_invoke_pre_solve(callback: ffi::b2PreSolveFcn, context: *mut c_void)
    -> bool;
    fn boxdd_abi_probe_invoke_restitution(callback: ffi::b2RestitutionCallback) -> f32;
    fn boxdd_abi_probe_invoke_task(callback: ffi::b2TaskCallback, context: *mut c_void) -> bool;
    fn boxdd_abi_probe_invoke_tree_box_cast(
        callback: ffi::b2TreeBoxCastCallbackFcn,
        context: *mut c_void,
    ) -> f32;
    fn boxdd_abi_probe_invoke_tree_query(
        callback: ffi::b2TreeQueryCallbackFcn,
        context: *mut c_void,
    ) -> bool;
    fn boxdd_abi_probe_invoke_tree_ray_cast(
        callback: ffi::b2TreeRayCastCallbackFcn,
        context: *mut c_void,
    ) -> f32;
}

pub const fn callback_names() -> &'static [&'static str] {
    &[
        "b2AllocFcn",
        "b2AssertFcn",
        "b2CastResultFcn",
        "b2CustomFilterFcn",
        "b2EnqueueTaskCallback",
        "b2FinishTaskCallback",
        "b2FreeFcn",
        "b2FrictionCallback",
        "b2LogFcn",
        "b2OverlapResultFcn",
        "b2PlaneResultFcn",
        "b2PreSolveFcn",
        "b2RestitutionCallback",
        "b2TaskCallback",
        "b2TreeBoxCastCallbackFcn",
        "b2TreeQueryCallbackFcn",
        "b2TreeRayCastCallbackFcn",
    ]
}

pub const fn is_double_precision() -> bool {
    cfg!(feature = "double-precision")
}

pub fn c_version() -> (i32, i32, i32) {
    version_tuple(unsafe { boxdd_abi_probe_get_version() })
}

pub fn rust_version() -> (i32, i32, i32) {
    version_tuple(unsafe { ffi::b2GetVersion() })
}

fn version_tuple(version: ffi::b2Version) -> (i32, i32, i32) {
    (version.major, version.minor, version.revision)
}

/// Result of invoking one Rust callback through its generated, strongly typed C trampoline.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CallbackProbeResult {
    pub name: &'static str,
    pub call_count: u32,
    pub argument_match_count: u32,
    pub nested_call_count: u32,
    pub return_matched: bool,
}

/// Invoke every public Box2D callback typedef exactly once through C.
pub fn callback_probe_results() -> Vec<CallbackProbeResult> {
    let _guard = CALLBACK_PROBE_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    vec![
        probe_alloc_callback(),
        probe_assert_callback(),
        probe_cast_result_callback(),
        probe_custom_filter_callback(),
        probe_enqueue_task_callback(),
        probe_finish_task_callback(),
        probe_free_callback(),
        probe_friction_callback(),
        probe_log_callback(),
        probe_overlap_result_callback(),
        probe_plane_result_callback(),
        probe_pre_solve_callback(),
        probe_restitution_callback(),
        probe_task_callback(),
        probe_tree_box_cast_callback(),
        probe_tree_query_callback(),
        probe_tree_ray_cast_callback(),
    ]
}

pub fn precision_matches() -> bool {
    unsafe { boxdd_abi_probe_precision_matches() }
}

pub fn mixed_precision_matches() -> bool {
    unsafe { boxdd_abi_probe_mixed_precision_matches() }
}

pub fn tree_node_anonymous_union_layout_matches() -> bool {
    let c_layout = unsafe {
        [
            boxdd_abi_probe_tree_node_size(),
            boxdd_abi_probe_tree_node_alignment(),
            boxdd_abi_probe_tree_node_aabb_offset(),
            boxdd_abi_probe_tree_node_category_bits_offset(),
            boxdd_abi_probe_tree_node_children_offset(),
            boxdd_abi_probe_tree_node_user_data_offset(),
            boxdd_abi_probe_tree_node_parent_offset(),
            boxdd_abi_probe_tree_node_next_offset(),
            boxdd_abi_probe_tree_node_height_offset(),
            boxdd_abi_probe_tree_node_flags_offset(),
        ]
    };
    let rust_layout = [
        std::mem::size_of::<ffi::b2TreeNode>(),
        std::mem::align_of::<ffi::b2TreeNode>(),
        std::mem::offset_of!(ffi::b2TreeNode, aabb),
        std::mem::offset_of!(ffi::b2TreeNode, categoryBits),
        std::mem::offset_of!(ffi::b2TreeNode, __bindgen_anon_1),
        std::mem::offset_of!(ffi::b2TreeNode, __bindgen_anon_1),
        std::mem::offset_of!(ffi::b2TreeNode, __bindgen_anon_2),
        std::mem::offset_of!(ffi::b2TreeNode, __bindgen_anon_2),
        std::mem::offset_of!(ffi::b2TreeNode, height),
        std::mem::offset_of!(ffi::b2TreeNode, flags),
    ];
    c_layout == rust_layout
}

#[derive(Default)]
struct ContextObservation {
    calls: u32,
    matches: u32,
}

impl ContextObservation {
    fn record(&mut self, arguments_match: bool) {
        self.calls += 1;
        self.matches += u32::from(arguments_match);
    }

    fn result(
        &self,
        name: &'static str,
        nested_call_count: u32,
        return_matched: bool,
    ) -> CallbackProbeResult {
        CallbackProbeResult {
            name,
            call_count: self.calls,
            argument_match_count: self.matches,
            nested_call_count,
            return_matched,
        }
    }
}

struct AtomicObservation {
    calls: AtomicU32,
    matches: AtomicU32,
}

impl AtomicObservation {
    const fn new() -> Self {
        Self {
            calls: AtomicU32::new(0),
            matches: AtomicU32::new(0),
        }
    }

    fn reset(&self) {
        self.calls.store(0, Ordering::SeqCst);
        self.matches.store(0, Ordering::SeqCst);
    }

    fn record(&self, arguments_match: bool) {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.matches
            .fetch_add(u32::from(arguments_match), Ordering::SeqCst);
    }

    fn result(&self, name: &'static str, return_matched: bool) -> CallbackProbeResult {
        CallbackProbeResult {
            name,
            call_count: self.calls.load(Ordering::SeqCst),
            argument_match_count: self.matches.load(Ordering::SeqCst),
            nested_call_count: 0,
            return_matched,
        }
    }
}

fn record_context(context: *mut c_void, arguments_match: bool) {
    if let Some(observation) = unsafe { context.cast::<ContextObservation>().as_mut() } {
        observation.record(arguments_match);
    }
}

fn c_string_matches(value: *const c_char, expected: &[u8]) -> bool {
    if value.is_null() {
        return false;
    }
    unsafe { CStr::from_ptr(value) }.to_bytes() == expected
}

static CALLBACK_PROBE_LOCK: Mutex<()> = Mutex::new(());
static ALLOC_OBSERVATION: AtomicObservation = AtomicObservation::new();
static FREE_OBSERVATION: AtomicObservation = AtomicObservation::new();
static ASSERT_OBSERVATION: AtomicObservation = AtomicObservation::new();
static LOG_OBSERVATION: AtomicObservation = AtomicObservation::new();
static FRICTION_OBSERVATION: AtomicObservation = AtomicObservation::new();
static RESTITUTION_OBSERVATION: AtomicObservation = AtomicObservation::new();

#[repr(C, align(64))]
struct AlignedAllocation {
    _storage: [u8; 64],
}

static mut ALLOC_SENTINEL: AlignedAllocation = AlignedAllocation { _storage: [0; 64] };

unsafe extern "C" fn alloc_callback(size: usize, alignment: i32) -> *mut c_void {
    ALLOC_OBSERVATION.record(size == 37 && alignment == 16);
    ptr::addr_of_mut!(ALLOC_SENTINEL).cast::<c_void>()
}

fn probe_alloc_callback() -> CallbackProbeResult {
    ALLOC_OBSERVATION.reset();
    let return_matched = unsafe { boxdd_abi_probe_invoke_alloc(Some(alloc_callback)) };
    ALLOC_OBSERVATION.result("b2AllocFcn", return_matched)
}

unsafe extern "C" fn free_callback(memory: *mut c_void, size: usize) {
    FREE_OBSERVATION.record(!memory.is_null() && size == 41);
}

fn probe_free_callback() -> CallbackProbeResult {
    FREE_OBSERVATION.reset();
    let return_matched = unsafe { boxdd_abi_probe_invoke_free(Some(free_callback)) };
    FREE_OBSERVATION.result("b2FreeFcn", return_matched)
}

unsafe extern "C" fn assert_callback(
    condition: *const c_char,
    file_name: *const c_char,
    line_number: i32,
) -> i32 {
    ASSERT_OBSERVATION.record(
        c_string_matches(condition, b"boxdd-condition")
            && c_string_matches(file_name, b"boxdd-file.c")
            && line_number == 73,
    );
    0x1234
}

fn probe_assert_callback() -> CallbackProbeResult {
    ASSERT_OBSERVATION.reset();
    let result = unsafe { boxdd_abi_probe_invoke_assert(Some(assert_callback)) };
    ASSERT_OBSERVATION.result("b2AssertFcn", result == 0x1234)
}

unsafe extern "C" fn log_callback(message: *const c_char) {
    LOG_OBSERVATION.record(c_string_matches(message, b"boxdd-log-message"));
}

fn probe_log_callback() -> CallbackProbeResult {
    LOG_OBSERVATION.reset();
    let return_matched = unsafe { boxdd_abi_probe_invoke_log(Some(log_callback)) };
    LOG_OBSERVATION.result("b2LogFcn", return_matched)
}

unsafe extern "C" fn friction_callback(
    friction_a: f32,
    material_a: u64,
    friction_b: f32,
    material_b: u64,
) -> f32 {
    FRICTION_OBSERVATION
        .record(friction_a == 0.25 && material_a == 101 && friction_b == 0.75 && material_b == 202);
    0.625
}

fn probe_friction_callback() -> CallbackProbeResult {
    FRICTION_OBSERVATION.reset();
    let result = unsafe { boxdd_abi_probe_invoke_friction(Some(friction_callback)) };
    FRICTION_OBSERVATION.result("b2FrictionCallback", result == 0.625)
}

unsafe extern "C" fn restitution_callback(
    restitution_a: f32,
    material_a: u64,
    restitution_b: f32,
    material_b: u64,
) -> f32 {
    RESTITUTION_OBSERVATION.record(
        restitution_a == 0.125 && material_a == 303 && restitution_b == 0.875 && material_b == 404,
    );
    0.375
}

fn probe_restitution_callback() -> CallbackProbeResult {
    RESTITUTION_OBSERVATION.reset();
    let result = unsafe { boxdd_abi_probe_invoke_restitution(Some(restitution_callback)) };
    RESTITUTION_OBSERVATION.result("b2RestitutionCallback", result == 0.375)
}

unsafe extern "C" fn task_callback(context: *mut c_void) {
    record_context(context, !context.is_null());
}

fn probe_task_callback() -> CallbackProbeResult {
    let mut observation = ContextObservation::default();
    let context = ptr::from_mut(&mut observation).cast::<c_void>();
    let return_matched = unsafe { boxdd_abi_probe_invoke_task(Some(task_callback), context) };
    observation.result("b2TaskCallback", 0, return_matched)
}

unsafe extern "C" fn enqueue_task_callback(
    task: ffi::b2TaskCallback,
    task_context: *mut c_void,
    user_context: *mut c_void,
) -> *mut c_void {
    record_context(
        user_context,
        task.is_some() && !task_context.is_null() && !user_context.is_null(),
    );
    if let Some(task) = task {
        unsafe { task(task_context) };
    }
    ptr::null_mut()
}

fn probe_enqueue_task_callback() -> CallbackProbeResult {
    let mut observation = ContextObservation::default();
    let context = ptr::from_mut(&mut observation).cast::<c_void>();
    let status =
        unsafe { boxdd_abi_probe_invoke_enqueue_task(Some(enqueue_task_callback), context) };
    observation.result(
        "b2EnqueueTaskCallback",
        u32::from(status & 1 != 0),
        status & 2 != 0,
    )
}

unsafe extern "C" fn finish_task_callback(user_task: *mut c_void, user_context: *mut c_void) {
    record_context(
        user_context,
        !user_task.is_null() && !user_context.is_null(),
    );
}

fn probe_finish_task_callback() -> CallbackProbeResult {
    let mut observation = ContextObservation::default();
    let context = ptr::from_mut(&mut observation).cast::<c_void>();
    let return_matched =
        unsafe { boxdd_abi_probe_invoke_finish_task(Some(finish_task_callback), context) };
    observation.result("b2FinishTaskCallback", 0, return_matched)
}

unsafe extern "C" fn tree_query_callback(
    proxy_id: i32,
    user_data: u64,
    context: *mut c_void,
) -> bool {
    record_context(context, proxy_id == 7 && user_data == 11);
    true
}

fn probe_tree_query_callback() -> CallbackProbeResult {
    let mut observation = ContextObservation::default();
    let context = ptr::from_mut(&mut observation).cast::<c_void>();
    let result = unsafe { boxdd_abi_probe_invoke_tree_query(Some(tree_query_callback), context) };
    observation.result("b2TreeQueryCallbackFcn", 0, result)
}

unsafe extern "C" fn tree_ray_cast_callback(
    input: *const ffi::b2RayCastInput,
    proxy_id: i32,
    user_data: u64,
    context: *mut c_void,
) -> f32 {
    let input_matches = unsafe { input.as_ref() }.is_some_and(|input| {
        input.origin.x == 1.25
            && input.origin.y == -2.5
            && input.translation.x == 3.5
            && input.translation.y == -4.5
            && input.maxFraction == 0.75
    });
    record_context(context, input_matches && proxy_id == 13 && user_data == 17);
    0.625
}

fn probe_tree_ray_cast_callback() -> CallbackProbeResult {
    let mut observation = ContextObservation::default();
    let context = ptr::from_mut(&mut observation).cast::<c_void>();
    let result =
        unsafe { boxdd_abi_probe_invoke_tree_ray_cast(Some(tree_ray_cast_callback), context) };
    observation.result("b2TreeRayCastCallbackFcn", 0, result == 0.625)
}

unsafe extern "C" fn tree_box_cast_callback(
    input: *const ffi::b2BoxCastInput,
    proxy_id: i32,
    user_data: u64,
    context: *mut c_void,
) -> f32 {
    let input_matches = unsafe { input.as_ref() }.is_some_and(|input| {
        input.box_.lowerBound.x == -1.25
            && input.box_.lowerBound.y == -2.25
            && input.box_.upperBound.x == 3.25
            && input.box_.upperBound.y == 4.25
            && input.translation.x == 5.5
            && input.translation.y == -6.5
            && input.maxFraction == 0.875
    });
    record_context(context, input_matches && proxy_id == 19 && user_data == 23);
    0.5
}

fn probe_tree_box_cast_callback() -> CallbackProbeResult {
    let mut observation = ContextObservation::default();
    let context = ptr::from_mut(&mut observation).cast::<c_void>();
    let result =
        unsafe { boxdd_abi_probe_invoke_tree_box_cast(Some(tree_box_cast_callback), context) };
    observation.result("b2TreeBoxCastCallbackFcn", 0, result == 0.5)
}

fn shape_id_matches(id: ffi::b2ShapeId, index1: i32, world0: u16, generation: u16) -> bool {
    id.index1 == index1 && id.world0 == world0 && id.generation == generation
}

unsafe extern "C" fn custom_filter_callback(
    shape_a: ffi::b2ShapeId,
    shape_b: ffi::b2ShapeId,
    context: *mut c_void,
) -> bool {
    record_context(
        context,
        shape_id_matches(shape_a, 29, 31, 37) && shape_id_matches(shape_b, 41, 43, 47),
    );
    true
}

fn probe_custom_filter_callback() -> CallbackProbeResult {
    let mut observation = ContextObservation::default();
    let context = ptr::from_mut(&mut observation).cast::<c_void>();
    let result =
        unsafe { boxdd_abi_probe_invoke_custom_filter(Some(custom_filter_callback), context) };
    observation.result("b2CustomFilterFcn", 0, result)
}

unsafe extern "C" fn pre_solve_callback(
    shape_a: ffi::b2ShapeId,
    shape_b: ffi::b2ShapeId,
    point: ffi::b2Pos,
    normal: ffi::b2Vec2,
    context: *mut c_void,
) -> bool {
    record_context(
        context,
        shape_id_matches(shape_a, 53, 59, 61)
            && shape_id_matches(shape_b, 67, 71, 73)
            && point.x == 1.25
            && point.y == -2.5
            && normal.x == 0.0
            && normal.y == 1.0,
    );
    true
}

fn probe_pre_solve_callback() -> CallbackProbeResult {
    let mut observation = ContextObservation::default();
    let context = ptr::from_mut(&mut observation).cast::<c_void>();
    let result = unsafe { boxdd_abi_probe_invoke_pre_solve(Some(pre_solve_callback), context) };
    observation.result("b2PreSolveFcn", 0, result)
}

unsafe extern "C" fn overlap_result_callback(shape: ffi::b2ShapeId, context: *mut c_void) -> bool {
    record_context(context, shape_id_matches(shape, 79, 83, 89));
    true
}

fn probe_overlap_result_callback() -> CallbackProbeResult {
    let mut observation = ContextObservation::default();
    let context = ptr::from_mut(&mut observation).cast::<c_void>();
    let result =
        unsafe { boxdd_abi_probe_invoke_overlap_result(Some(overlap_result_callback), context) };
    observation.result("b2OverlapResultFcn", 0, result)
}

unsafe extern "C" fn cast_result_callback(
    shape: ffi::b2ShapeId,
    point: ffi::b2Pos,
    normal: ffi::b2Vec2,
    fraction: f32,
    context: *mut c_void,
) -> f32 {
    record_context(
        context,
        shape_id_matches(shape, 97, 101, 103)
            && point.x == 2.25
            && point.y == -3.5
            && normal.x == -0.5
            && normal.y == 1.0
            && fraction == 0.375,
    );
    0.625
}

fn probe_cast_result_callback() -> CallbackProbeResult {
    let mut observation = ContextObservation::default();
    let context = ptr::from_mut(&mut observation).cast::<c_void>();
    let result = unsafe { boxdd_abi_probe_invoke_cast_result(Some(cast_result_callback), context) };
    observation.result("b2CastResultFcn", 0, result == 0.625)
}

unsafe extern "C" fn plane_result_callback(
    shape: ffi::b2ShapeId,
    plane: *const ffi::b2PlaneResult,
    context: *mut c_void,
) -> bool {
    let plane_matches = unsafe { plane.as_ref() }.is_some_and(|plane| {
        plane.plane.normal.x == 0.0
            && plane.plane.normal.y == 1.0
            && plane.plane.offset == 1.25
            && plane.point.x == -2.5
            && plane.point.y == 3.75
            && plane.hit
    });
    record_context(
        context,
        shape_id_matches(shape, 107, 109, 113) && plane_matches,
    );
    true
}

fn probe_plane_result_callback() -> CallbackProbeResult {
    let mut observation = ContextObservation::default();
    let context = ptr::from_mut(&mut observation).cast::<c_void>();
    let result =
        unsafe { boxdd_abi_probe_invoke_plane_result(Some(plane_result_callback), context) };
    observation.result("b2PlaneResultFcn", 0, result)
}
