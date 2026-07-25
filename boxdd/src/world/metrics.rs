use super::*;
use core::num::NonZeroU8;

/// Maximum number of workers supported by the pinned Box2D ABI.
pub const B2_MAX_WORKERS: u8 = 32;

/// Whether the selected 0.6 runtime adapter is qualified for multiple workers.
///
/// Native targets use the pinned Box2D scheduler. Current WASM adapters are
/// intentionally single-worker: Rust atomics alone do not prove that the
/// linked provider, memory, and host were qualified for Emscripten pthreads.
#[inline]
pub const fn target_supports_multiple_workers() -> bool {
    !cfg!(target_family = "wasm")
}

/// A validated runtime worker count.
///
/// Box2D's runtime setter accepts only values in `[1, B2_MAX_WORKERS]`. A
/// WebAssembly target can only use one worker until a pthread/shared-memory
/// provider has its own runtime-qualified ABI capability.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct WorkerCount(NonZeroU8);

impl WorkerCount {
    /// Smallest worker count accepted by the safe API.
    pub const MIN: u8 = 1;
    /// Largest worker count supported by the pinned Box2D ABI.
    pub const MAX: u8 = B2_MAX_WORKERS;

    /// Construct a worker count after checking the native range and target capabilities.
    pub fn new(value: u32) -> crate::error::ApiResult<Self> {
        let value_u8 = u8::try_from(value).map_err(|_| crate::error::ApiError::InvalidArgument)?;
        if !(Self::MIN..=Self::MAX).contains(&value_u8) {
            return Err(crate::error::ApiError::InvalidArgument);
        }
        if !Self::supports(value_u8) {
            return Err(crate::error::ApiError::UnsupportedWorkerCount { requested: value });
        }
        // The range check above proves this conversion cannot be zero.
        Ok(Self(
            NonZeroU8::new(value_u8).expect("worker count range excludes zero"),
        ))
    }

    #[inline]
    pub const fn get(self) -> u8 {
        self.0.get()
    }

    #[inline]
    pub(crate) const fn as_i32(self) -> i32 {
        self.get() as i32
    }

    /// Return whether a count is valid for this target and Box2D ABI.
    #[inline]
    pub const fn supports(value: u8) -> bool {
        value >= Self::MIN
            && value <= Self::MAX
            && (value == Self::MIN || target_supports_multiple_workers())
    }

    pub(crate) fn from_native(value: i32) -> crate::error::ApiResult<Self> {
        Self::new(u32::try_from(value).map_err(|_| crate::error::ApiError::InvalidArgument)?)
    }
}

impl TryFrom<u32> for WorkerCount {
    type Error = crate::error::ApiError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<i32> for WorkerCount {
    type Error = crate::error::ApiError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        Self::new(u32::try_from(value).map_err(|_| crate::error::ApiError::InvalidArgument)?)
    }
}

impl Default for WorkerCount {
    fn default() -> Self {
        // One worker is valid on every supported target.
        Self(NonZeroU8::new(Self::MIN).expect("worker count minimum is non-zero"))
    }
}

impl From<WorkerCount> for u8 {
    fn from(value: WorkerCount) -> Self {
        value.get()
    }
}

/// Validated initial or observed Box2D world capacities.
///
/// Native capacity fields are signed `int`s. This value type keeps them
/// non-negative and also rejects values that would overflow Box2D's signed
/// capacity sums, dynamic-tree sizing, or contact-table sizing arithmetic.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct WorldCapacity {
    static_shape_count: i32,
    dynamic_shape_count: i32,
    static_body_count: i32,
    dynamic_body_count: i32,
    contact_count: i32,
}

#[cfg(feature = "serde")]
impl serde::Serialize for WorkerCount {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u8(self.get())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for WorkerCount {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u32::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for WorldCapacity {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("WorldCapacity", 5)?;
        state.serialize_field("static_shape_count", &self.static_shape_count())?;
        state.serialize_field("dynamic_shape_count", &self.dynamic_shape_count())?;
        state.serialize_field("static_body_count", &self.static_body_count())?;
        state.serialize_field("dynamic_body_count", &self.dynamic_body_count())?;
        state.serialize_field("contact_count", &self.contact_count())?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for WorldCapacity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Repr {
            static_shape_count: u64,
            dynamic_shape_count: u64,
            static_body_count: u64,
            dynamic_body_count: u64,
            contact_count: u64,
        }
        let value = Repr::deserialize(deserializer)?;
        Self::new(
            value.static_shape_count,
            value.dynamic_shape_count,
            value.static_body_count,
            value.dynamic_body_count,
            value.contact_count,
        )
        .map_err(serde::de::Error::custom)
    }
}

impl WorldCapacity {
    const MAX_NATIVE: u64 = i32::MAX as u64;
    // Dynamic trees compute `2 * capacity - 1` in signed native arithmetic.
    const MAX_SHAPE_COUNT: u64 = (i32::MAX as u64).div_ceil(2);
    // The broad phase doubles this value and rounds it to a signed power of two.
    const MAX_CONTACT_COUNT: u64 = 1_u64 << 29;

