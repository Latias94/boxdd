use bevy::prelude::Resource;

pub const MIN_HERTZ: f64 = 10.0;
pub const MAX_HERTZ: f64 = 240.0;
pub const DEFAULT_HERTZ: f64 = 60.0;
pub const MIN_SUB_STEPS: i32 = 1;
pub const MAX_SUB_STEPS: i32 = 16;
pub const DEFAULT_SUB_STEPS: i32 = 4;

#[derive(Resource, Debug)]
pub struct TestbedState {
    pub scene_index: usize,
    pub scene_switching_enabled: bool,
    pub paused: bool,
    pub gravity_enabled: bool,
    pub sleeping_enabled: bool,
    pub warm_starting_enabled: bool,
    pub continuous_enabled: bool,
    pub sub_step_count: i32,
    pub hertz: f64,
    pub draw_overlays: bool,
    pub single_step_pending: bool,
    pub single_step_active: bool,
}

impl Default for TestbedState {
    fn default() -> Self {
        Self {
            scene_index: 0,
            scene_switching_enabled: true,
            paused: false,
            gravity_enabled: true,
            sleeping_enabled: true,
            warm_starting_enabled: true,
            continuous_enabled: true,
            sub_step_count: DEFAULT_SUB_STEPS,
            hertz: DEFAULT_HERTZ,
            draw_overlays: true,
            single_step_pending: false,
            single_step_active: false,
        }
    }
}

impl TestbedState {
    pub fn launch(scene_index: usize, scene_switching_enabled: bool) -> Self {
        Self {
            scene_index,
            scene_switching_enabled,
            ..Self::default()
        }
    }

    pub fn clamp_controls(&mut self) {
        self.sub_step_count = self.sub_step_count.clamp(MIN_SUB_STEPS, MAX_SUB_STEPS);
        self.hertz = self.hertz.clamp(MIN_HERTZ, MAX_HERTZ);
    }

    pub fn fixed_timestep_seconds(&self) -> f64 {
        1.0 / self.hertz.clamp(MIN_HERTZ, MAX_HERTZ)
    }

    pub fn request_single_step(&mut self) {
        if self.paused {
            self.single_step_pending = true;
        }
    }

    pub fn cancel_single_step(&mut self) {
        self.single_step_pending = false;
        self.single_step_active = false;
    }
}

#[derive(Resource, Debug, Default)]
pub struct EventStats {
    pub contact_begin_total: u64,
    pub contact_end_total: u64,
    pub contact_hit_total: u64,
    pub sensor_begin_total: u64,
    pub sensor_end_total: u64,
    pub contact_begin_frame: u32,
    pub contact_end_frame: u32,
    pub contact_hit_frame: u32,
    pub sensor_begin_frame: u32,
    pub sensor_end_frame: u32,
}
