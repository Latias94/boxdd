use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use boxdd::prelude::*;
use static_assertions::{assert_impl_all, assert_not_impl_any};

assert_not_impl_any!(Snapshot: Clone, Send, Sync);
assert_impl_all!(SnapshotImage: Send, Sync);

#[derive(Clone)]
struct DropProbe {
    value: u32,
    drops: Arc<AtomicUsize>,
}

impl Drop for DropProbe {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

fn world_with_body() -> (World, BodyId) {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().build());
    (world, body)
}

#[test]
fn snapshot_capture_rejects_borrowed_event_and_user_data_views() {
    let (mut world, _) = world_with_body();
    world.set_user_data(7_u32);

    let event_error = world.with_body_events_view(|_| world.try_snapshot().unwrap_err());
    assert_eq!(event_error, ApiError::WorldBusy);

    let user_data_error = world
        .with_user_data::<u32, _>(|_| world.try_snapshot().unwrap_err())
        .unwrap();
    assert_eq!(user_data_error, ApiError::WorldBusy);
    assert!(world.try_snapshot().is_ok());
}

fn chain_state(
    world: &mut World,
    body: BodyId,
    chain: ChainId,
) -> Vec<(
    ShapeId,
    Vec2,
    shapes::Segment,
    Vec2,
    SurfaceMaterial,
    Filter,
)> {
    world
        .body_shapes(body)
        .into_iter()
        .filter_map(|shape_id| {
            let shape = world.shape(shape_id).unwrap();
            (shape.parent_chain_id() == Some(chain)).then(|| {
                let segment = shape.chain_segment();
                (
                    shape_id,
                    segment.ghost1,
                    segment.segment,
                    segment.ghost2,
                    shape.surface_material(),
                    shape.filter(),
                )
            })
        })
        .collect()
}

fn reseal_image(bytes: &mut [u8]) {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&bytes[..24]);
    hasher.update(&bytes[56..]);
    bytes[24..56].copy_from_slice(hasher.finalize().as_bytes());
}

#[test]
fn image_envelope_rejects_bad_magic_truncation_and_checksum() {
    let (world, _) = world_with_body();
    let snapshot = world.snapshot();
    let bytes = snapshot.image().as_bytes();

    assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 3);
    assert_eq!(u32::from_le_bytes(bytes[12..16].try_into().unwrap()), 425);
    assert!(SnapshotImage::from_bytes(bytes).is_ok());

    let mut bad_magic = bytes.to_vec();
    bad_magic[0] ^= 0xff;
    assert_eq!(
        SnapshotImage::from_bytes(&bad_magic).unwrap_err(),
        ApiError::InvalidSnapshotImage
    );

    assert_eq!(
        SnapshotImage::from_bytes(&bytes[..bytes.len() - 1]).unwrap_err(),
        ApiError::InvalidSnapshotImage
    );

    let mut bad_checksum = bytes.to_vec();
    *bad_checksum.last_mut().unwrap() ^= 0xff;
    assert_eq!(
        SnapshotImage::from_bytes(&bad_checksum).unwrap_err(),
        ApiError::SnapshotChecksumMismatch
    );

    let mut bad_effective_source = bytes.to_vec();
    let effective_source_sha256 = 295..360;
    assert_eq!(effective_source_sha256.len(), 65);
    bad_effective_source[effective_source_sha256.start] ^= 1;
    reseal_image(&mut bad_effective_source);
    assert_eq!(
        SnapshotImage::from_bytes(&bad_effective_source).unwrap_err(),
        ApiError::SnapshotAbiMismatch
    );

    let mut unknown_host_requirement = bytes.to_vec();
    unknown_host_requirement[59] |= 0x80;
    assert_eq!(
        SnapshotImage::from_bytes(&unknown_host_requirement).unwrap_err(),
        ApiError::InvalidSnapshotImage
    );

    let mut nonzero_reserved = bytes.to_vec();
    nonzero_reserved[60] = 1;
    assert_eq!(
        SnapshotImage::from_bytes(&nonzero_reserved).unwrap_err(),
        ApiError::InvalidSnapshotImage
    );

    let mut oversized_payload = bytes.to_vec();
    oversized_payload[16..24].copy_from_slice(
        &(boxdd_sys::adapter::SnapshotLimits::default().max_image_bytes + 1).to_le_bytes(),
    );
    assert_eq!(
        SnapshotImage::from_bytes(&oversized_payload).unwrap_err(),
        ApiError::InvalidSnapshotImage
    );
}

