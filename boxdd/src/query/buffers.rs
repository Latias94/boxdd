#[cfg(not(target_arch = "wasm32"))]
use core::{fmt, slice};

use boxdd_sys::ffi;

use crate::error::{Error, Result};
#[cfg(not(target_arch = "wasm32"))]
use crate::world::QueryCall;
#[cfg(not(target_arch = "wasm32"))]
use crate::{MoverPlaneResult, Plane};
use crate::{Position, RayResult, ShapeId, Vec2};

#[cfg(not(target_arch = "wasm32"))]
macro_rules! impl_query_buffer_collection {
    ($buffer:ty, $item:ty) => {
        impl $buffer {
            pub fn new() -> Self {
                Self::default()
            }

            pub fn with_capacity(capacity: usize) -> Result<Self> {
                let mut buffer = Self::new();
                buffer.reserve(capacity)?;
                Ok(buffer)
            }

            pub fn reserve(&mut self, additional: usize) -> Result<()> {
                self.raw
                    .try_reserve(additional)
                    .map_err(|_| Error::FfiOutputAllocationFailed)?;
                self.mapped
                    .try_reserve(additional)
                    .map_err(|_| Error::FfiOutputAllocationFailed)
            }

            pub fn as_slice(&self) -> &[$item] {
                &self.mapped
            }

            pub fn iter(&self) -> slice::Iter<'_, $item> {
                self.mapped.iter()
            }

            pub fn len(&self) -> usize {
                self.mapped.len()
            }

            pub fn is_empty(&self) -> bool {
                self.mapped.is_empty()
            }

            pub fn capacity(&self) -> usize {
                self.raw.capacity().min(self.mapped.capacity())
            }

            pub fn clear(&mut self) {
                self.raw.clear();
                self.mapped.clear();
            }

            pub fn into_vec(self) -> Vec<$item> {
                self.mapped
            }
        }

        impl AsRef<[$item]> for $buffer {
            fn as_ref(&self) -> &[$item] {
                self.as_slice()
            }
        }

        impl core::ops::Deref for $buffer {
            type Target = [$item];

            fn deref(&self) -> &Self::Target {
                self.as_slice()
            }
        }

        impl<'a> IntoIterator for &'a $buffer {
            type Item = &'a $item;
            type IntoIter = slice::Iter<'a, $item>;

            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }

        impl IntoIterator for $buffer {
            type Item = $item;
            type IntoIter = std::vec::IntoIter<$item>;

            fn into_iter(self) -> Self::IntoIter {
                self.mapped.into_iter()
            }
        }
    };
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Copy, Clone)]
pub(super) struct RawRayHit {
    pub(super) shape_id: ffi::b2ShapeId,
    pub(super) point: ffi::b2Pos,
    pub(super) normal: ffi::b2Vec2,
    pub(super) fraction: f32,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Copy, Clone)]
pub(super) struct RawMoverPlane {
    pub(super) shape_id: ffi::b2ShapeId,
    pub(super) plane: ffi::b2PlaneResult,
}

#[derive(Copy, Clone)]
pub(super) struct ValidatedRayOutput {
    point: Position,
    normal: Vec2,
    fraction: f32,
}

