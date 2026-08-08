#![allow(
    clippy::too_many_arguments,
    reason = "query geometry and its absolute world origin are deliberately explicit"
)]

use boxdd_sys::ffi;

#[cfg(not(target_arch = "wasm32"))]
use crate::ShapeProxy;
use crate::core::callback_state::PendingUserValue;
#[cfg(all(test, not(target_arch = "wasm32")))]
use crate::core::world_core::WorldCore;
use crate::error::Result;
#[cfg(not(target_arch = "wasm32"))]
use crate::types::ShapeId;
use crate::types::{Position, Vec2};
use crate::world::{OwnerAdapter, QueryCall, QueryCallGuard, QueryProof};

#[cfg(not(target_arch = "wasm32"))]
use super::buffers::{MoverQueryBuffer, RayQueryBuffer, ShapeQueryBuffer};
use super::buffers::{
    validate_native_fraction, validate_native_query_counters, validate_native_ray_output,
};
use super::raw;
use super::types::*;

/// A read-only query capability tied to an ordinary world or recording session.
///
/// The capability holds a real owner borrow. Safe mutable world access, restore, and destruction
/// therefore cannot overlap a query, while ordinary and recording operations share one semantic
/// implementation.
pub struct Query<'owner> {
    proof: QueryProof<'owner>,
}

impl<'owner> Query<'owner> {
    pub(crate) fn new(owner: &'owner impl OwnerAdapter) -> Result<Self> {
        Ok(Self {
            proof: QueryProof::acquire(owner)?,
        })
    }

    /// Return the closest ray hit.
    pub fn cast_ray_closest<V: Into<Vec2>>(
        &self,
        origin: Position,
        translation: V,
        filter: QueryFilter,
    ) -> Result<Option<RayResult>> {
        Ok(self
            .cast_ray_closest_with_stats(origin, translation, filter)?
            .hit)
    }

    /// Return the closest ray hit and the native broad-phase traversal counters.
    pub fn cast_ray_closest_with_stats<V: Into<Vec2>>(
        &self,
        origin: Position,
        translation: V,
        filter: QueryFilter,
    ) -> Result<ClosestRayCastResult> {
        let (authorized, translation) = self.begin_owned_operation(translation)?;
        check_query_position_valid("Query::cast_ray_closest", "origin", origin)?;
        let translation = translation.into_inner().into();
        check_query_vec2_valid("Query::cast_ray_closest", "translation", translation)?;
        QueryCallGuard::invoke(authorized, |call| {
            map_closest_ray_result(
                &call,
                raw::cast_ray_closest(&call, origin, translation, filter),
            )
        })
    }

