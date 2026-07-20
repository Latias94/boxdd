#![cfg(feature = "serde")]

use boxdd::{
    Aabb, ApiError, BodyBuilder, BodyId, BodyType, ChainId, ContactId, DistanceJointDef, JointBase,
    JointId, QueryFilter, RawBodyId, RawChainId, RawContactId, RawJointId, RawShapeId, ShapeDef,
    ShapeId, Vec2, World, WorldDef,
    shapes::{self, chain::ChainDef},
};
use serde::{Serialize, de::DeserializeOwned};
use static_assertions::assert_not_impl_any;

assert_not_impl_any!(BodyId: Serialize);
assert_not_impl_any!(ShapeId: Serialize);
assert_not_impl_any!(JointId: Serialize);
assert_not_impl_any!(ChainId: Serialize);
assert_not_impl_any!(ContactId: Serialize);

fn roundtrip<T>(value: T) -> T
where
    T: Serialize + DeserializeOwned,
{
    serde_json::from_str(&serde_json::to_string(&value).unwrap()).unwrap()
}

fn assert_wrong_world<T>(result: boxdd::ApiResult<T>) {
    match result {
        Err(error) => assert_eq!(error, ApiError::WrongWorld),
        Ok(_) => panic!("expected WrongWorld, got Ok"),
    }
}

#[test]
fn aabb_serde_roundtrip() {
    let a = Aabb::new(Vec2::new(-1.0, -2.0), Vec2::new(3.0, 4.0));
    let s = serde_json::to_string(&a).unwrap();
    let b: Aabb = serde_json::from_str(&s).unwrap();
    assert_eq!(a, b);
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
fn raw_id_surrogates_roundtrip_and_require_target_world_binding() {
    let mut source = World::new(WorldDef::builder().gravity([0.0_f32, 0.0]).build()).unwrap();
    let body_a = source.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([-1.0_f32, 0.0])
            .build(),
    );
    let body_b = source.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([1.0_f32, 0.0])
            .build(),
    );
    let shape_def = ShapeDef::builder()
        .density(1.0)
        .enable_contact_events(true)
        .build();
    let shape_a =
        source.create_polygon_shape_for(body_a, &shape_def, &shapes::box_polygon(0.5_f32, 0.5));
    let _shape_b =
        source.create_polygon_shape_for(body_b, &shape_def, &shapes::box_polygon(0.5_f32, 0.5));
    let joint_body_a = source.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([-1.0_f32, 10.0])
            .build(),
    );
    let joint_body_b = source.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Static)
            .position([1.0_f32, 10.0])
            .build(),
    );
    let joint = source.create_distance_joint_id(&DistanceJointDef::new(JointBase::new(
        joint_body_a,
        joint_body_b,
    )));
    let chain_body = source.create_body_id(BodyBuilder::new().build());
    let chain = source.create_chain_for_id(
        chain_body,
        &ChainDef::builder()
            .points([[-2.0_f32, -10.0], [-1.0, -10.0], [1.0, -10.0], [2.0, -10.0]])
            .build(),
    );
    source.set_body_linear_velocity(body_a, [2.0_f32, 0.0]);
    source.set_body_linear_velocity(body_b, [-2.0_f32, 0.0]);

    let mut contact = None;
    for _ in 0..180 {
        source.step(1.0 / 60.0, 4);
        if let Some(event) = source.contact_events().begin.first() {
            contact = Some(event.contact_id);
            break;
        }
    }
    let contact = contact.expect("expected a live contact id");

    let raw_body: RawBodyId = roundtrip(body_a.unbind());
    let raw_shape: RawShapeId = roundtrip(shape_a.unbind());
    let raw_joint: RawJointId = roundtrip(joint.unbind());
    let raw_chain: RawChainId = roundtrip(chain.unbind());
    let raw_contact: RawContactId = roundtrip(contact.unbind());

    assert_eq!(source.bind_body_id(raw_body).unwrap(), body_a);
    assert_eq!(source.bind_shape_id(raw_shape).unwrap(), shape_a);
    assert_eq!(source.bind_joint_id(raw_joint).unwrap(), joint);
    assert_eq!(source.bind_chain_id(raw_chain).unwrap(), chain);
    assert_eq!(source.bind_contact_id(raw_contact).unwrap(), contact);

    let foreign = World::new(WorldDef::default()).unwrap();
    assert_wrong_world(foreign.bind_body_id(raw_body));
    assert_wrong_world(foreign.bind_shape_id(raw_shape));
    assert_wrong_world(foreign.bind_joint_id(raw_joint));
    assert_wrong_world(foreign.bind_chain_id(raw_chain));
    assert_wrong_world(foreign.bind_contact_id(raw_contact));
}

#[test]
fn tampering_with_any_serialized_raw_id_field_invalidates_authentication() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().build());
    let raw = body.unbind();
    let encoded = serde_json::to_value(raw).unwrap();
    let token = encoded["token"].as_u64().unwrap();
    let auth = encoded["auth"].as_u64().unwrap();

    for (field, replacement) in [
        ("version", serde_json::json!(99)),
        ("kind", serde_json::json!("Shape")),
        ("index1", serde_json::json!(raw.index1.wrapping_add(1))),
        ("world0", serde_json::json!(raw.world0.wrapping_add(1))),
        (
            "generation",
            serde_json::json!(raw.generation.wrapping_add(1)),
        ),
        (
            "world_generation",
            serde_json::json!(raw.world_generation.wrapping_add(1)),
        ),
        ("token", serde_json::json!(token + 1)),
        ("auth", serde_json::json!(auth ^ 1)),
    ] {
        let mut tampered = encoded.clone();
        tampered[field] = replacement;
        let tampered: RawBodyId = serde_json::from_value(tampered).unwrap();
        assert_eq!(world.bind_body_id(tampered), Err(ApiError::InvalidRawId));
    }
}