#[test]
fn external_image_creates_a_fresh_identity_domain() {
    let (origin, origin_body) = world_with_body();
    let snapshot = origin.snapshot();
    let image = SnapshotImage::from_bytes(snapshot.image().as_bytes()).unwrap();
    let mut first = image.load(WorkerCount::default()).unwrap();
    let first_body = first.body_ids()[0];
    let mut second = image.load(WorkerCount::default()).unwrap();
    let second_body = second.body_ids()[0];

    assert_eq!(
        first.world_mut().try_body(origin_body).err().unwrap(),
        ApiError::WrongWorld
    );
    assert_eq!(
        second.world_mut().try_body(first_body).err().unwrap(),
        ApiError::WrongWorld
    );
    assert_eq!(
        first.world_mut().try_body(second_body).err().unwrap(),
        ApiError::WrongWorld
    );
    assert_eq!(first.body_ids().len(), 1);
    assert_eq!(second.body_ids().len(), 1);
    assert!(first.world().is_valid());
    assert!(second.world().is_valid());
}

#[test]
fn callback_and_mixer_wiring_is_fail_closed() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().build());
    world.set_custom_filter(|_, _| true);
    world.set_pre_solve(|_, _, _, _| true);
    world.set_friction_callback(|left, right| left.coefficient.max(right.coefficient));
    world.create_circle_shape_for(
        body,
        &ShapeDef::builder()
            .enable_custom_filtering(true)
            .enable_pre_solve_events(true)
            .build(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );
    let snapshot = world.snapshot();

    assert_eq!(
        snapshot.image().load(WorkerCount::default()).unwrap_err(),
        ApiError::SnapshotHostWiringMismatch
    );

    world.clear_custom_filter();
    assert_eq!(
        world.try_restore(&snapshot).unwrap_err(),
        ApiError::SnapshotHostWiringMismatch
    );
    assert!(world.is_valid());

    world.set_custom_filter(|_, _| true);
    world.clear_friction_callback();
    assert_eq!(
        world.try_restore(&snapshot).unwrap_err(),
        ApiError::SnapshotHostWiringMismatch
    );
    assert!(world.is_valid());

    world.set_friction_callback(|left, right| left.coefficient.max(right.coefficient));
    assert!(world.try_restore(&snapshot).is_ok());
}

#[test]
fn missing_required_callbacks_reject_before_restore_and_preserve_world() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().build());
    world.create_circle_shape_for(
        body,
        &ShapeDef::builder()
            .enable_custom_filtering(true)
            .enable_pre_solve_events(true)
            .build(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );
    let snapshot = world.snapshot();

    assert_eq!(
        world.try_restore(&snapshot).unwrap_err(),
        ApiError::SnapshotCallbacksUnavailable
    );
    assert!(world.try_body(body).is_ok());
    assert_eq!(
        snapshot.image().load(WorkerCount::default()).unwrap_err(),
        ApiError::SnapshotCallbacksUnavailable
    );
}

