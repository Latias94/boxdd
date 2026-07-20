use super::*;

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
    /// Bytes currently allocated by this world.
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