    /// Construct capacities that are valid for Box2D's native allocation arithmetic.
    pub fn new(
        static_shape_count: u64,
        dynamic_shape_count: u64,
        static_body_count: u64,
        dynamic_body_count: u64,
        contact_count: u64,
    ) -> crate::error::ApiResult<Self> {
        if static_shape_count > Self::MAX_SHAPE_COUNT
            || dynamic_shape_count > Self::MAX_SHAPE_COUNT
            || contact_count > Self::MAX_CONTACT_COUNT
            || !Self::sum_fits_native(static_shape_count, dynamic_shape_count)
            || !Self::sum_fits_native(static_body_count, dynamic_body_count)
        {
            return Err(crate::error::ApiError::InvalidArgument);
        }

        Ok(Self {
            static_shape_count: Self::to_native(static_shape_count)?,
            dynamic_shape_count: Self::to_native(dynamic_shape_count)?,
            static_body_count: Self::to_native(static_body_count)?,
            dynamic_body_count: Self::to_native(dynamic_body_count)?,
            contact_count: Self::to_native(contact_count)?,
        })
    }

    #[inline]
    fn to_native(value: u64) -> crate::error::ApiResult<i32> {
        if value <= Self::MAX_NATIVE {
            Ok(value as i32)
        } else {
            Err(crate::error::ApiError::InvalidArgument)
        }
    }

    #[inline]
    fn sum_fits_native(left: u64, right: u64) -> bool {
        left.checked_add(right)
            .is_some_and(|sum| sum <= Self::MAX_NATIVE)
    }

    /// Parse a capacity returned by Box2D, rejecting an invalid negative field.
    pub fn try_from_raw(raw: ffi::b2Capacity) -> crate::error::ApiResult<Self> {
        Self::new(
            u64::try_from(raw.staticShapeCount)
                .map_err(|_| crate::error::ApiError::InvalidArgument)?,
            u64::try_from(raw.dynamicShapeCount)
                .map_err(|_| crate::error::ApiError::InvalidArgument)?,
            u64::try_from(raw.staticBodyCount)
                .map_err(|_| crate::error::ApiError::InvalidArgument)?,
            u64::try_from(raw.dynamicBodyCount)
                .map_err(|_| crate::error::ApiError::InvalidArgument)?,
            u64::try_from(raw.contactCount).map_err(|_| crate::error::ApiError::InvalidArgument)?,
        )
    }

    #[inline]
    pub const fn static_shape_count(self) -> u32 {
        self.static_shape_count as u32
    }

    #[inline]
    pub const fn dynamic_shape_count(self) -> u32 {
        self.dynamic_shape_count as u32
    }

    #[inline]
    pub const fn static_body_count(self) -> u32 {
        self.static_body_count as u32
    }

    #[inline]
    pub const fn dynamic_body_count(self) -> u32 {
        self.dynamic_body_count as u32
    }

    #[inline]
    pub const fn contact_count(self) -> u32 {
        self.contact_count as u32
    }