#[test]
fn fresh_load_publishes_joint_and_chain_ids_with_imported_relationships() {
    let mut origin = World::new(WorldDef::default()).unwrap();
    let body_a = origin.create_body_id(BodyBuilder::new().build());
    let body_b = origin.create_body_id(BodyBuilder::new().build());
    origin.create_distance_joint_id(
        &DistanceJointDef::new(JointBase::new(body_a, body_b)).length(1.0),
    );
    origin.create_chain_for_id(
        body_a,
        &shapes::chain::ChainDef::builder()
            .points([
                Vec2::new(-2.0, 0.0),
                Vec2::new(-1.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(2.0, 0.0),
            ])
            .build(),
    );
    let snapshot = origin.snapshot();

    let mut loaded = snapshot.image().load(WorkerCount::default()).unwrap();
    assert_eq!(loaded.body_ids().len(), 2);
    assert_eq!(loaded.joint_ids().len(), 1);
    assert_eq!(loaded.chain_ids().len(), 1);
    assert!(!loaded.shape_ids().is_empty());

    let joint = loaded.joint_ids()[0];
    let chain = loaded.chain_ids()[0];
    assert_eq!(
        loaded.world().try_joint_type(joint).unwrap(),
        JointType::Distance
    );
    assert!(loaded.world_mut().try_chain(chain).is_ok());
    assert!(loaded.world().try_snapshot().is_ok());
}

#[test]
fn restore_recovers_native_shape_chain_joint_and_body_state() {
    let mut world = World::new(WorldDef::default()).unwrap();
    world.set_pre_solve(|_, _, _, _| true);
    let body_a = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position(Position::new(2.0, 3.0))
            .build(),
    );
    let body_b = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position(Position::new(4.0, 3.0))
            .build(),
    );
    let shape = world.create_circle_shape_for(
        body_a,
        &ShapeDef::builder()
            .enable_sensor_events(true)
            .enable_contact_events(false)
            .enable_pre_solve_events(true)
            .enable_hit_events(false)
            .build(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );
    let joint = world.create_distance_joint_id(
        &DistanceJointDef::new(JointBase::new(body_a, body_b)).length(2.5),
    );
    let original_material = SurfaceMaterial::default()
        .with_friction(0.25)
        .with_restitution(0.35)
        .with_user_material_id(17);
    let original_filter = Filter {
        category_bits: 0x04,
        mask_bits: 0x0f,
        group_index: -2,
    };
    let chain = world.create_chain_for_id(
        body_a,
        &shapes::chain::ChainDef::builder()
            .points([
                Vec2::new(-2.0, 0.0),
                Vec2::new(-1.0, 1.0),
                Vec2::new(1.0, 1.0),
                Vec2::new(2.0, 0.0),
            ])
            .filter(original_filter)
            .single_material(&original_material)
            .build(),
    );
    let original_chain_state = chain_state(&mut world, body_a, chain);
    assert!(!original_chain_state.is_empty());
    let snapshot = world.snapshot();

    world.set_body_position_and_rotation(body_a, Position::new(9.0, 8.0), 0.4);
    world.distance_set_length(joint, 7.0);
    {
        let mut shape = world.shape(shape).unwrap();
        shape.enable_sensor_events(false);
        shape.enable_contact_events(true);
        shape.enable_pre_solve_events(false);
        shape.enable_hit_events(true);
    }
    world.destroy_chain_id(chain);
    let replacement_material = SurfaceMaterial::default().with_friction(0.9);
    world.create_chain_for_id(
        body_a,
        &shapes::chain::ChainDef::builder()
            .points([
                Vec2::new(10.0, 0.0),
                Vec2::new(11.0, 0.0),
                Vec2::new(12.0, 0.0),
                Vec2::new(13.0, 0.0),
            ])
            .single_material(&replacement_material)
            .build(),
    );

    let restored = world.try_restore(&snapshot).unwrap();
    let restored_body = restored.body_id(body_a).unwrap();
    let restored_shape = restored.shape_id(shape).unwrap();
    let restored_joint = restored.joint_id(joint).unwrap();
    let restored_chain = restored.chain_id(chain).unwrap();

    assert_eq!(world.body_position(restored_body), Position::new(2.0, 3.0));
    assert_eq!(world.distance_length(restored_joint), 2.5);
    {
        let shape = world.shape(restored_shape).unwrap();
        assert!(shape.sensor_events_enabled());
        assert!(!shape.contact_events_enabled());
        assert!(shape.pre_solve_events_enabled());
        assert!(!shape.hit_events_enabled());
    }

    let restored_chain_state = chain_state(&mut world, restored_body, restored_chain);
    assert_eq!(restored_chain_state.len(), original_chain_state.len());
    assert_eq!(
        restored_chain_state
            .iter()
            .map(|(_, ghost1, segment, ghost2, material, filter)| {
                (*ghost1, *segment, *ghost2, *material, *filter)
            })
            .collect::<Vec<_>>(),
        original_chain_state
            .iter()
            .map(|(_, ghost1, segment, ghost2, material, filter)| {
                (*ghost1, *segment, *ghost2, *material, *filter)
            })
            .collect::<Vec<_>>()
    );
    for (old_segment, _, _, _, _, _) in original_chain_state {
        let new_segment = restored.shape_id(old_segment).unwrap();
        assert_ne!(new_segment, old_segment);
        assert_eq!(
            world.try_shape(old_segment).err().unwrap(),
            ApiError::InvalidShapeId
        );
        assert_eq!(
            world.shape(new_segment).unwrap().parent_chain_id(),
            Some(restored_chain)
        );
    }
    assert_eq!(world.joint_type(restored_joint), JointType::Distance);
}

