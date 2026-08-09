use super::*;

mod base;
mod chain_segment;
mod contact_queries;
mod creation;
mod sensor_queries;
mod user_data;
mod validation;

pub(crate) use self::{
    base::*, chain_segment::*, contact_queries::*, creation::*, sensor_queries::*, user_data::*,
    validation::*,
};

impl Shape<'_> {
    #[inline]
    fn shape_id(&self) -> ShapeId {
        self.proof.id()
    }

    #[inline]
    fn shape_access(&self) -> &crate::world::ShapeProof<'_> {
        &self.proof
    }

    pub fn parent_chain_id(&self) -> Result<Option<ChainId>> {
        self.shape_access().call(shape_parent_chain_id_in_impl)
    }

    /// Set an opaque user data pointer on this shape.
    ///
    /// Box2D and `boxdd` store but never dereference this pointer. If typed user data was
    /// previously set via [`Self::set_user_data`], it is cleared and dropped.
    pub fn set_user_data_ptr_raw(&mut self, p: *mut c_void) -> Result<()> {
        self.shape_access()
            .call(|shape| shape_set_user_data_ptr_impl(shape, p))
    }

    pub fn user_data_ptr_raw(&self) -> Result<*mut c_void> {
        let id = self.shape_id();
        self.shape_access()
            .call(|_| Ok(shape_user_data_ptr_impl(id)))
    }

    pub fn set_user_data<T: 'static>(&mut self, value: T) -> Result<()> {
        let value = crate::core::callback_state::PendingUserValue::new(value);
        self.shape_access()
            .call(move |shape| shape_set_user_data_impl(shape, value))
    }

    pub fn clear_user_data(&mut self) -> Result<bool> {
        self.shape_access().call(shape_clear_user_data_impl)
    }

    pub fn with_user_data<T: 'static, R>(&self, f: impl FnOnce(&T) -> R) -> Result<Option<R>> {
        let f = crate::core::callback_state::PendingUserValue::new(f);
        self.shape_access()
            .call(move |shape| shape_with_user_data_impl(shape, f))
    }

    pub fn with_user_data_mut<T: 'static, R>(
        &mut self,
        f: impl FnOnce(&mut T) -> R,
    ) -> Result<Option<R>> {
        let f = crate::core::callback_state::PendingUserValue::new(f);
        self.shape_access()
            .call(move |shape| shape_with_user_data_mut_impl(shape, f))
    }

    pub fn take_user_data<T: 'static>(&mut self) -> Result<Option<T>> {
        self.shape_access().call(shape_take_user_data_impl::<T>)
    }

    pub fn contact_data(&self) -> Result<Vec<ContactData>> {
        self.shape_access().call(shape_contact_data_in_impl)
    }

    pub fn sensor_capacity(&self) -> Result<i32> {
        self.shape_access()
            .call(|_| shape_sensor_capacity_impl("Shape::sensor_capacity", self.shape_id()))
    }

    pub fn sensor_overlaps(&self) -> Result<Vec<ShapeId>> {
        self.shape_access().call(shape_sensor_overlaps_in_impl)
    }

    pub fn is_sensor(&self) -> Result<bool> {
        self.shape_access()
            .call(|_| Ok(shape_is_sensor_impl(self.shape_id())))
    }

    pub fn enable_sensor_events(&mut self, flag: bool) -> Result<()> {
        self.shape_access().call(|_| {
            shape_enable_sensor_events_impl(self.shape_id(), flag);
            Ok(())
        })
    }

    pub fn sensor_events_enabled(&self) -> Result<bool> {
        self.shape_access()
            .call(|_| Ok(shape_sensor_events_enabled_impl(self.shape_id())))
    }

    pub fn enable_contact_events(&mut self, flag: bool) -> Result<()> {
        self.shape_access().call(|_| {
            shape_enable_contact_events_impl(self.shape_id(), flag);
            Ok(())
        })
    }

    pub fn contact_events_enabled(&self) -> Result<bool> {
        self.shape_access()
            .call(|_| Ok(shape_contact_events_enabled_impl(self.shape_id())))
    }

    pub fn enable_pre_solve_events(&mut self, flag: bool) -> Result<()> {
        self.shape_access().call(|_| {
            shape_enable_pre_solve_events_impl(self.shape_id(), flag);
            Ok(())
        })
    }

    pub fn pre_solve_events_enabled(&self) -> Result<bool> {
        self.shape_access()
            .call(|_| Ok(shape_pre_solve_events_enabled_impl(self.shape_id())))
    }

    pub fn enable_hit_events(&mut self, flag: bool) -> Result<()> {
        self.shape_access().call(|_| {
            shape_enable_hit_events_impl(self.shape_id(), flag);
            Ok(())
        })
    }

    pub fn hit_events_enabled(&self) -> Result<bool> {
        self.shape_access()
            .call(|_| Ok(shape_hit_events_enabled_impl(self.shape_id())))
    }

    /// Return the geometry type authenticated when this shape capability was acquired.
    ///
    /// Geometry replacement methods update the authenticated type before returning.
    pub fn shape_type(&self) -> Result<ShapeType> {
        self.shape_access().call(|shape| Ok(shape.kind()))
    }

    pub fn body_id(&self) -> Result<BodyId> {
        self.shape_access().call(shape_body_id_in_impl)
    }

    pub fn circle(&self) -> Result<Circle> {
        self.shape_access().call(|shape| {
            shape.require_kind(ShapeType::Circle)?;
            shape_circle_impl(shape.id())
        })
    }

    pub fn segment(&self) -> Result<Segment> {
        self.shape_access().call(|shape| {
            shape.require_kind(ShapeType::Segment)?;
            shape_segment_impl(shape.id())
        })
    }

    pub fn chain_segment(&self) -> Result<ChainSegment> {
        self.shape_access().call(|shape| {
            shape.require_kind(ShapeType::ChainSegment)?;
            shape_chain_segment_impl(shape.id())
        })
    }

    pub fn capsule(&self) -> Result<Capsule> {
        self.shape_access().call(|shape| {
            shape.require_kind(ShapeType::Capsule)?;
            shape_capsule_impl(shape.id())
        })
    }

    pub fn polygon(&self) -> Result<Polygon> {
        self.shape_access().call(|shape| {
            shape.require_kind(ShapeType::Polygon)?;
            shape_polygon_impl(shape.id())
        })
    }

    pub fn closest_point(&self, target: Position) -> Result<Position> {
        self.shape_access().call(|shape| {
            check_shape_world_point_in_local_range(
                "Shape::closest_point",
                "target",
                shape,
                target,
            )?;
            shape_closest_point_impl(shape.id(), target)
        })
    }

    pub fn aabb(&self) -> Result<Aabb> {
        self.shape_access()
            .call(|_| shape_aabb_impl(self.shape_id()))
    }

    pub fn test_point(&self, point: Position) -> Result<bool> {
        self.shape_access().call(|shape| {
            check_shape_world_point_in_local_range("Shape::test_point", "point", shape, point)?;
            Ok(shape_test_point_impl(shape.id(), point))
        })
    }

    pub fn ray_cast<VT: Into<Vec2>>(
        &self,
        origin: Position,
        translation: VT,
    ) -> Result<WorldCastOutput> {
        let translation = crate::core::callback_state::PendingUserValue::new(translation);
        self.shape_access().call(move |shape| {
            let origin =
                crate::body::check_valid_body_position("Shape::ray_cast", "origin", origin)?;
            let translation = translation.into_inner().into();
            check_shape_vec2_valid("Shape::ray_cast", "translation", translation)?;
            check_shape_world_point_in_local_range("Shape::ray_cast", "origin", shape, origin)?;
            shape_ray_cast_impl(shape.id(), origin, translation)
        })
    }

    /// Fallible form of [`Self::apply_wind`].
    ///
    /// Returns `Error::InvalidArgument` when a numeric parameter violates its constraints.
    pub fn apply_wind<V: Into<Vec2>>(
        &mut self,
        wind: V,
        drag: f32,
        lift: f32,
        wake: bool,
    ) -> Result<()> {
        self.shape_access().call(|_| {
            let wind = wind.into();
            check_shape_wind_parameters_valid(wind, drag, lift)?;
            shape_apply_wind_impl(self.shape_id(), wind, drag, lift, wake);
            Ok(())
        })
    }

    pub fn set_circle(&mut self, circle: &Circle) -> Result<()> {
        let proof = self.shape_access();
        proof.call(|shape| {
            check_circle_geometry_valid(circle)?;
            check_orphan_shape_mutation_target(shape.id())?;
            shape_set_circle_impl(shape.id(), circle);
            proof.set_kind(ShapeType::Circle);
            Ok(())
        })
    }

    pub fn set_segment(&mut self, segment: &Segment) -> Result<()> {
        let proof = self.shape_access();
        proof.call(|shape| {
            check_segment_geometry_valid(segment)?;
            check_orphan_shape_mutation_target(shape.id())?;
            shape_set_segment_impl(shape.id(), segment);
            proof.set_kind(ShapeType::Segment);
            Ok(())
        })
    }

    pub fn set_chain_segment(&mut self, chain_segment: &ChainSegment) -> Result<()> {
        let proof = self.shape_access();
        proof.call(|shape| {
            set_chain_segment_checked(shape.id(), chain_segment)?;
            proof.set_kind(ShapeType::ChainSegment);
            Ok(())
        })
    }

    pub fn set_capsule(&mut self, capsule: &Capsule) -> Result<()> {
        let proof = self.shape_access();
        proof.call(|shape| {
            check_capsule_geometry_valid(capsule)?;
            check_orphan_shape_mutation_target(shape.id())?;
            shape_set_capsule_impl(shape.id(), capsule);
            proof.set_kind(ShapeType::Capsule);
            Ok(())
        })
    }

    pub fn set_polygon(&mut self, polygon: &Polygon) -> Result<()> {
        let proof = self.shape_access();
        proof.call(|shape| {
            check_polygon_geometry_valid(polygon)?;
            check_orphan_shape_mutation_target(shape.id())?;
            shape_set_polygon_impl(shape.id(), polygon);
            proof.set_kind(ShapeType::Polygon);
            Ok(())
        })
    }

    pub fn filter(&self) -> Result<Filter> {
        self.shape_access()
            .call(|_| Ok(shape_filter_impl(self.shape_id())))
    }

    pub fn set_filter(&mut self, filter: Filter) -> Result<()> {
        self.shape_access().call(|_| {
            shape_set_filter_impl(self.shape_id(), filter);
            Ok(())
        })
    }

    pub fn set_density(&mut self, density: f32, update_body_mass: bool) -> Result<()> {
        self.shape_access().call(|_| {
            check_non_negative_finite_shape_scalar("Shape::set_density", "density", density)?;
            shape_set_density_impl(self.shape_id(), density, update_body_mass);
            Ok(())
        })
    }

    pub fn density(&self) -> Result<f32> {
        self.shape_access()
            .call(|_| shape_density_impl(self.shape_id()))
    }

    pub fn mass_data(&self) -> Result<MassData> {
        self.shape_access()
            .call(|_| shape_mass_data_impl(self.shape_id()))
    }

    pub fn set_friction(&mut self, friction: f32) -> Result<()> {
        self.shape_access().call(|_| {
            check_non_negative_finite_shape_scalar("Shape::set_friction", "friction", friction)?;
            shape_set_friction_impl(self.shape_id(), friction);
            Ok(())
        })
    }

    pub fn friction(&self) -> Result<f32> {
        self.shape_access()
            .call(|_| shape_friction_impl(self.shape_id()))
    }

    pub fn set_restitution(&mut self, restitution: f32) -> Result<()> {
        self.shape_access().call(|_| {
            check_non_negative_finite_shape_scalar(
                "Shape::set_restitution",
                "restitution",
                restitution,
            )?;
            shape_set_restitution_impl(self.shape_id(), restitution);
            Ok(())
        })
    }

    pub fn restitution(&self) -> Result<f32> {
        self.shape_access()
            .call(|_| shape_restitution_impl(self.shape_id()))
    }

    pub fn set_user_material(&mut self, material: u64) -> Result<()> {
        self.shape_access().call(|_| {
            shape_set_user_material_impl(self.shape_id(), material);
            Ok(())
        })
    }

    pub fn user_material(&self) -> Result<u64> {
        self.shape_access()
            .call(|_| Ok(shape_user_material_impl(self.shape_id())))
    }

    pub fn set_surface_material(&mut self, material: &SurfaceMaterial) -> Result<()> {
        self.shape_access().call(|_| {
            check_surface_material_valid("Shape::set_surface_material", material)?;
            shape_set_surface_material_impl(self.shape_id(), material);
            Ok(())
        })
    }

    pub fn surface_material(&self) -> Result<SurfaceMaterial> {
        self.shape_access()
            .call(|_| shape_surface_material_impl(self.shape_id()))
    }
}