    #[inline]
    pub(crate) const fn into_raw(self) -> ffi::b2Capacity {
        ffi::b2Capacity {
            staticShapeCount: self.static_shape_count,
            dynamicShapeCount: self.dynamic_shape_count,
            staticBodyCount: self.static_body_count,
            dynamicBodyCount: self.dynamic_body_count,
            contactCount: self.contact_count,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct OwnedHandleCounts {
    pub bodies: usize,
    pub shapes: usize,
    pub joints: usize,
    pub chains: usize,
}

/// Simulation counters providing size and internal stats.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct Counters {
    /// Bytes currently allocated by Box2D process-wide when these counters were sampled.
    pub byte_count: i64,
    pub body_count: i32,
    pub shape_count: i32,
    pub contact_count: i32,
    pub joint_count: i32,
    pub island_count: i32,
    pub stack_used: i32,
    pub static_tree_height: i32,
    pub tree_height: i32,
    pub task_count: i32,
    pub color_counts: [i32; 24],
    /// Contacts visited by the most recent collide pass.
    pub awake_contact_count: i32,
    /// Contacts recycled during the most recent step.
    pub recycled_contact_count: i32,
}

impl Counters {
    #[inline]
    pub fn from_raw(raw: ffi::b2Counters) -> Self {
        Self {
            byte_count: raw.byteCount,
            body_count: raw.bodyCount,
            shape_count: raw.shapeCount,
            contact_count: raw.contactCount,
            joint_count: raw.jointCount,
            island_count: raw.islandCount,
            stack_used: raw.stackUsed,
            static_tree_height: raw.staticTreeHeight,
            tree_height: raw.treeHeight,
            task_count: raw.taskCount,
            color_counts: raw.colorCounts,
            awake_contact_count: raw.awakeContactCount,
            recycled_contact_count: raw.recycledContactCount,
        }
    }
}

/// Simulation profile timings in milliseconds for the last completed world step.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Profile {
    pub step: f32,
    pub pairs: f32,
    pub collide: f32,
    pub solve: f32,
    pub solver_setup: f32,
    pub constraints: f32,
    pub prepare_constraints: f32,
    pub integrate_velocities: f32,
    pub warm_start: f32,
    pub solve_impulses: f32,
    pub integrate_positions: f32,
    pub relax_impulses: f32,
    pub apply_restitution: f32,
    pub store_impulses: f32,
    pub split_islands: f32,
    pub transforms: f32,
    pub sensor_hits: f32,
    pub joint_events: f32,
    pub hit_events: f32,
    pub refit: f32,
    pub bullets: f32,
    pub sleep_islands: f32,
    pub sensors: f32,
}

impl Profile {
    #[inline]
    pub fn from_raw(raw: ffi::b2Profile) -> Self {
        Self {
            step: raw.step,
            pairs: raw.pairs,
            collide: raw.collide,
            solve: raw.solve,
            solver_setup: raw.solverSetup,
            constraints: raw.constraints,
            prepare_constraints: raw.prepareConstraints,
            integrate_velocities: raw.integrateVelocities,
            warm_start: raw.warmStart,
            solve_impulses: raw.solveImpulses,
            integrate_positions: raw.integratePositions,
            relax_impulses: raw.relaxImpulses,
            apply_restitution: raw.applyRestitution,
            store_impulses: raw.storeImpulses,
            split_islands: raw.splitIslands,
            transforms: raw.transforms,
            sensor_hits: raw.sensorHits,
            joint_events: raw.jointEvents,
            hit_events: raw.hitEvents,
            refit: raw.refit,
            bullets: raw.bullets,
            sleep_islands: raw.sleepIslands,
            sensors: raw.sensors,
        }
    }