#[test]
fn foreign_snapshot_rejects_without_mutating_the_target() {
    let (origin, _) = world_with_body();
    let snapshot = origin.snapshot();
    let (mut target, target_body) = world_with_body();

    assert_eq!(
        target.try_restore(&snapshot).unwrap_err(),
        ApiError::ForeignSnapshot
    );
    assert!(target.try_body(target_body).is_ok());
}

#[test]
fn restore_remaps_destroyed_snapshot_objects_and_invalidates_later_objects() {
    let (mut world, snapshot_body) = world_with_body();
    let snapshot = world.snapshot();

    world.destroy_body_id(snapshot_body);
    let later_body = world.create_body_id(BodyBuilder::new().build());
    let report = world.try_restore(&snapshot).unwrap();
    let restored_body = report.body_id(snapshot_body).unwrap();

    assert_ne!(restored_body, snapshot_body);
    assert_eq!(
        world.try_body(snapshot_body).err().unwrap(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        world.try_body(later_body).err().unwrap(),
        ApiError::InvalidBodyId
    );
    assert!(world.try_body(restored_body).is_ok());
}

#[test]
fn repeated_restore_preserves_exact_intersection_then_mints_after_divergence() {
    let (mut world, snapshot_body) = world_with_body();
    let snapshot = world.snapshot();

    let unchanged = world
        .try_restore(&snapshot)
        .unwrap()
        .body_id(snapshot_body)
        .unwrap();
    assert_eq!(unchanged, snapshot_body);
    world.destroy_body_id(unchanged);

    let first = world
        .try_restore(&snapshot)
        .unwrap()
        .body_id(snapshot_body)
        .unwrap();
    let second = world
        .try_restore(&snapshot)
        .unwrap()
        .body_id(snapshot_body)
        .unwrap();

    assert_ne!(first, snapshot_body);
    assert_ne!(second, first);
    assert_eq!(
        world.try_body(snapshot_body).err().unwrap(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        world.try_body(first).err().unwrap(),
        ApiError::InvalidBodyId
    );
    assert!(world.try_body(second).is_ok());
}

#[test]
fn restore_discards_transient_events_for_objects_destroyed_after_snapshot() {
    let mut world = World::new(WorldDef::default()).unwrap();

    let sensor_body = world.create_body_id(
        BodyBuilder::new()
            .position(Position::new(-4.0, 0.0))
            .build(),
    );
    let sensor_visitor = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position(Position::new(-4.0, 0.0))
            .build(),
    );
    let sensor_def = ShapeDef::builder()
        .sensor(true)
        .enable_sensor_events(true)
        .build();
    let visitor_def = ShapeDef::builder().enable_sensor_events(true).build();
    world.create_circle_shape_for(
        sensor_body,
        &sensor_def,
        &shapes::circle([0.0_f32, 0.0], 1.0),
    );
    world.create_circle_shape_for(
        sensor_visitor,
        &visitor_def,
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );

    let contact_body =
        world.create_body_id(BodyBuilder::new().position(Position::new(4.0, 0.0)).build());
    let contact_visitor = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position(Position::new(4.0, 0.0))
            .build(),
    );
    let contact_def = ShapeDef::builder().enable_contact_events(true).build();
    world.create_circle_shape_for(
        contact_body,
        &contact_def,
        &shapes::circle([0.0_f32, 0.0], 1.0),
    );
    world.create_circle_shape_for(
        contact_visitor,
        &contact_def,
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );

    world.step(1.0 / 60.0, 4);
    assert_eq!(world.sensor_events().begin.len(), 1);
    assert_eq!(world.contact_events().begin.len(), 1);
    let snapshot = world.snapshot();

    world.destroy_body_id(sensor_visitor);
    world.destroy_body_id(contact_visitor);
    let restore = world.try_restore(&snapshot).unwrap();
    assert_ne!(restore.body_id(sensor_visitor).unwrap(), sensor_visitor);
    assert_ne!(restore.body_id(contact_visitor).unwrap(), contact_visitor);

    world.step(1.0 / 60.0, 4);
    let sensor_events = world.sensor_events();
    let contact_events = world.contact_events();

    assert!(sensor_events.begin.is_empty());
    assert!(sensor_events.end.is_empty());
    assert!(contact_events.begin.is_empty());
    assert!(contact_events.end.is_empty());
    assert!(contact_events.hit.is_empty());
}

