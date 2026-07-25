use super::RecordingSession;
use crate::core::world_core::{WorldAccess, WorldCore};
use crate::shapes::chain::ChainDef;
use crate::{ApiResult, BodyId, ChainId, ShapeId, SurfaceMaterial, Vec2};

const RECORDING: WorldAccess = WorldAccess::Recording;

impl RecordingSession<'_> {
    /// Enable or disable sensor events for a shape and record the mutation.
    pub fn shape_enable_sensor_events(&mut self, shape: ShapeId, flag: bool) {
        self.try_shape_enable_sensor_events(shape, flag)
            .expect("recording session received an invalid ShapeId")
    }

    pub fn try_shape_enable_sensor_events(&mut self, shape: ShapeId, flag: bool) -> ApiResult<()> {
        crate::shapes::try_shape_enable_sensor_events_with_access(
            self.world.core(),
            shape,
            flag,
            RECORDING,
        )
    }

    /// Enable or disable contact events for a shape and record the mutation.
    pub fn shape_enable_contact_events(&mut self, shape: ShapeId, flag: bool) {
        self.try_shape_enable_contact_events(shape, flag)
            .expect("recording session received an invalid ShapeId")
    }

    pub fn try_shape_enable_contact_events(&mut self, shape: ShapeId, flag: bool) -> ApiResult<()> {
        crate::shapes::try_shape_enable_contact_events_with_access(
            self.world.core(),
            shape,
            flag,
            RECORDING,
        )
    }

    /// Enable or disable pre-solve events for a shape and record the mutation.
    pub fn shape_enable_pre_solve_events(&mut self, shape: ShapeId, flag: bool) {
        self.try_shape_enable_pre_solve_events(shape, flag)
            .expect("recording session received an invalid ShapeId")
    }

    pub fn try_shape_enable_pre_solve_events(
        &mut self,
        shape: ShapeId,
        flag: bool,
    ) -> ApiResult<()> {
        crate::shapes::try_shape_enable_pre_solve_events_with_access(
            self.world.core(),
            shape,
            flag,
            RECORDING,
        )
    }

    /// Enable or disable hit events for a shape and record the mutation.
    pub fn shape_enable_hit_events(&mut self, shape: ShapeId, flag: bool) {
        self.try_shape_enable_hit_events(shape, flag)
            .expect("recording session received an invalid ShapeId")
    }

    pub fn try_shape_enable_hit_events(&mut self, shape: ShapeId, flag: bool) -> ApiResult<()> {
        crate::shapes::try_shape_enable_hit_events_with_access(
            self.world.core(),
            shape,
            flag,
            RECORDING,
        )
    }

    /// Apply wind to a shape and record the mutation.
    ///
    /// `wind` must be finite and `drag` must be finite and non-negative. `lift` must be finite;
    /// negative values reverse the perpendicular lift direction.
    pub fn shape_apply_wind<V: Into<Vec2>>(
        &mut self,
        shape: ShapeId,
        wind: V,
        drag: f32,
        lift: f32,
        wake: bool,
    ) {
        self.try_shape_apply_wind(shape, wind, drag, lift, wake)
            .expect("recording session received invalid shape wind parameters")
    }

    /// Fallible form of [`Self::shape_apply_wind`].
    ///
    /// Returns `ApiError::InvalidArgument` when a numeric parameter violates its constraints.
    pub fn try_shape_apply_wind<V: Into<Vec2>>(
        &mut self,
        shape: ShapeId,
        wind: V,
        drag: f32,
        lift: f32,
        wake: bool,
    ) -> ApiResult<()> {
        crate::shapes::try_shape_apply_wind_with_access(
            self.world.core(),
            shape,
            wind,
            drag,
            lift,
            wake,
            RECORDING,
        )
    }

    /// Create a chain attached to a body and record the mutation.
    pub fn create_chain(&mut self, body: BodyId, def: &ChainDef) -> ChainId {
        self.try_create_chain(body, def)
            .expect("recording session could not create a chain")
    }

    pub fn try_create_chain(&mut self, body: BodyId, def: &ChainDef) -> ApiResult<ChainId> {
        crate::shapes::chain::try_create_chain_for_body_with_access(
            self.world.core(),
            body,
            def,
            RECORDING,
        )
    }

    /// Set a visible chain segment's surface material and record the mutation.
    pub fn chain_set_surface_material(
        &mut self,
        chain: ChainId,
        index: i32,
        material: &SurfaceMaterial,
    ) {
        self.try_chain_set_surface_material(chain, index, material)
            .expect("recording session received invalid chain material parameters")
    }

    pub fn try_chain_set_surface_material(
        &mut self,
        chain: ChainId,
        index: i32,
        material: &SurfaceMaterial,
    ) -> ApiResult<()> {
        crate::shapes::chain::try_chain_set_surface_material_with_access(
            self.world.core(),
            chain,
            index,
            material,
            RECORDING,
        )
    }

    /// Destroy a chain and record the mutation.
    pub fn destroy_chain(&mut self, chain: ChainId) {
        self.try_destroy_chain(chain)
            .expect("recording session received an invalid ChainId")
    }

    pub fn try_destroy_chain(&mut self, chain: ChainId) -> ApiResult<()> {
        WorldCore::destroy_chain_now_with_access(self.world.core(), chain, RECORDING)
    }
}
