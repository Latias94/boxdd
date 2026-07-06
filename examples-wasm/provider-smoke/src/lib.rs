use std::cell::RefCell;

use boxdd::{
    BodyBuilder, BodyId, BodyType, DistanceInput, DistanceJointDef, JointBaseBuilder, QueryFilter,
    ShapeCastPairInput, ShapeDef, ShapeProxy, SimplexCache, Transform, Vec2, World, WorldDef,
    shape_cast, shape_distance, shapes,
};

const OK: i32 = 0;
const ERR_WORLD: i32 = -1;
const ERR_SHAPE: i32 = -2;
const ERR_RUNTIME: i32 = -4;
const ERR_MOTION: i32 = -6;
const ERR_QUERY: i32 = -7;
const ERR_COLLISION: i32 = -9;
const ERR_JOINT: i32 = -10;

const SHAPE_BOX: i32 = 1;
const SHAPE_CIRCLE: i32 = 2;

thread_local! {
    static RUNTIME: RefCell<Option<RuntimeScene>> = const { RefCell::new(None) };
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

#[unsafe(no_mangle)]
pub extern "C" fn boxdd_provider_smoke() -> i32 {
    match run_smoke() {
        Ok(()) => OK,
        Err(code) => code,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn boxdd_provider_drop_millimeters() -> i32 {
    run_drop_millimeters().unwrap_or_else(|code| code)
}

#[unsafe(no_mangle)]
pub extern "C" fn boxdd_provider_ray_hit_millimeters() -> i32 {
    run_ray_hit_millimeters().unwrap_or_else(|code| code)
}

#[unsafe(no_mangle)]
pub extern "C" fn boxdd_provider_shape_cast_permyriad() -> i32 {
    run_shape_cast_permyriad().unwrap_or_else(|code| code)
}

#[unsafe(no_mangle)]
pub extern "C" fn boxdd_provider_joint_error_millimeters() -> i32 {
    run_joint_error_millimeters().unwrap_or_else(|code| code)
}

#[unsafe(no_mangle)]
pub extern "C" fn boxdd_runtime_init() -> i32 {
    match create_runtime_scene() {
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
        scene.world.step(1.0 / 60.0, 4);
        scene.frame += 1;
        scene.frame
    })
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
        (scene.world.body_position(body.id).x * 1000.0).round() as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn boxdd_runtime_body_y_millimeters(index: i32) -> i32 {
    with_runtime_body(index, |scene, body| {
        (scene.world.body_position(body.id).y * 1000.0).round() as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn boxdd_runtime_body_angle_milliradians(index: i32) -> i32 {
    with_runtime_body(index, |scene, body| {
        (scene.world.body_rotation(body.id).angle() * 1000.0).round() as i32
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

fn with_runtime_body(index: i32, f: impl FnOnce(&RuntimeScene, RuntimeBody) -> i32) -> i32 {
    if index < 0 {
        return ERR_RUNTIME;
    }
    RUNTIME.with(|runtime| {
        let runtime = runtime.borrow();
        let Some(scene) = runtime.as_ref() else {
            return ERR_RUNTIME;
        };
        let Some(body) = scene.bodies.get(index as usize).copied() else {
            return ERR_RUNTIME;
        };
        f(scene, body)
    })
}

fn run_smoke() -> Result<(), i32> {
    run_drop_millimeters()?;
    run_ray_hit_millimeters()?;
    run_shape_cast_permyriad()?;
    run_joint_error_millimeters()?;
    Ok(())
}

fn run_drop_millimeters() -> Result<i32, i32> {
    let mut world = World::new(
        WorldDef::builder()
            .gravity([0.0_f32, -10.0])
            .worker_count(0)
            .build(),
    )
    .map_err(|_| ERR_WORLD)?;

    let ground = world.create_body_id(BodyBuilder::new().position([0.0_f32, -1.0]).build());
    let ground_shape = shapes::box_polygon(8.0, 0.5);
    world.create_polygon_shape_for(ground, &ShapeDef::default(), &ground_shape);

    let body = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 4.0])
            .build(),
    );
    let box_shape = shapes::box_polygon(0.5, 0.5);
    world
        .try_create_polygon_shape_for(body, &ShapeDef::builder().density(1.0).build(), &box_shape)
        .map_err(|_| ERR_SHAPE)?;

    let start_y = world.body_position(body).y;
    for _ in 0..60 {
        world.step(1.0 / 60.0, 4);
    }
    let end_y = world.body_position(body).y;
    if end_y >= start_y - 0.1 {
        return Err(ERR_MOTION);
    }

    Ok(((start_y - end_y).max(0.0) * 1000.0).round() as i32)
}

fn run_ray_hit_millimeters() -> Result<i32, i32> {
    let mut world =
        World::new(WorldDef::builder().worker_count(0).build()).map_err(|_| ERR_WORLD)?;
    let body = world.create_body_id(BodyBuilder::new().position(Vec2::ZERO).build());
    let circle = shapes::circle([0.0_f32, 0.0], 0.5);
    world
        .try_create_circle_shape_for(body, &ShapeDef::builder().density(1.0).build(), &circle)
        .map_err(|_| ERR_SHAPE)?;

    let hit = world.cast_ray_closest([-3.0_f32, 0.0], [6.0, 0.0], QueryFilter::default());
    if !hit.hit || !hit.fraction.is_finite() || !(0.0..=1.0).contains(&hit.fraction) {
        return Err(ERR_QUERY);
    }

    Ok((hit.fraction * 6000.0).round() as i32)
}

fn run_shape_cast_permyriad() -> Result<i32, i32> {
    let square_a = square_proxy()?;
    let square_b = square_proxy()?;

    let mut cache = SimplexCache::default();
    let distance = shape_distance(
        DistanceInput::new(
            square_a,
            square_b,
            Transform::IDENTITY,
            Transform::from_pos_angle([1.4_f32, 0.0], 0.0),
        ),
        &mut cache,
    );
    if !distance.distance.is_finite() || !(0.35..=0.45).contains(&distance.distance) {
        return Err(ERR_COLLISION);
    }

    let cast = shape_cast(ShapeCastPairInput::new(
        square_a,
        square_b,
        Transform::IDENTITY,
        Transform::from_pos_angle([3.0_f32, 0.0], 0.0),
        [-4.0_f32, 0.0],
    ));
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
    .ok_or(ERR_COLLISION)
}

fn run_joint_error_millimeters() -> Result<i32, i32> {
    let mut world =
        World::new(WorldDef::builder().worker_count(0).build()).map_err(|_| ERR_WORLD)?;
    let anchor = world.create_body_id(BodyBuilder::new().position([0.0_f32, 0.0]).build());
    let body = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([1.0_f32, 0.0])
            .linear_velocity([3.0_f32, 0.0])
            .build(),
    );
    let circle = shapes::circle([0.0_f32, 0.0], 0.25);
    world
        .try_create_circle_shape_for(body, &ShapeDef::builder().density(1.0).build(), &circle)
        .map_err(|_| ERR_SHAPE)?;

    let base = JointBaseBuilder::new()
        .bodies_by_id(anchor, body)
        .local_frames([0.0_f32, 0.0], 0.0, [0.0_f32, 0.0], 0.0)
        .build();
    let joint = world.create_distance_joint_id(&DistanceJointDef::new(base).length(1.0));
    for _ in 0..60 {
        world.step(1.0 / 60.0, 4);
    }
    let length = world.distance_current_length(joint);
    if !length.is_finite() || !(0.5..=1.5).contains(&length) {
        return Err(ERR_JOINT);
    }

    Ok(((length - 1.0).abs() * 1000.0).round() as i32)
}

fn create_runtime_scene() -> Result<RuntimeScene, i32> {
    let mut world = World::new(
        WorldDef::builder()
            .gravity([0.0_f32, -10.0])
            .worker_count(0)
            .build(),
    )
    .map_err(|_| ERR_WORLD)?;

    let ground = world.create_body_id(BodyBuilder::new().position([0.0_f32, -1.0]).build());
    let ground_shape = shapes::box_polygon(9.0, 0.4);
    world.create_polygon_shape_for(ground, &ShapeDef::default(), &ground_shape);

    let mut bodies = Vec::new();
    let dynamic_def = ShapeDef::builder().density(1.0).build();
    for (index, x) in [-1.6_f32, 0.0, 1.6].into_iter().enumerate() {
        let body = world.create_body_id(
            BodyBuilder::new()
                .body_type(BodyType::Dynamic)
                .position([x, 2.4 + index as f32 * 0.9])
                .angle(index as f32 * 0.18)
                .build(),
        );
        world
            .try_create_polygon_shape_for(body, &dynamic_def, &shapes::box_polygon(0.45, 0.45))
            .map_err(|_| ERR_SHAPE)?;
        bodies.push(RuntimeBody {
            id: body,
            shape: SHAPE_BOX,
            half_width: 0.45,
            half_height: 0.45,
            radius: 0.0,
        });
    }

    let circle_body = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([2.8_f32, 4.9])
            .linear_velocity([-0.8_f32, 0.0])
            .build(),
    );
    world
        .try_create_circle_shape_for(
            circle_body,
            &dynamic_def,
            &shapes::circle([0.0_f32, 0.0], 0.36),
        )
        .map_err(|_| ERR_SHAPE)?;
    bodies.push(RuntimeBody {
        id: circle_body,
        shape: SHAPE_CIRCLE,
        half_width: 0.0,
        half_height: 0.0,
        radius: 0.36,
    });

    let base = JointBaseBuilder::new()
        .bodies_by_id(bodies[0].id, bodies[1].id)
        .local_frames([0.0_f32, 0.0], 0.0, [0.0_f32, 0.0], 0.0)
        .build();
    world.create_distance_joint_id(&DistanceJointDef::new(base).length(1.6));

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
        assert!(run_drop_millimeters().expect("drop metric") > 100);
        let ray_hit = run_ray_hit_millimeters().expect("ray metric");
        assert!((2200..=2800).contains(&ray_hit));
        let shape_cast = run_shape_cast_permyriad().expect("shape-cast metric");
        assert!((4500..=5500).contains(&shape_cast));
        let joint_error = run_joint_error_millimeters().expect("joint metric");
        assert!((0..=500).contains(&joint_error));
    }

    #[test]
    fn runtime_scene_steps() {
        let mut scene = create_runtime_scene().expect("scene");
        let y0 = scene.world.body_position(scene.bodies[0].id).y;
        for _ in 0..10 {
            scene.world.step(1.0 / 60.0, 4);
        }
        let y1 = scene.world.body_position(scene.bodies[0].id).y;
        assert!(y1 < y0);
    }
}
