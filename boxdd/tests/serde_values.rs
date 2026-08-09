#![cfg(feature = "serde")]

use boxdd::{
    Aabb, BodyDef, BodyId, Capsule, ChainId, ChainSegment, Circle, CollisionPlane,
    ConstraintTuning, ContactId, Foundation, FoundationConfig, JointId, MassData, Plane,
    PlaneSolverResult, Position, QueryFilter, Rot, Segment, ShapeDef, ShapeId, SurfaceMaterial,
    Transform, Vec2, WorkerCount, WorldCapacity, WorldDef, WorldScalar, WorldTransform,
};
use serde::{Serialize, de::DeserializeOwned};
use static_assertions::assert_not_impl_any;

assert_not_impl_any!(BodyId: Serialize, DeserializeOwned);
assert_not_impl_any!(ShapeId: Serialize, DeserializeOwned);
assert_not_impl_any!(JointId: Serialize, DeserializeOwned);
assert_not_impl_any!(ChainId: Serialize, DeserializeOwned);
assert_not_impl_any!(ContactId: Serialize, DeserializeOwned);

#[cfg(not(feature = "double-precision"))]
const TEST_WORLD_X: WorldScalar = 10_000.125;
#[cfg(feature = "double-precision")]
const TEST_WORLD_X: WorldScalar = 10_000_000.001;

fn roundtrip<T>(value: T) -> T
where
    T: Serialize + DeserializeOwned,
{
    serde_json::from_str(&serde_json::to_string(&value).unwrap()).unwrap()
}

#[test]
fn aabb_serde_roundtrip() {
    let a = Aabb::new(Vec2::new(-1.0, -2.0), Vec2::new(3.0, 4.0)).unwrap();
    let s = serde_json::to_string(&a).unwrap();
    let b: Aabb = serde_json::from_str(&s).unwrap();
    assert_eq!(a, b);
}

