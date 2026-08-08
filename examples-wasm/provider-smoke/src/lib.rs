use std::{
    alloc::{Layout, alloc, dealloc},
    cell::RefCell,
    ptr::NonNull,
};

use boxdd::{
    BodyBuilder, BodyId, BodyType, DistanceInput, DistanceJointDef, Foundation, Position,
    QueryFilter, ShapeCastPairInput, ShapeDef, ShapeProxy, SimplexCache, Transform, Vec2,
    WorkerCount, World, WorldBuilder, shape_cast, shape_distance, shapes,
};

const OK: i32 = 0;
const ERR_WORLD: i32 = -1;
const ERR_SHAPE: i32 = -2;
const ERR_RUNTIME: i32 = -4;
const ERR_MOTION: i32 = -6;
const ERR_QUERY: i32 = -7;
const ERR_COLLISION: i32 = -9;
const ERR_JOINT: i32 = -10;
const ERR_PROVIDER: i32 = -11;
#[cfg(target_arch = "wasm32")]
const ERR_WORKER_POLICY: i32 = -12;

const SHAPE_BOX: i32 = 1;
const SHAPE_CIRCLE: i32 = 2;

fn default_foundation() -> Result<&'static Foundation, i32> {
    Foundation::initialize_default().map_err(|_| ERR_WORLD)
}

thread_local! {
    static RUNTIME: RefCell<Option<RuntimeScene>> = const { RefCell::new(None) };
    static ALLOCATOR_PROBE: RefCell<Vec<ProbeAllocation>> = const { RefCell::new(Vec::new()) };
    static ALIGNED_ALLOCATOR_PROBE: RefCell<Vec<AlignedProbeAllocation>> =
        const { RefCell::new(Vec::new()) };
}

#[derive(Clone, Copy)]
struct RuntimeBody {
    id: BodyId,
    shape: i32,
    half_width: f32,
    half_height: f32,
    radius: f32,
}

struct RuntimeScene {
    world: World,
    bodies: Vec<RuntimeBody>,
    frame: i32,
}

struct ProbeAllocation {
    bytes: Vec<u8>,
    pattern: u8,
}

struct AlignedProbeAllocation {
    pointer: NonNull<u8>,
    layout: Layout,
    pattern: u8,
}