    /// Cast a capsule mover and return its remaining fraction (`1.0` means unobstructed).
    pub fn cast_mover<V1: Into<Vec2>, V2: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: Position,
        c1: V1,
        c2: V2,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
    ) -> Result<f32> {
        let (authorized, inputs) = self.begin_owned_operation((c1, c2, translation))?;
        check_query_position_valid("Query::cast_mover", "origin", origin)?;
        check_query_mover_radius_valid("Query::cast_mover", radius)?;
        let (c1, c2, translation) = inputs.into_inner();
        let c1 = c1.into();
        let c2 = c2.into();
        let translation = translation.into();
        check_query_vec2_valid("Query::cast_mover", "c1", c1)?;
        check_query_vec2_valid("Query::cast_mover", "c2", c2)?;
        check_query_vec2_valid("Query::cast_mover", "translation", translation)?;
        authorized.invoke(|call| {
            let fraction = raw::cast_mover(&call, origin, c1, c2, radius, translation, filter);
            validate_native_fraction("Query::cast_mover", "fraction", fraction)
        })
    }

    /// Collect every shape overlapping an AABB.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn overlap_aabb(
        &self,
        origin: Position,
        aabb: Aabb,
        filter: QueryFilter,
    ) -> Result<Vec<ShapeId>> {
        let mut buffer = ShapeQueryBuffer::new();
        self.overlap_aabb_into(origin, aabb, filter, &mut buffer)?;
        Ok(buffer.into_vec())
    }

    /// Collect every shape overlapping an AABB into reusable raw-and-mapped storage.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn overlap_aabb_into(
        &self,
        origin: Position,
        aabb: Aabb,
        filter: QueryFilter,
        buffer: &mut ShapeQueryBuffer,
    ) -> Result<()> {
        let authorized = self.prepare_buffer(buffer)?;
        check_query_position_valid("Query::overlap_aabb", "origin", origin)?;
        check_query_aabb_valid("Query::overlap_aabb", aabb)?;
        buffer_transaction(buffer, |buffer| {
            authorized.invoke(|call| {
                raw::overlap_aabb(&call, origin, aabb, filter, buffer)?;
                buffer.publish(&call)
            })
        })
    }

    /// Visit AABB overlap results after their native IDs have been bound in one batch.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn visit_overlap_aabb<F>(
        &self,
        origin: Position,
        aabb: Aabb,
        filter: QueryFilter,
        visit: F,
    ) -> Result<bool>
    where
        F: FnMut(ShapeId) -> bool,
    {
        let mut buffer = ShapeQueryBuffer::new();
        self.visit_overlap_aabb_with_buffer(origin, aabb, filter, &mut buffer, visit)
    }

    /// Visit AABB overlap results while reusing both raw and mapped storage.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn visit_overlap_aabb_with_buffer<F>(
        &self,
        origin: Position,
        aabb: Aabb,
        filter: QueryFilter,
        buffer: &mut ShapeQueryBuffer,
        visit: F,
    ) -> Result<bool>
    where
        F: FnMut(ShapeId) -> bool,
    {
        let mut visit = PendingUserValue::new(visit);
        let authorized = self.prepare_buffer(buffer)?;
        check_query_position_valid("Query::visit_overlap_aabb", "origin", origin)?;
        check_query_aabb_valid("Query::visit_overlap_aabb", aabb)?;
        buffer_transaction(buffer, |buffer| {
            authorized.invoke(|call| {
                raw::overlap_aabb(&call, origin, aabb, filter, buffer)?;
                buffer.publish(&call)
            })?;
            Ok(visit_shape_ids(buffer.as_slice(), visit.as_mut()))
        })
    }

    /// Collect shapes overlapping `proxy` at the world-space `origin`.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn overlap_shape(
        &self,
        origin: Position,
        proxy: ShapeProxy,
        filter: QueryFilter,
    ) -> Result<Vec<ShapeId>> {
        let mut buffer = ShapeQueryBuffer::new();
        self.overlap_shape_into(origin, proxy, filter, &mut buffer)?;
        Ok(buffer.into_vec())
    }

    /// Collect shapes overlapping `proxy` into reusable storage.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn overlap_shape_into(
        &self,
        origin: Position,
        proxy: ShapeProxy,
        filter: QueryFilter,
        buffer: &mut ShapeQueryBuffer,
    ) -> Result<()> {
        let authorized = self.prepare_buffer(buffer)?;
        check_query_position_valid("Query::overlap_shape", "origin", origin)?;
        proxy.validate()?;
        let proxy = proxy.into_raw();
        buffer_transaction(buffer, |buffer| {
            authorized.invoke(|call| {
                raw::overlap_shape(&call, origin, &proxy, filter, buffer)?;
                buffer.publish(&call)
            })
        })
    }

    /// Visit shapes overlapping `proxy` after batch identity binding.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn visit_overlap_shape<F>(
        &self,
        origin: Position,
        proxy: ShapeProxy,
        filter: QueryFilter,
        visit: F,
    ) -> Result<bool>
    where
        F: FnMut(ShapeId) -> bool,
    {
        let mut buffer = ShapeQueryBuffer::new();
        self.visit_overlap_shape_with_buffer(origin, proxy, filter, &mut buffer, visit)
    }

    /// Visit shapes overlapping `proxy` with reusable storage.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn visit_overlap_shape_with_buffer<F>(
        &self,
        origin: Position,
        proxy: ShapeProxy,
        filter: QueryFilter,
        buffer: &mut ShapeQueryBuffer,
        visit: F,
    ) -> Result<bool>
    where
        F: FnMut(ShapeId) -> bool,
    {
        let (authorized, visit) =
            self.prepare_owned_buffer(buffer, PendingUserValue::new(visit))?;
        check_query_position_valid("Query::visit_overlap_shape", "origin", origin)?;
        proxy.validate()?;
        let proxy = proxy.into_raw();
        let mut visit = visit.into_inner();
        buffer_transaction(buffer, |buffer| {
            authorized.invoke(|call| {
                raw::overlap_shape(&call, origin, &proxy, filter, buffer)?;
                buffer.publish(&call)
            })?;
            Ok(visit_shape_ids(buffer.as_slice(), visit.as_mut()))
        })
    }

    /// Cast a ray and collect every hit.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn cast_ray_all<V: Into<Vec2>>(
        &self,
        origin: Position,
        translation: V,
        filter: QueryFilter,
    ) -> Result<Vec<RayResult>> {
        let mut buffer = RayQueryBuffer::new();
        self.cast_ray_all_into(origin, translation, filter, &mut buffer)?;
        Ok(buffer.into_vec())
    }

    /// Cast a ray into reusable raw-and-mapped storage.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn cast_ray_all_into<V: Into<Vec2>>(
        &self,
        origin: Position,
        translation: V,
        filter: QueryFilter,
        buffer: &mut RayQueryBuffer,
    ) -> Result<()> {
        let (authorized, translation) = self.prepare_owned_buffer(buffer, translation)?;
        check_query_position_valid("Query::cast_ray_all", "origin", origin)?;
        let translation = translation.into_inner().into();
        check_query_vec2_valid("Query::cast_ray_all", "translation", translation)?;
        buffer_transaction(buffer, |buffer| {
            authorized.invoke(|call| {
                raw::cast_ray_all(&call, origin, translation, filter, buffer)?;
                buffer.publish("Query::cast_ray_all", &call)
            })
        })
    }

    /// Cast `proxy` through the world and collect every hit.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn cast_shape<V: Into<Vec2>>(
        &self,
        origin: Position,
        proxy: ShapeProxy,
        translation: V,
        filter: QueryFilter,
    ) -> Result<Vec<RayResult>> {
        let mut buffer = RayQueryBuffer::new();
        self.cast_shape_into(origin, proxy, translation, filter, &mut buffer)?;
        Ok(buffer.into_vec())
    }

    /// Cast `proxy` through the world into reusable storage.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn cast_shape_into<V: Into<Vec2>>(
        &self,
        origin: Position,
        proxy: ShapeProxy,
        translation: V,
        filter: QueryFilter,
        buffer: &mut RayQueryBuffer,
    ) -> Result<()> {
        let (authorized, translation) = self.prepare_owned_buffer(buffer, translation)?;
        check_query_position_valid("Query::cast_shape", "origin", origin)?;
        proxy.validate()?;
        let translation = translation.into_inner().into();
        check_query_vec2_valid("Query::cast_shape", "translation", translation)?;
        let proxy = proxy.into_raw();
        buffer_transaction(buffer, |buffer| {
            authorized.invoke(|call| {
                raw::cast_shape(&call, origin, &proxy, translation, filter, buffer)?;
                buffer.publish("Query::cast_shape", &call)
            })
        })
    }

    /// Collect collision planes for a capsule mover.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn collide_mover<V1: Into<Vec2>, V2: Into<Vec2>>(
        &self,
        origin: Position,
        c1: V1,
        c2: V2,
        radius: f32,
        filter: QueryFilter,
    ) -> Result<Vec<MoverPlaneResult>> {
        let mut buffer = MoverQueryBuffer::new();
        self.collide_mover_into(origin, c1, c2, radius, filter, &mut buffer)?;
        core::result::Result::Ok(buffer.into_vec())
    }

    /// Collect capsule-mover collision planes into reusable storage.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn collide_mover_into<V1: Into<Vec2>, V2: Into<Vec2>>(
        &self,
        origin: Position,
        c1: V1,
        c2: V2,
        radius: f32,
        filter: QueryFilter,
        buffer: &mut MoverQueryBuffer,
    ) -> Result<()> {
        let (authorized, inputs) = self.prepare_owned_buffer(buffer, (c1, c2))?;
        check_query_position_valid("Query::collide_mover", "origin", origin)?;
        check_query_mover_radius_valid("Query::collide_mover", radius)?;
        let (c1, c2) = inputs.into_inner();
        let c1 = c1.into();
        let c2 = c2.into();
        check_query_vec2_valid("Query::collide_mover", "c1", c1)?;
        check_query_vec2_valid("Query::collide_mover", "c2", c2)?;
        buffer_transaction(buffer, |buffer| {
            authorized.invoke(|call| {
                raw::collide_mover(&call, origin, c1, c2, radius, filter, buffer)?;
                buffer.publish("Query::collide_mover", &call)
            })
        })
    }

    /// Cross owner preflight without committing caller-owned generic inputs.
    fn begin_owned_operation<'query, T>(
        &'query self,
        operation: T,
    ) -> Result<(QueryCallGuard<'query, 'owner>, PendingUserValue<T>)> {
        let operation = PendingUserValue::new(operation);
        let authorized = self.proof.begin()?;
        Ok((authorized, operation))
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// Clear reusable output and cross preflight while retaining the input cleanup boundary.
    fn prepare_owned_buffer<'query, B, T>(
        &'query self,
        buffer: &mut B,
        operation: T,
    ) -> Result<(QueryCallGuard<'query, 'owner>, PendingUserValue<T>)>
    where
        B: QueryBuffer,
    {
        let operation = PendingUserValue::new(operation);
        let authorized = self.prepare_buffer(buffer)?;
        Ok((authorized, operation))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn prepare_buffer<'query, B: QueryBuffer>(
        &'query self,
        buffer: &mut B,
    ) -> Result<QueryCallGuard<'query, 'owner>> {
        buffer.clear();
        self.proof.begin()
    }
}