impl ValidatedRayOutput {
    #[inline]
    pub(super) fn with_shape(self, shape_id: ShapeId) -> RayResult {
        RayResult {
            shape_id,
            point: self.point,
            normal: self.normal,
            fraction: self.fraction,
            hit: true,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Copy, Clone)]
struct ValidatedMoverPlane {
    plane: Plane,
    point: Vec2,
}

#[cfg(not(target_arch = "wasm32"))]
impl ValidatedMoverPlane {
    #[inline]
    fn with_shape(self, shape_id: ShapeId) -> MoverPlaneResult {
        MoverPlaneResult {
            shape_id,
            plane: self.plane,
            point: self.point,
            hit: true,
        }
    }
}

#[inline]
fn invalid_native_output(
    operation: &'static str,
    output: &'static str,
    constraint: &'static str,
) -> Error {
    Error::InvalidNativeOutput {
        operation,
        output,
        constraint,
    }
}

#[inline]
fn unit_vector_is_valid(value: Vec2) -> bool {
    value.is_valid() && (1.0 - (value.x * value.x + value.y * value.y)).abs() < 100.0 * f32::EPSILON
}

pub(super) fn validate_native_fraction(
    operation: &'static str,
    output: &'static str,
    fraction: f32,
) -> Result<f32> {
    if fraction.is_finite() && (0.0..=1.0).contains(&fraction) {
        Ok(fraction)
    } else {
        Err(invalid_native_output(
            operation,
            output,
            "a finite value in 0.0..=1.0",
        ))
    }
}

pub(super) fn validate_native_ray_output(
    operation: &'static str,
    point: ffi::b2Pos,
    normal: ffi::b2Vec2,
    fraction: f32,
) -> Result<ValidatedRayOutput> {
    let point = Position::from_raw(point);
    if !point.is_valid() {
        return core::result::Result::Err(invalid_native_output(
            operation,
            "point",
            "a finite world position",
        ));
    }
    let fraction = validate_native_fraction(operation, "fraction", fraction)?;
    let normal = Vec2::from_raw(normal);
    let is_initial_overlap = fraction == 0.0 && normal == Vec2::ZERO;
    if !is_initial_overlap && !unit_vector_is_valid(normal) {
        return core::result::Result::Err(invalid_native_output(
            operation,
            "normal",
            "a finite unit vector, or zero when fraction is zero",
        ));
    }
    Ok(ValidatedRayOutput {
        point,
        normal,
        fraction,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn validate_native_mover_plane(
    operation: &'static str,
    raw: ffi::b2PlaneResult,
) -> Result<ValidatedMoverPlane> {
    if !raw.hit {
        return core::result::Result::Err(invalid_native_output(
            operation,
            "hit",
            "true for a delivered mover collision plane",
        ));
    }
    let plane = Plane::from_raw_unvalidated(raw.plane);
    if !plane.is_valid() {
        return core::result::Result::Err(invalid_native_output(
            operation,
            "plane",
            "a finite plane with a unit normal",
        ));
    }
    let point = Vec2::from_raw(raw.point);
    if !point.is_valid() {
        return core::result::Result::Err(invalid_native_output(
            operation,
            "point",
            "a finite vector",
        ));
    }
    core::result::Result::Ok(ValidatedMoverPlane { plane, point })
}

pub(super) fn validate_native_query_counters(
    operation: &'static str,
    node_visits: i32,
    leaf_visits: i32,
) -> Result<()> {
    if node_visits < 0 {
        return Err(invalid_native_output(
            operation,
            "node_visits",
            "a non-negative native int",
        ));
    }
    if leaf_visits < 0 {
        return Err(invalid_native_output(
            operation,
            "leaf_visits",
            "a non-negative native int no greater than node_visits",
        ));
    }
    if leaf_visits > node_visits {
        return Err(invalid_native_output(
            operation,
            "leaf_visits",
            "a count no greater than node_visits",
        ));
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn publish_batch<T>(
    mapped: &mut Vec<T>,
    count: usize,
    mut map: impl FnMut(usize) -> Result<T>,
) -> Result<()> {
    mapped.clear();
    mapped
        .try_reserve(count)
        .map_err(|_| Error::FfiOutputAllocationFailed)?;
    for index in 0..count {
        mapped.push(map(index)?);
    }
    core::result::Result::Ok(())
}

/// Reusable raw-and-mapped storage for overlap query results.
///
/// A query first fills the private native-ID buffer, then binds the complete batch through the
/// owning world's registry. [`Self::as_slice`] is empty whenever validation, native output
/// collection, identity binding, or allocation fails.
#[derive(Default)]
#[cfg(not(target_arch = "wasm32"))]
pub struct ShapeQueryBuffer {
    pub(super) raw: Vec<ffi::b2ShapeId>,
    mapped: Vec<ShapeId>,
}

#[cfg(not(target_arch = "wasm32"))]
impl ShapeQueryBuffer {
    #[cfg(test)]
    pub(super) fn begin(&mut self) {
        self.clear();
    }

    pub(super) fn push_raw(&mut self, raw: ffi::b2ShapeId) -> Result<()> {
        self.raw
            .try_reserve(1)
            .map_err(|_| Error::FfiOutputAllocationFailed)?;
        self.raw.push(raw);
        Ok(())
    }

    pub(super) fn publish(&mut self, call: &QueryCall<'_>) -> Result<()> {
        let raw = &self.raw;
        let mapped = &mut self.mapped;
        let result = call.with_output_identity_resolver(|resolver| {
            publish_batch(mapped, raw.len(), |index| resolver.active_shape(raw[index]))
        });
        if result.is_err() {
            self.clear();
        }
        result
    }

    #[cfg(test)]
    pub(super) fn storage_ptrs(&self) -> (*const ffi::b2ShapeId, *const ShapeId) {
        (self.raw.as_ptr(), self.mapped.as_ptr())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl_query_buffer_collection!(ShapeQueryBuffer, ShapeId);

#[cfg(not(target_arch = "wasm32"))]
impl fmt::Debug for ShapeQueryBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ShapeQueryBuffer")
            .field("results", &self.mapped)
            .field("capacity", &self.capacity())
            .finish()
    }
}

/// Reusable raw and mapped storage for ray and shape-cast results.
#[derive(Default)]
#[cfg(not(target_arch = "wasm32"))]
pub struct RayQueryBuffer {
    pub(super) raw: Vec<RawRayHit>,
    mapped: Vec<RayResult>,
}

#[cfg(not(target_arch = "wasm32"))]
impl RayQueryBuffer {
    #[cfg(test)]
    pub(super) fn begin(&mut self) {
        self.clear();
    }

    pub(super) fn push_raw(&mut self, raw: RawRayHit) -> Result<()> {
        self.raw
            .try_reserve(1)
            .map_err(|_| Error::FfiOutputAllocationFailed)?;
        self.raw.push(raw);
        Ok(())
    }

    pub(super) fn publish(&mut self, operation: &'static str, call: &QueryCall<'_>) -> Result<()> {
        let raw = &self.raw;
        let mapped = &mut self.mapped;
        let result = call.with_output_identity_resolver(|resolver| {
            publish_batch(mapped, raw.len(), |index| {
                let raw = raw[index];
                let output =
                    validate_native_ray_output(operation, raw.point, raw.normal, raw.fraction)?;
                Ok(output.with_shape(resolver.active_shape(raw.shape_id)?))
            })
        });
        if let Err(error) = result {
            self.clear();
            return Err(error);
        }
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn storage_ptrs(&self) -> (*const RawRayHit, *const RayResult) {
        (self.raw.as_ptr(), self.mapped.as_ptr())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl_query_buffer_collection!(RayQueryBuffer, RayResult);

#[cfg(not(target_arch = "wasm32"))]
impl fmt::Debug for RayQueryBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RayQueryBuffer")
            .field("results", &self.mapped)
            .field("capacity", &self.capacity())
            .finish()
    }
}

/// Reusable raw and mapped storage for mover collision planes.
#[derive(Default)]
#[cfg(not(target_arch = "wasm32"))]
pub struct MoverQueryBuffer {
    pub(super) raw: Vec<RawMoverPlane>,
    mapped: Vec<MoverPlaneResult>,
}

#[cfg(not(target_arch = "wasm32"))]
impl MoverQueryBuffer {
    #[cfg(test)]
    pub(super) fn begin(&mut self) {
        self.clear();
    }

    pub(super) fn push_raw(&mut self, raw: RawMoverPlane) -> Result<()> {
        self.raw
            .try_reserve(1)
            .map_err(|_| Error::FfiOutputAllocationFailed)?;
        self.raw.push(raw);
        Ok(())
    }

    pub(super) fn publish(&mut self, operation: &'static str, call: &QueryCall<'_>) -> Result<()> {
        let raw = &self.raw;
        let mapped = &mut self.mapped;
        let result = call.with_output_identity_resolver(|resolver| {
            publish_batch(mapped, raw.len(), |index| {
                let raw = raw[index];
                let output = validate_native_mover_plane(operation, raw.plane)?;
                Ok(output.with_shape(resolver.active_shape(raw.shape_id)?))
            })
        });
        if let Err(error) = result {
            self.clear();
            return Err(error);
        }
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn storage_ptrs(&self) -> (*const RawMoverPlane, *const MoverPlaneResult) {
        (self.raw.as_ptr(), self.mapped.as_ptr())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl_query_buffer_collection!(MoverQueryBuffer, MoverPlaneResult);

#[cfg(not(target_arch = "wasm32"))]
impl fmt::Debug for MoverQueryBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MoverQueryBuffer")
            .field("results", &self.mapped)
            .field("capacity", &self.capacity())
            .finish()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::world::QueryCall;
    use crate::{ShapeDef, World};

    fn world_with_shape() -> (World, ShapeId) {
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
        let shape_id = {
            let mut body = world.body(body_id).unwrap();
            body.create_circle(
                &ShapeDef::default(),
                &crate::shapes::Circle::new(Vec2::ZERO, 0.5).unwrap(),
            )
            .unwrap()
        };
        (world, shape_id)
    }

    fn invalid_shape_raw(world: &World) -> ffi::b2ShapeId {
        ffi::b2ShapeId {
            index1: i32::MAX,
            world0: world.brand().world0(),
            generation: 0,
        }
    }

    #[test]
    fn query_buffers_expose_standard_read_only_collection_protocols() {
        let shapes = ShapeQueryBuffer::new();
        let _: &[ShapeId] = shapes.as_ref();
        assert_eq!((&shapes).into_iter().count(), 0);
        let _: Vec<ShapeId> = shapes.into_iter().collect();

        let rays = RayQueryBuffer::new();
        let _: &[RayResult] = &rays;
        assert_eq!((&rays).into_iter().count(), 0);
        let _: Vec<RayResult> = rays.into_iter().collect();

        let movers = MoverQueryBuffer::new();
        let _: &[MoverPlaneResult] = movers.as_ref();
        assert_eq!((&movers).into_iter().count(), 0);
        let _: Vec<MoverPlaneResult> = movers.into_iter().collect();
    }

    #[test]
    fn warmed_shape_buffer_reuses_both_storages_and_failure_is_transactional() {
        let (world, shape_id) = world_with_shape();
        let mut buffer = ShapeQueryBuffer::with_capacity(2).unwrap();
        let pointers = buffer.storage_ptrs();

        buffer.push_raw(shape_id.into_raw()).unwrap();
        buffer.publish(&QueryCall::for_test(world.core())).unwrap();
        assert_eq!(buffer.as_slice(), &[shape_id]);
        assert_eq!(buffer.storage_ptrs(), pointers);

        buffer.begin();
        buffer.publish(&QueryCall::for_test(world.core())).unwrap();
        assert!(buffer.is_empty());
        assert_eq!(buffer.storage_ptrs(), pointers);

        buffer.begin();
        buffer.push_raw(shape_id.into_raw()).unwrap();
        buffer.push_raw(invalid_shape_raw(&world)).unwrap();
        assert_eq!(
            buffer.publish(&QueryCall::for_test(world.core())),
            Err(Error::InvalidShapeId)
        );
        assert!(buffer.is_empty());
        assert_eq!(buffer.storage_ptrs(), pointers);
    }

    #[test]
    fn warmed_ray_buffer_reuses_raw_and_mapped_storage() {
        let (world, shape_id) = world_with_shape();
        let mut buffer = RayQueryBuffer::with_capacity(2).unwrap();
        let pointers = buffer.storage_ptrs();

        buffer
            .push_raw(RawRayHit {
                shape_id: shape_id.into_raw(),
                point: Position::ZERO.into_raw(),
                normal: Vec2::new(1.0, 0.0).into_raw(),
                fraction: 0.5,
            })
            .unwrap();
        buffer
            .publish("test_ray_query", &QueryCall::for_test(world.core()))
            .unwrap();
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer.as_slice()[0].shape_id, shape_id);
        assert_eq!(buffer.storage_ptrs(), pointers);

        buffer.begin();
        buffer
            .push_raw(RawRayHit {
                shape_id: shape_id.into_raw(),
                point: Position::ZERO.into_raw(),
                normal: Vec2::new(1.0, 0.0).into_raw(),
                fraction: 0.25,
            })
            .unwrap();
        buffer
            .push_raw(RawRayHit {
                shape_id: shape_id.into_raw(),
                point: Position::new(crate::WorldScalar::NAN, 0.0).into_raw(),
                normal: Vec2::new(1.0, 0.0).into_raw(),
                fraction: 0.5,
            })
            .unwrap();
        assert_eq!(
            buffer.publish("test_ray_query", &QueryCall::for_test(world.core())),
            Err(Error::InvalidNativeOutput {
                operation: "test_ray_query",
                output: "point",
                constraint: "a finite world position",
            })
        );
        assert!(buffer.raw.is_empty());
        assert!(buffer.is_empty());
        assert_eq!(buffer.storage_ptrs(), pointers);

        buffer.begin();
        buffer
            .push_raw(RawRayHit {
                shape_id: shape_id.into_raw(),
                point: Position::ZERO.into_raw(),
                normal: Vec2::new(1.0, 0.0).into_raw(),
                fraction: 0.25,
            })
            .unwrap();
        buffer
            .push_raw(RawRayHit {
                shape_id: invalid_shape_raw(&world),
                point: Position::ZERO.into_raw(),
                normal: Vec2::new(1.0, 0.0).into_raw(),
                fraction: 0.5,
            })
            .unwrap();
        assert_eq!(
            buffer.publish("test_ray_query", &QueryCall::for_test(world.core())),
            Err(Error::InvalidShapeId)
        );
        assert!(buffer.is_empty());
        assert_eq!(buffer.storage_ptrs(), pointers);
    }

    #[test]
    fn ray_buffer_accepts_box2d_initial_overlap_output() {
        let (world, shape_id) = world_with_shape();
        let mut buffer = RayQueryBuffer::new();

        buffer
            .push_raw(RawRayHit {
                shape_id: shape_id.into_raw(),
                point: Position::ZERO.into_raw(),
                normal: Vec2::ZERO.into_raw(),
                fraction: 0.0,
            })
            .unwrap();
        buffer
            .publish("test_ray_query", &QueryCall::for_test(world.core()))
            .unwrap();

        let hit = buffer.as_slice()[0];
        assert_eq!(hit.shape_id, shape_id);
        assert_eq!(hit.normal, Vec2::ZERO);
        assert_eq!(hit.fraction, 0.0);
    }

    #[test]
    fn warmed_mover_buffer_reuses_raw_and_mapped_storage() {
        let (world, shape_id) = world_with_shape();
        let mut buffer = MoverQueryBuffer::with_capacity(2).unwrap();
        let pointers = buffer.storage_ptrs();
        let raw_plane = ffi::b2PlaneResult {
            plane: ffi::b2Plane {
                normal: Vec2::new(0.0, 1.0).into_raw(),
                offset: 0.0,
            },
            point: Vec2::ZERO.into_raw(),
            hit: true,
        };

        buffer
            .push_raw(RawMoverPlane {
                shape_id: shape_id.into_raw(),
                plane: raw_plane,
            })
            .unwrap();
        buffer
            .publish("test_mover_query", &QueryCall::for_test(world.core()))
            .unwrap();
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer.as_slice()[0].shape_id, shape_id);
        assert_eq!(buffer.storage_ptrs(), pointers);

        buffer.begin();
        buffer
            .push_raw(RawMoverPlane {
                shape_id: shape_id.into_raw(),
                plane: raw_plane,
            })
            .unwrap();
        buffer
            .push_raw(RawMoverPlane {
                shape_id: shape_id.into_raw(),
                plane: ffi::b2PlaneResult {
                    plane: ffi::b2Plane {
                        normal: Vec2::new(0.0, 2.0).into_raw(),
                        offset: 0.0,
                    },
                    point: Vec2::ZERO.into_raw(),
                    hit: true,
                },
            })
            .unwrap();
        assert_eq!(
            buffer.publish("test_mover_query", &QueryCall::for_test(world.core())),
            Err(Error::InvalidNativeOutput {
                operation: "test_mover_query",
                output: "plane",
                constraint: "a finite plane with a unit normal",
            })
        );
        assert!(buffer.raw.is_empty());
        assert!(buffer.is_empty());
        assert_eq!(buffer.storage_ptrs(), pointers);

        buffer.begin();
        buffer
            .push_raw(RawMoverPlane {
                shape_id: shape_id.into_raw(),
                plane: raw_plane,
            })
            .unwrap();
        buffer
            .push_raw(RawMoverPlane {
                shape_id: shape_id.into_raw(),
                plane: ffi::b2PlaneResult {
                    hit: false,
                    ..raw_plane
                },
            })
            .unwrap();
        assert_eq!(
            buffer.publish("test_mover_query", &QueryCall::for_test(world.core())),
            Err(Error::InvalidNativeOutput {
                operation: "test_mover_query",
                output: "hit",
                constraint: "true for a delivered mover collision plane",
            })
        );
        assert!(buffer.raw.is_empty());
        assert!(buffer.is_empty());
        assert_eq!(buffer.storage_ptrs(), pointers);

        buffer.begin();
        buffer
            .push_raw(RawMoverPlane {
                shape_id: shape_id.into_raw(),
                plane: raw_plane,
            })
            .unwrap();
        buffer
            .push_raw(RawMoverPlane {
                shape_id: invalid_shape_raw(&world),
                plane: raw_plane,
            })
            .unwrap();
        assert_eq!(
            buffer.publish("test_mover_query", &QueryCall::for_test(world.core())),
            Err(Error::InvalidShapeId)
        );
        assert!(buffer.is_empty());
        assert_eq!(buffer.storage_ptrs(), pointers);
    }
}