impl Drop for AlignedProbeAllocation {
    fn drop(&mut self) {
        // SAFETY: pointer was allocated by the active global allocator with this exact Layout.
        unsafe { dealloc(self.pointer.as_ptr(), self.layout) };
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn boxdd_provider_smoke() -> i32 {
    match run_smoke() {
        Ok(()) => OK,
        Err(code) => code,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn boxdd_provider_drop_millimeters() -> i32 {
    default_foundation()
        .and_then(run_drop_millimeters)
        .unwrap_or_else(|code| code)
}

#[unsafe(no_mangle)]
pub extern "C" fn boxdd_provider_ray_hit_millimeters() -> i32 {
    default_foundation()
        .and_then(run_ray_hit_millimeters)
        .unwrap_or_else(|code| code)
}

#[unsafe(no_mangle)]
pub extern "C" fn boxdd_provider_shape_cast_permyriad() -> i32 {
    default_foundation()
        .and_then(run_shape_cast_permyriad)
        .unwrap_or_else(|code| code)
}

#[unsafe(no_mangle)]
pub extern "C" fn boxdd_provider_joint_error_millimeters() -> i32 {
    default_foundation()
        .and_then(run_joint_error_millimeters)
        .unwrap_or_else(|code| code)
}

#[unsafe(no_mangle)]
pub extern "C" fn boxdd_runtime_init() -> i32 {
    match default_foundation().and_then(create_runtime_scene) {
        Ok(scene) => {
            RUNTIME.with(|runtime| {
                *runtime.borrow_mut() = Some(scene);
            });
            OK
        }
        Err(code) => code,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn boxdd_runtime_step() -> i32 {
    RUNTIME.with(|runtime| {
        let mut runtime = runtime.borrow_mut();
        let Some(scene) = runtime.as_mut() else {
            return ERR_RUNTIME;
        };
        if scene.world.step(1.0 / 60.0, 4).is_err() {
            return ERR_RUNTIME;
        }
        scene.frame += 1;
        scene.frame
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn boxdd_runtime_reset() -> i32 {
    RUNTIME.with(|runtime| {
        *runtime.borrow_mut() = None;
    });
    OK
}

#[unsafe(no_mangle)]
pub extern "C" fn boxdd_provider_box2d_byte_count() -> i64 {
    // SAFETY: this read-only Box2D metric has no pointer arguments.
    unsafe { boxdd_sys::ffi::b2GetByteCount() }
}

#[unsafe(no_mangle)]
pub extern "C" fn boxdd_allocator_probe_push(byte_count: u32, pattern: u32) -> i32 {
    const MAX_PROBE_ALLOCATION_BYTES: u32 = 32 * 1024 * 1024;
    if byte_count == 0 || byte_count > MAX_PROBE_ALLOCATION_BYTES || pattern > u8::MAX.into() {
        return ERR_PROVIDER;
    }
    let allocation = ProbeAllocation {
        bytes: vec![pattern as u8; byte_count as usize],
        pattern: pattern as u8,
    };
    ALLOCATOR_PROBE.with(|probe| {
        probe.borrow_mut().push(allocation);
    });
    OK
}

#[unsafe(no_mangle)]
pub extern "C" fn boxdd_allocator_probe_validate() -> i32 {
    ALLOCATOR_PROBE.with(|probe| {
        let probe = probe.borrow();
        for allocation in probe.iter() {
            let page_samples_match = allocation
                .bytes
                .iter()
                .step_by(4096)
                .all(|byte| *byte == allocation.pattern);
            if !page_samples_match || allocation.bytes.last().copied() != Some(allocation.pattern) {
                return ERR_PROVIDER;
            }
        }
        i32::try_from(probe.len()).unwrap_or(ERR_PROVIDER)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn boxdd_allocator_aligned_probe_push(
    byte_count: u32,
    alignment: u32,
    pattern: u32,
) -> i32 {
    const MAX_ALIGNED_PROBE_BYTES: u32 = 4 * 1024 * 1024;
    const MAX_PROBE_ALIGNMENT: u32 = 1024 * 1024;
    if byte_count == 0
        || byte_count > MAX_ALIGNED_PROBE_BYTES
        || alignment < std::mem::align_of::<usize>() as u32
        || alignment > MAX_PROBE_ALIGNMENT
        || !alignment.is_power_of_two()
        || pattern > u8::MAX.into()
    {
        return ERR_PROVIDER;
    }
    let Ok(layout) = Layout::from_size_align(byte_count as usize, alignment as usize) else {
        return ERR_PROVIDER;
    };
    // SAFETY: layout was validated above and the returned allocation is checked before use.
    let pointer = unsafe { alloc(layout) };
    let Some(pointer) = NonNull::new(pointer) else {
        return ERR_PROVIDER;
    };
    if !(pointer.as_ptr() as usize).is_multiple_of(layout.align()) {
        // SAFETY: pointer came from alloc with this exact Layout.
        unsafe { dealloc(pointer.as_ptr(), layout) };
        return ERR_PROVIDER;
    }
    // SAFETY: pointer owns layout.size() writable bytes.
    unsafe { pointer.as_ptr().write_bytes(pattern as u8, layout.size()) };
    ALIGNED_ALLOCATOR_PROBE.with(|probe| {
        probe.borrow_mut().push(AlignedProbeAllocation {
            pointer,
            layout,
            pattern: pattern as u8,
        });
    });
    OK
}

#[unsafe(no_mangle)]
pub extern "C" fn boxdd_allocator_aligned_probe_validate() -> i32 {
    ALIGNED_ALLOCATOR_PROBE.with(|probe| {
        let probe = probe.borrow();
        for allocation in probe.iter() {
            if !(allocation.pointer.as_ptr() as usize).is_multiple_of(allocation.layout.align()) {
                return ERR_PROVIDER;
            }
            // SAFETY: the allocation remains owned by this probe for the duration of the borrow.
            let bytes = unsafe {
                std::slice::from_raw_parts(allocation.pointer.as_ptr(), allocation.layout.size())
            };
            let page_samples_match = bytes
                .iter()
                .step_by(4096)
                .all(|byte| *byte == allocation.pattern);
            if !page_samples_match || bytes.last().copied() != Some(allocation.pattern) {
                return ERR_PROVIDER;
            }
        }
        i32::try_from(probe.len()).unwrap_or(ERR_PROVIDER)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn boxdd_allocator_probe_reset() -> i32 {
    ALLOCATOR_PROBE.with(|probe| {
        let mut probe = probe.borrow_mut();
        probe.clear();
        probe.shrink_to_fit();
    });
    ALIGNED_ALLOCATOR_PROBE.with(|probe| {
        let mut probe = probe.borrow_mut();
        probe.clear();
        probe.shrink_to_fit();
    });
    OK
}

#[unsafe(no_mangle)]
pub extern "C" fn boxdd_runtime_body_count() -> i32 {
    RUNTIME.with(|runtime| {
        runtime
            .borrow()
            .as_ref()
            .map_or(ERR_RUNTIME, |scene| scene.bodies.len() as i32)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn boxdd_runtime_body_shape(index: i32) -> i32 {
    with_runtime_body(index, |_, body| body.shape)
}

#[unsafe(no_mangle)]
pub extern "C" fn boxdd_runtime_body_x_millimeters(index: i32) -> i32 {
    with_runtime_body(index, |scene, body| {
        scene
            .world
            .body(body.id)
            .and_then(|body| body.position())
            .map_or(ERR_RUNTIME, |position| (position.x * 1000.0).round() as i32)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn boxdd_runtime_body_y_millimeters(index: i32) -> i32 {
    with_runtime_body(index, |scene, body| {
        scene
            .world
            .body(body.id)
            .and_then(|body| body.position())
            .map_or(ERR_RUNTIME, |position| (position.y * 1000.0).round() as i32)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn boxdd_runtime_body_angle_milliradians(index: i32) -> i32 {
    with_runtime_body(index, |scene, body| {
        scene
            .world
            .body(body.id)
            .and_then(|body| body.rotation())
            .map_or(ERR_RUNTIME, |rotation| {
                (rotation.angle() * 1000.0).round() as i32
            })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn boxdd_runtime_body_half_width_millimeters(index: i32) -> i32 {
    with_runtime_body(index, |_, body| (body.half_width * 1000.0).round() as i32)
}

#[unsafe(no_mangle)]
pub extern "C" fn boxdd_runtime_body_half_height_millimeters(index: i32) -> i32 {
    with_runtime_body(index, |_, body| (body.half_height * 1000.0).round() as i32)
}

#[unsafe(no_mangle)]
pub extern "C" fn boxdd_runtime_body_radius_millimeters(index: i32) -> i32 {
    with_runtime_body(index, |_, body| (body.radius * 1000.0).round() as i32)
}

fn with_runtime_body(index: i32, f: impl FnOnce(&mut RuntimeScene, RuntimeBody) -> i32) -> i32 {
    if index < 0 {
        return ERR_RUNTIME;
    }
    RUNTIME.with(|runtime| {
        let mut runtime = runtime.borrow_mut();
        let Some(scene) = runtime.as_mut() else {
            return ERR_RUNTIME;
        };
        let Some(body) = scene.bodies.get(index as usize).copied() else {
            return ERR_RUNTIME;
        };
        f(scene, body)
    })
}

fn run_smoke() -> Result<(), i32> {
    let foundation = default_foundation()?;
    verify_provider_identity()?;
    #[cfg(target_arch = "wasm32")]
    verify_worker_count_policy()?;
    run_drop_millimeters(foundation)?;
    run_ray_hit_millimeters(foundation)?;
    run_shape_cast_permyriad(foundation)?;
    run_joint_error_millimeters(foundation)?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn verify_worker_count_policy() -> Result<(), i32> {
    if WorkerCount::default().get() != 1
        || WorkerCount::new(2) != Err(boxdd::Error::UnsupportedWorkerCount { requested: 2 })
    {
        return Err(ERR_WORKER_POLICY);
    }
    Ok(())
}

fn verify_provider_identity() -> Result<(), i32> {
    fn text(bytes: &[u8]) -> Result<&str, i32> {
        let end = bytes
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(bytes.len());
        std::str::from_utf8(&bytes[..end]).map_err(|_| ERR_PROVIDER)
    }

    let identity = boxdd_sys::adapter::runtime_identity().ok_or(ERR_PROVIDER)?;
    if identity.struct_size as usize != std::mem::size_of_val(&identity)
        || identity.abi_version != boxdd_sys::adapter::ADAPTER_ABI_VERSION
        || identity.snapshot_version == 0
        || identity.snapshot_layout_hash == 0
        || identity.pointer_width != 4
        || identity.little_endian == 0
        || (identity.double_precision != 0) != boxdd_sys::IS_DOUBLE_PRECISION
        || identity.validation_enabled != 0
        || identity.private_abi_hash.iter().all(|byte| *byte == 0)
        || text(&identity.upstream_sha)? != boxdd_sys::UPSTREAM_SHA
        || boxdd_sys::TARGET_ABI != "wasm32-unknown-unknown"
        || text(&identity.target_abi)? != boxdd_sys::TARGET_ABI
        || text(&identity.adapter_source_sha256)? != boxdd_sys::ADAPTER_SOURCE_SHA256
        || text(&identity.recording_contract_blake3)? != boxdd_sys::RECORDING_CONTRACT_BLAKE3
    {
        return Err(ERR_PROVIDER);
    }
    if unsafe { boxdd_sys::adapter::boxddAdapter_AbiVersion() } != identity.abi_version
        || unsafe { boxdd_sys::adapter::boxddAdapter_GetSnapshotLayoutHash() }
            != identity.snapshot_layout_hash
        || unsafe { boxdd_sys::adapter::boxddRecPlayer_IsHealthy(std::ptr::null()) }
        || boxdd_sys::adapter::validate_snapshot(
            &[],
            &boxdd_sys::adapter::SnapshotLimits::default(),
        )
        .is_ok()
    {
        return Err(ERR_PROVIDER);
    }
    Ok(())
}

fn run_drop_millimeters(foundation: &'static Foundation) -> Result<i32, i32> {
    let mut world = foundation
        .create_world(
            WorldBuilder::from(foundation.world_def())
                .gravity([0.0_f32, -10.0])
                .worker_count(WorkerCount::default())
                .build()
                .map_err(|_| ERR_WORLD)?,
        )
        .map_err(|_| ERR_WORLD)?;
    let expected_gravity = Vec2::new(3.25, -7.5);
    world.set_gravity(expected_gravity).map_err(|_| ERR_WORLD)?;
    if world.gravity().map_err(|_| ERR_WORLD)? != expected_gravity {
        return Err(ERR_WORLD);
    }
    world.set_gravity([0.0, -10.0]).map_err(|_| ERR_WORLD)?;

    let ground = world
        .create_body(
            BodyBuilder::from(foundation.body_def())
                .position([0.0_f32, -1.0])
                .build()
                .map_err(|_| ERR_SHAPE)?,
        )
        .map_err(|_| ERR_SHAPE)?;
    let ground_shape = shapes::box_polygon(8.0, 0.5).map_err(|_| ERR_SHAPE)?;
    world
        .body(ground)
        .and_then(|mut body| body.create_polygon(&ShapeDef::default(), &ground_shape))
        .map_err(|_| ERR_SHAPE)?;

    let body = world
        .create_body(
            BodyBuilder::from(foundation.body_def())
                .body_type(BodyType::Dynamic)
                .position([0.0_f32, 4.0])
                .build()
                .map_err(|_| ERR_SHAPE)?,
        )
        .map_err(|_| ERR_SHAPE)?;
    let box_shape = shapes::box_polygon(0.5, 0.5).map_err(|_| ERR_SHAPE)?;
    world
        .body(body)
        .and_then(|mut body| {
            body.create_polygon(&ShapeDef::builder().density(1.0).build()?, &box_shape)
        })
        .map_err(|_| ERR_SHAPE)?;

    let start_y = world
        .body(body)
        .and_then(|body| body.position())
        .map_err(|_| ERR_MOTION)?
        .y;
    for _ in 0..60 {
        drop(world.step(1.0 / 60.0, 4).map_err(|_| ERR_MOTION)?);
    }
    let end_y = world
        .body(body)
        .and_then(|body| body.position())
        .map_err(|_| ERR_MOTION)?
        .y;
    if end_y >= start_y - 0.1 {
        return Err(ERR_MOTION);
    }

    Ok(((start_y - end_y).max(0.0) * 1000.0).round() as i32)
}

fn run_ray_hit_millimeters(foundation: &'static Foundation) -> Result<i32, i32> {
    let mut world = foundation
        .create_world(
            WorldBuilder::from(foundation.world_def())
                .worker_count(WorkerCount::default())
                .build()
                .map_err(|_| ERR_WORLD)?,
        )
        .map_err(|_| ERR_WORLD)?;
    let body = world
        .create_body(
            BodyBuilder::from(foundation.body_def())
                .position(Vec2::ZERO)
                .build()
                .map_err(|_| ERR_SHAPE)?,
        )
        .map_err(|_| ERR_SHAPE)?;
    let circle = shapes::circle([0.0_f32, 0.0], 0.5).map_err(|_| ERR_SHAPE)?;
    world
        .body(body)
        .and_then(|mut body| {
            body.create_circle(&ShapeDef::builder().density(1.0).build()?, &circle)
        })
        .map_err(|_| ERR_SHAPE)?;

    let hit = world
        .query()
        .map_err(|_| ERR_QUERY)?
        .cast_ray_closest(
            Position::from([-3.0_f32, 0.0]),
            [6.0, 0.0],
            QueryFilter::default(),
        )
        .map_err(|_| ERR_QUERY)?
        .ok_or(ERR_QUERY)?;
    if !hit.hit || !hit.fraction.is_finite() || !(0.0..=1.0).contains(&hit.fraction) {
        return Err(ERR_QUERY);
    }

    Ok((hit.fraction * 6000.0).round() as i32)
}

fn run_shape_cast_permyriad(_foundation: &'static Foundation) -> Result<i32, i32> {
    let square_a = square_proxy()?;
    let square_b = square_proxy()?;

    let mut cache = SimplexCache::default();
    let distance_input = DistanceInput::new(
        square_a,
        square_b,
        Transform::from_pos_angle([1.4_f32, 0.0], 0.0).map_err(|_| ERR_COLLISION)?,
    )
    .map_err(|_| ERR_COLLISION)?;
    let distance = shape_distance(distance_input, &mut cache).map_err(|_| ERR_COLLISION)?;
    if !distance.distance.is_finite() || !(0.35..=0.45).contains(&distance.distance) {
        return Err(ERR_COLLISION);
    }

    let cast_input = ShapeCastPairInput::new(
        square_a,
        square_b,
        Transform::from_pos_angle([3.0_f32, 0.0], 0.0).map_err(|_| ERR_COLLISION)?,
        [-4.0_f32, 0.0],
    )
    .map_err(|_| ERR_COLLISION)?;
    let cast = shape_cast(cast_input).map_err(|_| ERR_COLLISION)?;
    if !cast.hit || !cast.fraction.is_finite() || !(0.0..=1.0).contains(&cast.fraction) {
        return Err(ERR_COLLISION);
    }

    Ok((cast.fraction * 10_000.0).round() as i32)
}

fn square_proxy() -> Result<ShapeProxy, i32> {
    ShapeProxy::new(
        [
            [-0.5_f32, -0.5],
            [0.5_f32, -0.5],
            [0.5_f32, 0.5],
            [-0.5_f32, 0.5],
        ],
        0.0,
    )
    .map_err(|_| ERR_COLLISION)
}

fn run_joint_error_millimeters(foundation: &'static Foundation) -> Result<i32, i32> {
    let mut world = foundation
        .create_world(
            WorldBuilder::from(foundation.world_def())
                .worker_count(WorkerCount::default())
                .build()
                .map_err(|_| ERR_WORLD)?,
        )
        .map_err(|_| ERR_WORLD)?;
    let anchor = world
        .create_body(
            BodyBuilder::from(foundation.body_def())
                .position([0.0_f32, 0.0])
                .build()
                .map_err(|_| ERR_SHAPE)?,
        )
        .map_err(|_| ERR_JOINT)?;
    let body = world
        .create_body(
            BodyBuilder::from(foundation.body_def())
                .body_type(BodyType::Dynamic)
                .position([1.0_f32, 0.0])
                .linear_velocity([3.0_f32, 0.0])
                .build()
                .map_err(|_| ERR_SHAPE)?,
        )
        .map_err(|_| ERR_JOINT)?;
    let circle = shapes::circle([0.0_f32, 0.0], 0.25).map_err(|_| ERR_SHAPE)?;
    world
        .body(body)
        .and_then(|mut body| {
            body.create_circle(&ShapeDef::builder().density(1.0).build()?, &circle)
        })
        .map_err(|_| ERR_SHAPE)?;

    let base = world.joint_base(anchor, body).map_err(|_| ERR_JOINT)?;
    let joint = world
        .create_distance_joint(&DistanceJointDef::new(base).length(1.0))
        .map_err(|_| ERR_JOINT)?;
    for _ in 0..60 {
        drop(world.step(1.0 / 60.0, 4).map_err(|_| ERR_JOINT)?);
    }
    let length = world
        .joint(joint)
        .and_then(|joint| joint.into_distance())
        .and_then(|joint| joint.current_length())
        .map_err(|_| ERR_JOINT)?;
    if !length.is_finite() || !(0.5..=1.5).contains(&length) {
        return Err(ERR_JOINT);
    }

    Ok(((length - 1.0).abs() * 1000.0).round() as i32)
}

fn create_runtime_scene(foundation: &'static Foundation) -> Result<RuntimeScene, i32> {
    let mut world = foundation
        .create_world(
            WorldBuilder::from(foundation.world_def())
                .gravity([0.0_f32, -10.0])
                .worker_count(WorkerCount::default())
                .build()
                .map_err(|_| ERR_WORLD)?,
        )
        .map_err(|_| ERR_WORLD)?;

    let ground = world
        .create_body(
            BodyBuilder::from(foundation.body_def())
                .position([0.0_f32, -1.0])
                .build()
                .map_err(|_| ERR_SHAPE)?,
        )
        .map_err(|_| ERR_SHAPE)?;
    let ground_shape = shapes::box_polygon(9.0, 0.4).map_err(|_| ERR_SHAPE)?;
    world
        .body(ground)
        .and_then(|mut body| body.create_polygon(&ShapeDef::default(), &ground_shape))
        .map_err(|_| ERR_SHAPE)?;

    let mut bodies = Vec::new();
    let dynamic_def = ShapeDef::builder()
        .density(1.0)
        .build()
        .map_err(|_| ERR_SHAPE)?;
    for (index, x) in [-1.6_f32, 0.0, 1.6].into_iter().enumerate() {
        let body = world
            .create_body(
                BodyBuilder::from(foundation.body_def())
                    .body_type(BodyType::Dynamic)
                    .position([x, 2.4 + index as f32 * 0.9])
                    .angle(index as f32 * 0.18)
                    .build()
                    .map_err(|_| ERR_SHAPE)?,
            )
            .map_err(|_| ERR_SHAPE)?;
        let polygon = shapes::box_polygon(0.45, 0.45).map_err(|_| ERR_SHAPE)?;
        world
            .body(body)
            .and_then(|mut body| body.create_polygon(&dynamic_def, &polygon))
            .map_err(|_| ERR_SHAPE)?;
        bodies.push(RuntimeBody {
            id: body,
            shape: SHAPE_BOX,
            half_width: 0.45,
            half_height: 0.45,
            radius: 0.0,
        });
    }

    let circle_body = world
        .create_body(
            BodyBuilder::from(foundation.body_def())
                .body_type(BodyType::Dynamic)
                .position([2.8_f32, 4.9])
                .linear_velocity([-0.8_f32, 0.0])
                .build()
                .map_err(|_| ERR_SHAPE)?,
        )
        .map_err(|_| ERR_SHAPE)?;
    let circle = shapes::circle([0.0_f32, 0.0], 0.36).map_err(|_| ERR_SHAPE)?;
    world
        .body(circle_body)
        .and_then(|mut body| body.create_circle(&dynamic_def, &circle))
        .map_err(|_| ERR_SHAPE)?;
    bodies.push(RuntimeBody {
        id: circle_body,
        shape: SHAPE_CIRCLE,
        half_width: 0.0,
        half_height: 0.0,
        radius: 0.36,
    });

    let base = world
        .joint_base(bodies[0].id, bodies[1].id)
        .map_err(|_| ERR_JOINT)?;
    world
        .create_distance_joint(&DistanceJointDef::new(base).length(1.6))
        .map_err(|_| ERR_JOINT)?;

    Ok(RuntimeScene {
        world,
        bodies,
        frame: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exported_metrics_cover_provider_runtime() {
        let foundation = default_foundation().expect("foundation");
        assert!(run_drop_millimeters(foundation).expect("drop metric") > 100);
        let ray_hit = run_ray_hit_millimeters(foundation).expect("ray metric");
        assert!((2200..=2800).contains(&ray_hit));
        let shape_cast = run_shape_cast_permyriad(foundation).expect("shape-cast metric");
        assert!((4500..=5500).contains(&shape_cast));
        let joint_error = run_joint_error_millimeters(foundation).expect("joint metric");
        assert!((0..=500).contains(&joint_error));
    }

    #[test]
    fn runtime_scene_steps() {
        let foundation = default_foundation().expect("foundation");
        let mut scene = create_runtime_scene(foundation).expect("scene");
        let y0 = scene
            .world
            .body(scene.bodies[0].id)
            .and_then(|body| body.position())
            .unwrap()
            .y;
        for _ in 0..10 {
            drop(scene.world.step(1.0 / 60.0, 4).unwrap());
        }
        let y1 = scene
            .world
            .body(scene.bodies[0].id)
            .and_then(|body| body.position())
            .unwrap()
            .y;
        assert!(y1 < y0);
    }
}