impl crate::World {
    /// Acquire a read-only query capability for this world.
    pub fn query(&self) -> Result<Query<'_>> {
        Query::new(self)
    }
}

impl crate::RecordingSession<'_> {
    /// Acquire the same query capability while appending native operations to this recording.
    pub fn query(&self) -> Result<Query<'_>> {
        Query::new(self)
    }
}

fn map_closest_ray_result(
    call: &QueryCall<'_>,
    raw: ffi::b2RayResult,
) -> Result<ClosestRayCastResult> {
    const OPERATION: &str = "Query::cast_ray_closest";
    validate_native_query_counters(OPERATION, raw.nodeVisits, raw.leafVisits)?;
    let hit = if raw.hit {
        let output = validate_native_ray_output(OPERATION, raw.point, raw.normal, raw.fraction)?;
        core::option::Option::Some(output.with_shape(call.resolve_shape(raw.shapeId)?))
    } else {
        core::option::Option::None
    };
    core::result::Result::Ok(ClosestRayCastResult {
        hit,
        node_visits: raw.nodeVisits,
        leaf_visits: raw.leafVisits,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn buffer_transaction<T, B>(
    buffer: &mut B,
    operation: impl FnOnce(&mut B) -> Result<T>,
) -> Result<T>
where
    B: QueryBuffer,
{
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| operation(buffer))) {
        core::result::Result::Ok(result) => {
            if result.is_err() {
                buffer.clear();
            }
            result
        }
        core::result::Result::Err(payload) => {
            buffer.clear();
            std::panic::resume_unwind(payload)
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
trait QueryBuffer {
    fn clear(&mut self);
}

#[cfg(not(target_arch = "wasm32"))]
impl QueryBuffer for ShapeQueryBuffer {
    fn clear(&mut self) {
        ShapeQueryBuffer::clear(self);
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl QueryBuffer for RayQueryBuffer {
    fn clear(&mut self) {
        RayQueryBuffer::clear(self);
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl QueryBuffer for MoverQueryBuffer {
    fn clear(&mut self) {
        MoverQueryBuffer::clear(self);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn visit_shape_ids<F>(shape_ids: &[ShapeId], visit: &mut F) -> bool
where
    F: FnMut(ShapeId) -> bool,
{
    // This visitor runs after the native owner boundary and has a usable `false` fallback. Do not
    // enter `CallbackGuard`: ordinary calls resume the panic, while destructor paths preserve an
    // outer panic instead of starting a second unwind.
    let mut panic = crate::core::callback_state::PanicSlot::default();
    let completed = panic
        .capture_result(std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || {
                for &shape_id in shape_ids {
                    if !visit(shape_id) {
                        return false;
                    }
                }
                true
            },
        )))
        .unwrap_or(false);
    panic.resume_or_forget();
    completed
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::{ShapeDef, World};
    use std::cell::Cell;

    fn world_with_shape() -> World {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let body_id = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_def(),
            )
            .unwrap();
        let mut body = world.body(body_id).unwrap();
        body.create_circle(
            &ShapeDef::default(),
            &crate::shapes::Circle::new(Vec2::ZERO, 0.5).unwrap(),
        )
        .unwrap();
        world
    }

    fn invalid_aabb() -> Aabb {
        Aabb {
            lower: Vec2::new(1.0, 1.0),
            upper: Vec2::new(-1.0, -1.0),
        }
    }

    struct CountingOwner {
        world: World,
        preflights: Cell<usize>,
        postflights: Cell<usize>,
        sticky_error: Cell<Option<crate::Error>>,
        postflight_error: Cell<Option<crate::Error>>,
    }

    impl CountingOwner {
        fn new(world: World) -> Self {
            Self {
                world,
                preflights: Cell::new(0),
                postflights: Cell::new(0),
                sticky_error: Cell::new(None),
                postflight_error: Cell::new(None),
            }
        }
    }

    impl OwnerAdapter for CountingOwner {
        fn capability_core(&self) -> &WorldCore {
            self.world.core()
        }

        fn capability_completed_step(&self) -> &crate::events::CompletedStepState {
            self.world.completed_step_state()
        }

        fn capability_preflight(&self) -> Result<()> {
            self.preflights.set(self.preflights.get() + 1);
            crate::world::check_world_available(&self.world)?;
            match self.sticky_error.get() {
                Some(error) => Err(error),
                None => Ok(()),
            }
        }

        fn capability_postflight(&self) -> Result<()> {
            self.postflights.set(self.postflights.get() + 1);
            match self.postflight_error.get() {
                Some(error) => Err(error),
                None => Ok(()),
            }
        }
    }

    #[test]
    fn native_query_mapping_rejects_malformed_counters_geometry_and_fractions() {
        let world = world_with_shape();
        let call = QueryCall::for_test(world.core());
        let null_shape = ffi::b2ShapeId {
            index1: 0,
            world0: 0,
            generation: 0,
        };

        let invalid_counters = ffi::b2RayResult {
            shapeId: null_shape,
            point: Position::ZERO.into_raw(),
            normal: Vec2::ZERO.into_raw(),
            fraction: 0.0,
            nodeVisits: -1,
            leafVisits: 0,
            hit: false,
        };
        assert_eq!(
            map_closest_ray_result(&call, invalid_counters).err(),
            Some(crate::Error::InvalidNativeOutput {
                operation: "Query::cast_ray_closest",
                output: "node_visits",
                constraint: "a non-negative native int",
            })
        );
        let invalid_leaf_counter = ffi::b2RayResult {
            nodeVisits: 0,
            leafVisits: -1,
            ..invalid_counters
        };
        assert_eq!(
            map_closest_ray_result(&call, invalid_leaf_counter).err(),
            Some(crate::Error::InvalidNativeOutput {
                operation: "Query::cast_ray_closest",
                output: "leaf_visits",
                constraint: "a non-negative native int no greater than node_visits",
            })
        );
        let invalid_counter_relationship = ffi::b2RayResult {
            nodeVisits: 1,
            leafVisits: 2,
            ..invalid_counters
        };
        assert_eq!(
            map_closest_ray_result(&call, invalid_counter_relationship).err(),
            Some(crate::Error::InvalidNativeOutput {
                operation: "Query::cast_ray_closest",
                output: "leaf_visits",
                constraint: "a count no greater than node_visits",
            })
        );

        let invalid_hit_normal = ffi::b2RayResult {
            shapeId: null_shape,
            point: Position::ZERO.into_raw(),
            normal: Vec2::ZERO.into_raw(),
            fraction: 0.5,
            nodeVisits: 0,
            leafVisits: 0,
            hit: true,
        };
        assert_eq!(
            map_closest_ray_result(&call, invalid_hit_normal).err(),
            Some(crate::Error::InvalidNativeOutput {
                operation: "Query::cast_ray_closest",
                output: "normal",
                constraint: "a finite unit vector, or zero when fraction is zero",
            })
        );

        for fraction in [f32::NAN, -f32::EPSILON, 1.0 + f32::EPSILON] {
            assert_eq!(
                validate_native_fraction("Query::cast_mover", "fraction", fraction).err(),
                Some(crate::Error::InvalidNativeOutput {
                    operation: "Query::cast_mover",
                    output: "fraction",
                    constraint: "a finite value in 0.0..=1.0",
                })
            );
        }
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn visitor_runs_only_after_recording_postflight_commits() {
        let owner = CountingOwner::new(world_with_shape());
        let query = Query::new(&owner).unwrap();
        let mut buffer = ShapeQueryBuffer::with_capacity(1).unwrap();
        let pointers = buffer.storage_ptrs();
        let calls = Cell::new(0);

        let completed = query
            .visit_overlap_aabb_with_buffer(
                Position::ZERO,
                Aabb::new([-1.0_f32, -1.0], [1.0_f32, 1.0]).unwrap(),
                QueryFilter::default(),
                &mut buffer,
                |_| {
                    calls.set(calls.get() + 1);
                    assert_eq!(owner.postflights.get(), 1);
                    true
                },
            )
            .unwrap();
        assert!(completed);
        assert_eq!(calls.get(), 1);

        owner
            .postflight_error
            .set(Some(crate::Error::RecordingLimitExceeded));
        assert_eq!(
            query.visit_overlap_aabb_with_buffer(
                Position::ZERO,
                Aabb::new([-1.0_f32, -1.0], [1.0_f32, 1.0]).unwrap(),
                QueryFilter::default(),
                &mut buffer,
                |_| {
                    calls.set(calls.get() + 1);
                    true
                },
            ),
            Err(crate::Error::RecordingLimitExceeded)
        );
        assert_eq!(calls.get(), 1);
        assert_eq!(owner.postflights.get(), 2);
        assert!(buffer.is_empty());
        assert_eq!(buffer.storage_ptrs(), pointers);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn warmed_overlap_query_reuses_raw_and_mapped_storage_on_hit_and_miss() {
        let world = world_with_shape();
        let query = world.query().unwrap();
        let mut buffer = ShapeQueryBuffer::with_capacity(4).unwrap();
        let pointers = buffer.storage_ptrs();

        query
            .overlap_aabb_into(
                Position::ZERO,
                Aabb::new([-1.0_f32, -1.0], [1.0_f32, 1.0]).unwrap(),
                QueryFilter::default(),
                &mut buffer,
            )
            .unwrap();
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer.storage_ptrs(), pointers);

        query
            .overlap_aabb_into(
                Position::ZERO,
                Aabb::new([10.0_f32, 10.0], [11.0_f32, 11.0]).unwrap(),
                QueryFilter::default(),
                &mut buffer,
            )
            .unwrap();
        assert!(buffer.is_empty());
        assert_eq!(buffer.storage_ptrs(), pointers);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn each_query_operation_preflights_once_and_sticky_errors_precede_validation() {
        let owner = CountingOwner::new(world_with_shape());
        let access_checks_before = owner.world.core().access_check_count_for_test();
        let query = Query::new(&owner).unwrap();
        assert_eq!(owner.preflights.get(), 1);
        assert_eq!(owner.postflights.get(), 0);
        assert_eq!(
            owner.world.core().access_check_count_for_test(),
            access_checks_before + 1
        );

        let mut buffer = ShapeQueryBuffer::with_capacity(1).unwrap();
        query
            .overlap_aabb_into(
                Position::ZERO,
                Aabb::new([-1.0_f32, -1.0], [1.0_f32, 1.0]).unwrap(),
                QueryFilter::default(),
                &mut buffer,
            )
            .unwrap();
        let pointers = buffer.storage_ptrs();
        assert_eq!(owner.preflights.get(), 2);
        assert_eq!(owner.postflights.get(), 1);
        assert_eq!(
            owner.world.core().access_check_count_for_test(),
            access_checks_before + 2
        );
        assert!(!buffer.is_empty());

        owner
            .sticky_error
            .set(Some(crate::Error::RecordingLimitExceeded));
        assert_eq!(
            query.overlap_aabb_into(
                Position::ZERO,
                invalid_aabb(),
                QueryFilter::default(),
                &mut buffer,
            ),
            Err(crate::Error::RecordingLimitExceeded)
        );
        assert_eq!(owner.preflights.get(), 3);
        assert_eq!(owner.postflights.get(), 1);
        assert_eq!(
            owner.world.core().access_check_count_for_test(),
            access_checks_before + 3
        );
        assert!(buffer.is_empty());
        assert_eq!(buffer.storage_ptrs(), pointers);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn query_capability_keeps_the_direct_owner_at_a_stable_address() {
        let world = world_with_shape();
        let core = world.core() as *const WorldCore;
        let query = world.query().unwrap();

        assert_eq!(world.core() as *const WorldCore, core);
        let mut observed_during_call = core::ptr::null();
        let completed = query
            .visit_overlap_aabb(
                Position::ZERO,
                Aabb::new([-1.0_f32, -1.0], [1.0_f32, 1.0]).unwrap(),
                QueryFilter::default(),
                |_| {
                    observed_during_call = world.core() as *const WorldCore;
                    false
                },
            )
            .unwrap();
        assert!(!completed);
        assert_eq!(observed_during_call, core);
        assert_eq!(world.core() as *const WorldCore, core);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn operation_preflight_precedes_validation_and_clears_visible_output() {
        let world = world_with_shape();
        let query = world.query().unwrap();
        let mut buffer = ShapeQueryBuffer::with_capacity(1).unwrap();
        query
            .overlap_aabb_into(
                Position::ZERO,
                Aabb::new([-1.0_f32, -1.0], [1.0_f32, 1.0]).unwrap(),
                QueryFilter::default(),
                &mut buffer,
            )
            .unwrap();
        let pointers = buffer.storage_ptrs();
        assert!(!buffer.is_empty());

        let _callback = crate::core::callback_state::CallbackGuard::enter();
        assert_eq!(
            query.cast_mover(
                Position::ZERO,
                Vec2::ZERO,
                Vec2::ZERO,
                f32::NAN,
                Vec2::ZERO,
                QueryFilter::default(),
            ),
            Err(crate::Error::InCallback)
        );
        assert_eq!(
            query.overlap_aabb_into(
                Position::ZERO,
                invalid_aabb(),
                QueryFilter::default(),
                &mut buffer,
            ),
            Err(crate::Error::InCallback)
        );
        assert!(buffer.is_empty());
        assert_eq!(buffer.storage_ptrs(), pointers);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn owned_generic_rejection_cleanup_during_outer_unwind_does_not_abort() {
        const CHILD: &str = "BOXDD_OUTER_UNWIND_REJECTED_QUERY_GENERIC";
        const TEST_NAME: &str = "query::capability::tests::owned_generic_rejection_cleanup_during_outer_unwind_does_not_abort";
        const PREFLIGHT_PRIMARY: &str = "outer rejected query-generic preflight remains primary";

        struct PanickingVecInput {
            dropped: std::rc::Rc<Cell<usize>>,
            converted: std::rc::Rc<Cell<bool>>,
            armed: bool,
        }

        impl From<PanickingVecInput> for Vec2 {
            fn from(mut input: PanickingVecInput) -> Self {
                input.converted.set(true);
                input.armed = false;
                Vec2::ZERO
            }
        }

        impl Drop for PanickingVecInput {
            fn drop(&mut self) {
                if self.armed {
                    self.dropped.set(self.dropped.get() + 1);
                    panic!("secondary rejected query-generic cleanup panic");
                }
            }
        }

        struct InvokeOnDrop<F: FnOnce()>(Option<F>);

        impl<F: FnOnce()> Drop for InvokeOnDrop<F> {
            fn drop(&mut self) {
                if let Some(invoke) = self.0.take() {
                    invoke();
                }
            }
        }

        if std::env::var_os(CHILD).is_some() {
            let owner = CountingOwner::new(world_with_shape());
            let query = Query::new(&owner).unwrap();

            let generic_dropped = std::rc::Rc::new(Cell::new(0));
            let generic_converted = std::rc::Rc::new(Cell::new(false));
            let preflight_rejected = std::rc::Rc::new(Cell::new(false));
            owner
                .sticky_error
                .set(Some(crate::Error::RecordingLimitExceeded));
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let dropped = std::rc::Rc::clone(&generic_dropped);
                let converted = std::rc::Rc::clone(&generic_converted);
                let rejected = std::rc::Rc::clone(&preflight_rejected);
                let _operation = InvokeOnDrop(Some(|| {
                    rejected.set(matches!(
                        query.cast_ray_closest(
                            Position::ZERO,
                            PanickingVecInput {
                                dropped,
                                converted,
                                armed: true,
                            },
                            QueryFilter::default(),
                        ),
                        Err(crate::Error::RecordingLimitExceeded)
                    ));
                }));
                std::panic::panic_any(PREFLIGHT_PRIMARY);
            }));
            let payload = result.expect_err("the preflight outer panic must keep unwinding");
            assert_eq!(
                payload.downcast_ref::<&'static str>(),
                Some(&PREFLIGHT_PRIMARY)
            );
            assert!(preflight_rejected.get());
            assert_eq!(generic_dropped.get(), 1);
            assert!(!generic_converted.get());

            eprintln!("boxdd-outer-unwind-rejected-query-generics: completed");
            return;
        }

        let output = std::process::Command::new(
            std::env::current_exe().expect("test executable path must be available"),
        )
        .args(["--exact", TEST_NAME, "--nocapture"])
        .env(CHILD, "1")
        .output()
        .expect("outer-unwind rejected query-generics child process must start");
        assert!(
            output.status.success(),
            "outer-unwind rejected query-generics child aborted\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("boxdd-outer-unwind-rejected-query-generics: completed"),
            "outer-unwind rejected query-generics child did not complete\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn post_native_visitor_drops_foreign_world_immediately_before_resuming_panic() {
        let query_world = world_with_shape();
        let foreign_world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let foreign_raw = foreign_world.raw();
        let mut foreign_world = Some(foreign_world);
        let query = query_world.query().unwrap();
        let mut buffer = ShapeQueryBuffer::with_capacity(1).unwrap();
        let pointers = buffer.storage_ptrs();

        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = query.visit_overlap_aabb_with_buffer(
                Position::ZERO,
                Aabb::new([-1.0_f32, -1.0], [1.0_f32, 1.0]).unwrap(),
                QueryFilter::default(),
                &mut buffer,
                |_| {
                    drop(foreign_world.take());
                    assert!(!unsafe { boxdd_sys::ffi::b2World_IsValid(foreign_raw) });
                    panic!("query visitor panic");
                },
            );
        }));

        assert!(panic.is_err());
        assert!(buffer.is_empty());
        assert_eq!(buffer.storage_ptrs(), pointers);
        assert!(!unsafe { boxdd_sys::ffi::b2World_IsValid(foreign_raw) });
    }
}