#[test]
fn invariant_bearing_values_reject_invalid_serde_payloads() {
    assert!(serde_json::from_value::<Rot>(serde_json::json!(1.0e100)).is_err());
    assert!(
        serde_json::from_value::<Transform>(serde_json::json!({
            "pos": { "x": 0.0, "y": 0.0 },
            "angle": 1.0e100
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<WorldTransform>(serde_json::json!({
            "position": { "x": 0.0, "y": 0.0 },
            "angle": 1.0e100
        }))
        .is_err()
    );

    assert!(
        serde_json::from_value::<Circle>(serde_json::json!({
            "center": { "x": 0.0, "y": 0.0 },
            "radius": -1.0
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<Segment>(serde_json::json!({
            "point1": { "x": 0.0, "y": 0.0 },
            "point2": { "x": 0.0, "y": 0.0 }
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<Capsule>(serde_json::json!({
            "center1": { "x": 0.0, "y": 0.0 },
            "center2": { "x": 0.0, "y": 0.0 },
            "radius": 1.0
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<ChainSegment>(serde_json::json!({
            "ghost1": { "x": -1.0, "y": 0.0 },
            "segment": {
                "point1": { "x": 0.0, "y": 0.0 },
                "point2": { "x": 0.0, "y": 0.0 }
            },
            "ghost2": { "x": 1.0, "y": 0.0 }
        }))
        .is_err()
    );

    assert!(
        serde_json::from_value::<Aabb>(serde_json::json!({
            "lower": { "x": 1.0, "y": 0.0 },
            "upper": { "x": 0.0, "y": 1.0 }
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<Plane>(serde_json::json!({
            "normal": { "x": 2.0, "y": 0.0 },
            "offset": 0.0
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<CollisionPlane>(serde_json::json!({
            "plane": {
                "normal": { "x": 1.0, "y": 0.0 },
                "offset": 0.0
            },
            "push_limit": -1.0,
            "push": 0.0,
            "clip_velocity": true
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<MassData>(serde_json::json!({
            "mass": -1.0,
            "center": { "x": 0.0, "y": 0.0 },
            "rotational_inertia": 0.0
        }))
        .is_err()
    );
}

#[test]
fn checked_joint_and_solver_values_reject_invalid_serde_payloads() {
    let tuning = ConstraintTuning::new(4.0, 0.5).unwrap();
    assert_eq!(roundtrip(tuning), tuning);

    for payload in [
        serde_json::json!({ "hertz": -1.0, "damping_ratio": 0.5 }),
        serde_json::json!({ "hertz": 4.0, "damping_ratio": -1.0 }),
        serde_json::json!({ "hertz": 1.0e100, "damping_ratio": 0.5 }),
        serde_json::json!({ "hertz": 4.0, "damping_ratio": 1.0e100 }),
    ] {
        assert!(
            serde_json::from_value::<ConstraintTuning>(payload.clone()).is_err(),
            "ConstraintTuning accepted invalid payload {payload}"
        );
    }

    assert!(
        serde_json::from_value::<PlaneSolverResult>(serde_json::json!({
            "translation": { "x": 0.0, "y": 0.0 },
            "iteration_count": -1
        }))
        .is_err()
    );
}

#[test]
fn shape_definition_serde_uses_native_defaults_and_rejects_invalid_values() {
    let material: SurfaceMaterial = serde_json::from_str("{}").unwrap();
    assert_eq!(material.friction(), 0.6);

    let definition: ShapeDef = serde_json::from_str("{}").unwrap();
    assert_eq!(definition.density(), 1.0);
    assert!(definition.invokes_contact_creation());
    assert!(definition.updates_body_mass());

    assert!(
        serde_json::from_value::<SurfaceMaterial>(serde_json::json!({ "friction": -1.0 })).is_err()
    );
    assert!(serde_json::from_value::<ShapeDef>(serde_json::json!({ "density": -1.0 })).is_err());
}

#[test]
fn definition_serde_rejects_unknown_configuration_fields() {
    assert!(
        serde_json::from_value::<WorldDef>(serde_json::json!({
            "length_units_per_meter": 1.0,
            "enable_continous": true
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<BodyDef>(serde_json::json!({
            "length_units_per_meter": 1.0,
            "gravity_scal": 0.5
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<SurfaceMaterial>(serde_json::json!({
            "rolling_resistence": 0.1
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<ShapeDef>(serde_json::json!({
            "enable_contact_event": true
        }))
        .is_err()
    );
}

#[test]
fn world_precision_values_serde_roundtrip_without_narrowing() {
    let position = Position::new(TEST_WORLD_X, -TEST_WORLD_X);
    assert_eq!(roundtrip(position), position);

    let transform = WorldTransform::new(position, Rot::from_radians(0.375).unwrap()).unwrap();
    let round_trip = roundtrip(transform);
    assert_eq!(round_trip.position(), position);
    assert!((round_trip.rotation().angle() - transform.rotation().angle()).abs() < 1.0e-6);

    #[cfg(feature = "double-precision")]
    assert_ne!(
        f64::from(round_trip.position().x as f32),
        round_trip.position().x
    );
}

#[test]
fn query_filter_serde_roundtrip() {
    let q = QueryFilter::default().category(0x11).mask(0x22);
    let s = serde_json::to_string(&q).unwrap();
    let q2: QueryFilter = serde_json::from_str(&s).unwrap();
    assert_eq!(q.category_bits(), q2.category_bits());
    assert_eq!(q.mask_bits(), q2.mask_bits());
}

#[test]
fn body_definition_serde_preserves_optional_fields_with_explicit_scale() {
    let foundation = Foundation::initialize(FoundationConfig::new(2.5)).unwrap();
    let definition = foundation
        .body_builder()
        .name("serialized")
        .unwrap()
        .sleep_threshold(0.375)
        .build()
        .unwrap();
    let decoded = roundtrip(definition);
    assert_eq!(decoded.name(), Some(c"serialized"));
    assert_eq!(decoded.sleep_threshold(), 0.375);

    let default = foundation.body_def();
    let mut legacy = serde_json::to_value(&default).unwrap();
    let object = legacy.as_object_mut().unwrap();
    object.remove("name");
    object.remove("sleep_threshold");
    let decoded: BodyDef = serde_json::from_value(legacy).unwrap();
    assert_eq!(decoded.name(), None);
    assert_eq!(decoded.sleep_threshold(), default.sleep_threshold());
}

#[test]
fn definition_serde_requires_length_scale_provenance() {
    assert!(serde_json::from_str::<BodyDef>("{}").is_err());
    assert!(serde_json::from_str::<WorldDef>("{}").is_err());

    let foundation = Foundation::initialize(FoundationConfig::new(2.5)).unwrap();
    let mut body = serde_json::to_value(foundation.body_def()).unwrap();
    body.as_object_mut()
        .unwrap()
        .remove("length_units_per_meter");
    assert!(serde_json::from_value::<BodyDef>(body).is_err());

    let mut world = serde_json::to_value(foundation.world_def()).unwrap();
    world
        .as_object_mut()
        .unwrap()
        .remove("length_units_per_meter");
    assert!(serde_json::from_value::<WorldDef>(world).is_err());
}

#[test]
fn body_and_world_definition_serde_reject_invalid_length_scales() {
    let ray_limit = if cfg!(feature = "double-precision") {
        0.5 / 1.0e9_f32
    } else {
        0.5 / 1.0e5_f32
    };
    for value in [
        serde_json::json!(0.0),
        serde_json::json!(-1.0),
        serde_json::json!(f32::from_bits(1)),
        serde_json::json!(ray_limit),
        serde_json::json!(1.0e13_f32),
        serde_json::json!(f32::MAX),
        serde_json::json!(1.0e100),
        serde_json::json!(-1.0e100),
    ] {
        let payload = serde_json::json!({ "length_units_per_meter": value });
        assert!(
            serde_json::from_value::<BodyDef>(payload.clone()).is_err(),
            "BodyDef accepted invalid length scale {payload}"
        );
        assert!(
            serde_json::from_value::<WorldDef>(payload.clone()).is_err(),
            "WorldDef accepted invalid length scale {payload}"
        );
    }

    // JSON has no non-finite numeric literals, so these must fail at the parser boundary.
    for token in ["NaN", "Infinity", "-Infinity"] {
        let payload = format!(r#"{{"length_units_per_meter":{token}}}"#);
        assert!(serde_json::from_str::<BodyDef>(&payload).is_err());
        assert!(serde_json::from_str::<WorldDef>(&payload).is_err());
    }
}

#[test]
fn scaled_foundation_definition_roundtrips_preserve_length_scale_provenance() {
    const LENGTH_UNITS_PER_METER: f32 = 2.5;

    let foundation = Foundation::initialize(FoundationConfig::new(LENGTH_UNITS_PER_METER))
        .expect("custom-scale foundation should initialize");

    let world_value = serde_json::to_value(foundation.world_def()).unwrap();
    assert_eq!(
        world_value["length_units_per_meter"],
        serde_json::json!(LENGTH_UNITS_PER_METER)
    );
    let world_def: WorldDef = serde_json::from_value(world_value).unwrap();

    let body_value = serde_json::to_value(foundation.body_def()).unwrap();
    assert_eq!(
        body_value["length_units_per_meter"],
        serde_json::json!(LENGTH_UNITS_PER_METER)
    );
    let body_def: BodyDef = serde_json::from_value(body_value).unwrap();

    let mut world = foundation
        .create_world(world_def)
        .expect("roundtripped world definition should retain the foundation scale");
    world
        .create_body(body_def)
        .expect("roundtripped body definition should retain the foundation scale");
}

#[test]
fn body_and_world_definition_serde_reject_invalid_operational_values() {
    let foundation = Foundation::initialize(FoundationConfig::new(2.5)).unwrap();
    let mut body = serde_json::to_value(foundation.body_def()).unwrap();
    assert_eq!(body["length_units_per_meter"], serde_json::json!(2.5));
    body["linear_damping"] = serde_json::json!(-1.0);
    assert!(serde_json::from_value::<BodyDef>(body).is_err());

    let mut world = serde_json::to_value(foundation.world_def()).unwrap();
    assert_eq!(world["length_units_per_meter"], serde_json::json!(2.5));
    world["maximum_linear_speed"] = serde_json::json!(0.0);
    assert!(serde_json::from_value::<WorldDef>(world).is_err());

    let mut world = serde_json::to_value(foundation.world_def()).unwrap();
    world["maximum_linear_speed"] = serde_json::json!(f32::MAX);
    assert!(serde_json::from_value::<WorldDef>(world).is_err());
}

#[test]
fn world_operational_values_serde_preserves_validation() {
    let foundation = Foundation::initialize(FoundationConfig::new(2.5)).unwrap();
    let capacity = WorldCapacity::new(1, 2, 3, 4, 5).unwrap();
    let def = foundation
        .world_builder()
        .worker_count(WorkerCount::new(2).unwrap())
        .capacity(capacity)
        .build()
        .unwrap();
    let decoded: WorldDef = roundtrip(def);
    assert_eq!(decoded.worker_count().get(), 2);
    assert_eq!(decoded.capacity(), capacity);

    assert!(serde_json::from_str::<WorkerCount>("0").is_err());
    assert!(serde_json::from_str::<WorkerCount>("33").is_err());
    assert!(
        serde_json::from_value::<WorldCapacity>(serde_json::json!({
            "static_shape_count": u64::from(i32::MAX as u32) + 1,
            "dynamic_shape_count": 0,
            "static_body_count": 0,
            "dynamic_body_count": 0,
            "contact_count": 0
        }))
        .is_err()
    );
}