#[test]
fn restore_reattaches_only_unchanged_userdata_versions() {
    let keep_drops = Arc::new(AtomicUsize::new(0));
    let replace_drops = Arc::new(AtomicUsize::new(0));
    let later_drops = Arc::new(AtomicUsize::new(0));
    let world_drops = Arc::new(AtomicUsize::new(0));

    let mut world = World::new(WorldDef::default()).unwrap();
    let keep_body = world.create_body_id(BodyBuilder::new().build());
    let replace_body = world.create_body_id(BodyBuilder::new().build());
    let joint_body = world.create_body_id(BodyBuilder::new().build());
    let shape = world.create_circle_shape_for(
        keep_body,
        &ShapeDef::default(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );
    let joint = world.create_distance_joint_id(
        &DistanceJointDef::new(JointBase::new(keep_body, joint_body)).length(1.0),
    );

    world.set_user_data(DropProbe {
        value: 10,
        drops: Arc::clone(&world_drops),
    });
    world.body(keep_body).unwrap().set_user_data(DropProbe {
        value: 20,
        drops: Arc::clone(&keep_drops),
    });
    world.body(replace_body).unwrap().set_user_data(DropProbe {
        value: 30,
        drops: Arc::clone(&replace_drops),
    });
    world.shape(shape).unwrap().set_user_data(40_u32);
    world.joint(joint).unwrap().set_user_data(50_u32);

    let snapshot = world.snapshot();

    world.body(replace_body).unwrap().set_user_data(DropProbe {
        value: 31,
        drops: Arc::clone(&replace_drops),
    });
    let later_body = world.create_body_id(BodyBuilder::new().build());
    world.body(later_body).unwrap().set_user_data(DropProbe {
        value: 60,
        drops: Arc::clone(&later_drops),
    });

    let report = world.try_restore(&snapshot).unwrap();
    let keep_body = report.body_id(keep_body).unwrap();
    let replace_body = report.body_id(replace_body).unwrap();
    let shape = report.shape_id(shape).unwrap();
    let joint = report.joint_id(joint).unwrap();

    assert_eq!(
        world.with_user_data::<DropProbe, _>(|value| value.value),
        Some(10)
    );
    assert_eq!(
        world
            .body(keep_body)
            .unwrap()
            .with_user_data::<DropProbe, _>(|value| value.value),
        Some(20)
    );
    assert_eq!(
        world
            .body(replace_body)
            .unwrap()
            .with_user_data::<DropProbe, _>(|value| value.value),
        None
    );
    assert_eq!(
        world
            .shape(shape)
            .unwrap()
            .with_user_data::<u32, _>(|value| *value),
        Some(40)
    );
    assert_eq!(
        world
            .joint(joint)
            .unwrap()
            .with_user_data::<u32, _>(|value| *value),
        Some(50)
    );
    assert_eq!(replace_drops.load(Ordering::SeqCst), 2);
    assert_eq!(later_drops.load(Ordering::SeqCst), 1);
    assert_eq!(keep_drops.load(Ordering::SeqCst), 0);
    assert_eq!(world_drops.load(Ordering::SeqCst), 0);

    drop(world);
    assert_eq!(keep_drops.load(Ordering::SeqCst), 1);
    assert_eq!(world_drops.load(Ordering::SeqCst), 1);
}

#[test]
fn taken_userdata_is_not_resurrected_by_native_identity_reuse() {
    let drops = Arc::new(AtomicUsize::new(0));
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().build());
    world.body(body).unwrap().set_user_data(DropProbe {
        value: 41,
        drops: Arc::clone(&drops),
    });
    let snapshot = world.snapshot();

    let taken = world
        .body(body)
        .unwrap()
        .take_user_data::<DropProbe>()
        .unwrap();
    assert_eq!(taken.value, 41);
    drop(taken);
    assert_eq!(drops.load(Ordering::SeqCst), 1);

    let restored = world.try_restore(&snapshot).unwrap().body_id(body).unwrap();
    assert_eq!(
        world
            .body(restored)
            .unwrap()
            .with_user_data::<DropProbe, _>(|value| value.value),
        None
    );
    drop(world);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}