    #[inline]
    pub fn into_raw(self) -> ffi::b2Profile {
        ffi::b2Profile {
            step: self.step,
            pairs: self.pairs,
            collide: self.collide,
            solve: self.solve,
            solverSetup: self.solver_setup,
            constraints: self.constraints,
            prepareConstraints: self.prepare_constraints,
            integrateVelocities: self.integrate_velocities,
            warmStart: self.warm_start,
            solveImpulses: self.solve_impulses,
            integratePositions: self.integrate_positions,
            relaxImpulses: self.relax_impulses,
            applyRestitution: self.apply_restitution,
            storeImpulses: self.store_impulses,
            splitIslands: self.split_islands,
            transforms: self.transforms,
            sensorHits: self.sensor_hits,
            jointEvents: self.joint_events,
            hitEvents: self.hit_events,
            refit: self.refit,
            bullets: self.bullets,
            sleepIslands: self.sleep_islands,
            sensors: self.sensors,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_count_and_capacity_reject_native_overflow() {
        assert!(!WorkerCount::supports(0));
        assert!(!WorkerCount::supports(B2_MAX_WORKERS + 1));
        #[cfg(target_family = "wasm")]
        assert!(!WorkerCount::supports(2));
        #[cfg(not(target_family = "wasm"))]
        assert!(WorkerCount::supports(2));
        assert_eq!(WorkerCount::new(0), Err(crate::ApiError::InvalidArgument));
        assert_eq!(
            WorkerCount::new(u32::from(B2_MAX_WORKERS) + 1),
            Err(crate::ApiError::InvalidArgument)
        );
        assert_eq!(
            WorldCapacity::new(u64::from(i32::MAX as u32) + 1, 0, 0, 0, 0),
            Err(crate::ApiError::InvalidArgument)
        );
        assert_eq!(
            WorldCapacity::new(0, 0, i32::MAX as u64, 1, 0),
            Err(crate::ApiError::InvalidArgument)
        );
        assert_eq!(
            WorldCapacity::new(WorldCapacity::MAX_SHAPE_COUNT + 1, 0, 0, 0, 0),
            Err(crate::ApiError::InvalidArgument)
        );
        assert_eq!(
            WorldCapacity::new(0, 0, 0, 0, WorldCapacity::MAX_CONTACT_COUNT + 1),
            Err(crate::ApiError::InvalidArgument)
        );
        assert!(
            WorldCapacity::new(
                WorldCapacity::MAX_SHAPE_COUNT,
                0,
                0,
                0,
                WorldCapacity::MAX_CONTACT_COUNT,
            )
            .is_ok()
        );
        assert_eq!(
            WorldCapacity::try_from_raw(ffi::b2Capacity {
                staticShapeCount: -1,
                dynamicShapeCount: 0,
                staticBodyCount: 0,
                dynamicBodyCount: 0,
                contactCount: 0,
            }),
            Err(crate::ApiError::InvalidArgument)
        );
    }

    #[test]
    fn capacity_round_trips_all_fields_without_signed_aliasing() {
        let capacity = WorldCapacity::new(1, 2, 3, 4, 5).unwrap();
        let raw = capacity.into_raw();
        assert_eq!(WorldCapacity::try_from_raw(raw), Ok(capacity));
        assert_eq!(capacity.static_shape_count(), 1);
        assert_eq!(capacity.dynamic_shape_count(), 2);
        assert_eq!(capacity.static_body_count(), 3);
        assert_eq!(capacity.dynamic_body_count(), 4);
        assert_eq!(capacity.contact_count(), 5);
    }

    #[test]
    fn counters_preserve_all_box2d_3_2_fields() {
        let mut color_counts = [0; 24];
        for (index, count) in color_counts.iter_mut().enumerate() {
            *count = i32::try_from(index).unwrap() + 100;
        }

        let raw = ffi::b2Counters {
            byteCount: i64::from(i32::MAX) + 123,
            bodyCount: 1,
            shapeCount: 2,
            contactCount: 3,
            jointCount: 4,
            islandCount: 5,
            stackUsed: 6,
            staticTreeHeight: 7,
            treeHeight: 8,
            taskCount: 9,
            colorCounts: color_counts,
            awakeContactCount: 10,
            recycledContactCount: 11,
        };

        let counters = Counters::from_raw(raw);
        assert_eq!(counters.byte_count, i64::from(i32::MAX) + 123);
        assert_eq!(counters.body_count, 1);
        assert_eq!(counters.shape_count, 2);
        assert_eq!(counters.contact_count, 3);
        assert_eq!(counters.joint_count, 4);
        assert_eq!(counters.island_count, 5);
        assert_eq!(counters.stack_used, 6);
        assert_eq!(counters.static_tree_height, 7);
        assert_eq!(counters.tree_height, 8);
        assert_eq!(counters.task_count, 9);
        assert_eq!(counters.color_counts, color_counts);
        assert_eq!(counters.awake_contact_count, 10);
        assert_eq!(counters.recycled_contact_count, 11);
    }

    #[test]
    fn profile_round_trip_uses_current_native_field_names() {
        let profile = Profile {
            step: 1.0,
            pairs: 2.0,
            collide: 3.0,
            solve: 4.0,
            solver_setup: 5.0,
            constraints: 6.0,
            prepare_constraints: 7.0,
            integrate_velocities: 8.0,
            warm_start: 9.0,
            solve_impulses: 10.0,
            integrate_positions: 11.0,
            relax_impulses: 12.0,
            apply_restitution: 13.0,
            store_impulses: 14.0,
            split_islands: 15.0,
            transforms: 16.0,
            sensor_hits: 17.0,
            joint_events: 18.0,
            hit_events: 19.0,
            refit: 20.0,
            bullets: 21.0,
            sleep_islands: 22.0,
            sensors: 23.0,
        };

        assert_eq!(Profile::from_raw(profile.into_raw()), profile);
    }
}
