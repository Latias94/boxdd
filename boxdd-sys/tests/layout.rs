use std::mem::{align_of, size_of};

use boxdd_sys::ffi;

#[test]
fn math_and_id_layout_matches_c_headers() {
    assert_eq!(size_of::<ffi::b2Vec2>(), 8);
    assert_eq!(align_of::<ffi::b2Vec2>(), 4);
    assert_eq!(size_of::<ffi::b2Rot>(), 8);
    assert_eq!(align_of::<ffi::b2Rot>(), 4);
    assert_eq!(size_of::<ffi::b2Transform>(), 16);
    assert_eq!(align_of::<ffi::b2Transform>(), 4);
    assert_eq!(size_of::<ffi::b2AABB>(), 16);
    assert_eq!(align_of::<ffi::b2AABB>(), 4);

    assert_eq!(size_of::<ffi::b2WorldId>(), 4);
    assert_eq!(align_of::<ffi::b2WorldId>(), 2);
    assert_eq!(size_of::<ffi::b2BodyId>(), 8);
    assert_eq!(size_of::<ffi::b2ShapeId>(), 8);
    assert_eq!(size_of::<ffi::b2ChainId>(), 8);
    assert_eq!(size_of::<ffi::b2JointId>(), 8);
    assert_eq!(size_of::<ffi::b2ContactId>(), 12);
}

#[test]
fn default_definitions_are_initialized_by_upstream_functions() {
    unsafe {
        let world = ffi::b2DefaultWorldDef();
        assert_ne!(world.internalValue, 0);
        assert!(world.enableSleep);

        let body = ffi::b2DefaultBodyDef();
        assert_ne!(body.internalValue, 0);
        assert_eq!(body.type_, ffi::b2BodyType_b2_staticBody);

        let shape = ffi::b2DefaultShapeDef();
        assert_ne!(shape.internalValue, 0);
        assert!(!shape.enableContactEvents);
    }
}

#[test]
fn world_symbols_link_and_validate_lifecycle() {
    unsafe {
        let mut def = ffi::b2DefaultWorldDef();
        def.gravity = ffi::b2Vec2 { x: 0.0, y: -10.0 };

        let world = ffi::b2CreateWorld(&def);
        assert!(ffi::b2World_IsValid(world));

        ffi::b2World_Step(world, 1.0 / 60.0, 4);
        ffi::b2DestroyWorld(world);
        assert!(!ffi::b2World_IsValid(world));
    }
}
