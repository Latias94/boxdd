use super::*;

impl Circle {
    #[inline]
    pub fn new<C: Into<Vec2>>(center: C, radius: f32) -> Result<Self> {
        let circle = Self {
            center: center.into(),
            radius,
        };
        check_circle_helper_geometry_valid("Circle::new", circle)?;
        Ok(circle)
    }

    #[inline]
    /// Construct from a raw Box2D geometry value after validating its invariants.
    pub fn from_raw(raw: ffi::b2Circle) -> Result<Self> {
        let circle = Self {
            center: Vec2::from_raw(raw.center),
            radius: raw.radius,
        };
        check_circle_helper_geometry_valid("Circle::from_raw", circle)?;
        Ok(circle)
    }

    #[inline]
    pub const fn center(self) -> Vec2 {
        self.center
    }

    #[inline]
    pub const fn radius(self) -> f32 {
        self.radius
    }

    #[inline]
    /// Convert into the raw Box2D geometry value.
    pub fn into_raw(self) -> ffi::b2Circle {
        ffi::b2Circle {
            center: self.center.into_raw(),
            radius: self.radius,
        }
    }

    #[inline]
    /// Validate this circle for safe Box2D shape and standalone collision use.
    pub fn is_valid(self) -> bool {
        circle_helper_geometry_is_valid(self)
    }

    #[inline]
    /// Validate this circle for safe Box2D shape and standalone collision use.
    pub fn validate(self) -> Result<()> {
        check_circle_helper_geometry_valid("Circle::validate", self)
    }

    #[inline]
    pub fn mass_data(self, density: f32) -> Result<MassData> {
        check_circle_helper_geometry_valid("Circle::mass_data", self)?;
        check_non_negative_finite_density("Circle::mass_data", density)?;
        let raw = self.into_raw();
        let _lease = transient_native_lease()?;
        MassData::from_native("Circle::mass_data", unsafe {
            ffi::b2ComputeCircleMass(&raw, density)
        })
    }

    /// Compute an absolute world-space AABB using `transform` as the circle's
    /// local-to-world transform.
    ///
    /// The result uses `f32` coordinates in both precision modes. Double-precision world bounds
    /// are narrowed outward by Box2D so the returned AABB remains conservative.
    #[inline]
    pub fn aabb(self, transform: WorldTransform) -> Result<Aabb> {
        check_circle_helper_geometry_valid("Circle::aabb", self)?;
        check_world_transform_valid("Circle::aabb", transform)?;
        let raw = self.into_raw();
        let _lease = transient_native_lease()?;
        check_native_geometry_aabb("Circle::aabb", unsafe {
            ffi::b2ComputeCircleAABB(&raw, transform.into_raw())
        })
    }

    #[inline]
    pub fn contains_point<P: Into<Vec2>>(self, point: P) -> Result<bool> {
        let point = point.into();
        check_circle_helper_geometry_valid("Circle::contains_point", self)?;
        check_valid_geometry_vec2("Circle::contains_point", "point", point)?;
        let raw = self.into_raw();
        let _lease = transient_native_lease()?;
        Ok(unsafe { ffi::b2PointInCircle(&raw, point.into_raw()) })
    }

    #[inline]
    pub fn ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        self,
        origin: VO,
        translation: VT,
    ) -> Result<CastOutput> {
        let input = materialize_ray_input(origin, translation);
        check_circle_helper_geometry_valid("Circle::ray_cast", self)?;
        check_ray_input_valid("Circle::ray_cast", &input)?;
        let raw = self.into_raw();
        let _lease = transient_native_lease()?;
        CastOutput::from_native("Circle::ray_cast", unsafe {
            ffi::b2RayCastCircle(&raw, &input)
        })
    }

    #[inline]
    pub fn shape_cast(self, input: ShapeCastInput) -> Result<CastOutput> {
        check_circle_helper_geometry_valid("Circle::shape_cast", self)?;
        input.validate()?;
        let raw = self.into_raw();
        let input = input.into_raw();
        let _lease = transient_native_lease()?;
        CastOutput::from_native("Circle::shape_cast", unsafe {
            ffi::b2ShapeCastCircle(&raw, &input)
        })
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Circle {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Repr {
            center: Vec2,
            radius: f32,
        }

        let repr = <Repr as serde::Deserialize>::deserialize(deserializer)?;
        Self::new(repr.center, repr.radius).map_err(serde::de::Error::custom)
    }
}
