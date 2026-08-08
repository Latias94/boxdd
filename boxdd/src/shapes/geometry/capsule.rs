use super::*;

impl Capsule {
    #[inline]
    pub fn new<C1: Into<Vec2>, C2: Into<Vec2>>(
        center1: C1,
        center2: C2,
        radius: f32,
    ) -> Result<Self> {
        let capsule = Self {
            center1: center1.into(),
            center2: center2.into(),
            radius,
        };
        check_capsule_geometry_valid_for_operation("Capsule::new", capsule)?;
        Ok(capsule)
    }

    #[inline]
    /// Construct from a raw Box2D geometry value after validating its invariants.
    pub fn from_raw(raw: ffi::b2Capsule) -> Result<Self> {
        let capsule = Self {
            center1: Vec2::from_raw(raw.center1),
            center2: Vec2::from_raw(raw.center2),
            radius: raw.radius,
        };
        check_capsule_geometry_valid_for_operation("Capsule::from_raw", capsule)?;
        Ok(capsule)
    }

    #[inline]
    pub const fn center1(self) -> Vec2 {
        self.center1
    }

    #[inline]
    pub const fn center2(self) -> Vec2 {
        self.center2
    }

    #[inline]
    pub const fn radius(self) -> f32 {
        self.radius
    }

    #[inline]
    /// Convert into the raw Box2D geometry value.
    pub fn into_raw(self) -> ffi::b2Capsule {
        ffi::b2Capsule {
            center1: self.center1.into_raw(),
            center2: self.center2.into_raw(),
            radius: self.radius,
        }
    }

    #[inline]
    /// Validate this capsule for safe Box2D shape and standalone collision use.
    pub fn is_valid(self) -> bool {
        capsule_geometry_is_valid(self)
    }

    #[inline]
    /// Validate this capsule for safe Box2D shape and standalone collision use.
    pub fn validate(self) -> Result<()> {
        check_capsule_geometry_valid_for_operation("Capsule::validate", self)
    }

    #[inline]
    pub fn mass_data(self, density: f32) -> Result<MassData> {
        check_capsule_helper_geometry_valid("Capsule::mass_data", self)?;
        check_non_negative_finite_density("Capsule::mass_data", density)?;
        let raw = self.into_raw();
        let _lease = transient_native_lease()?;
        MassData::from_native("Capsule::mass_data", unsafe {
            ffi::b2ComputeCapsuleMass(&raw, density)
        })
    }

    /// Compute an absolute world-space AABB using `transform` as the capsule's
    /// local-to-world transform.
    ///
    /// The result uses `f32` coordinates in both precision modes. Double-precision world bounds
    /// are narrowed outward by Box2D so the returned AABB remains conservative.
    #[inline]
    pub fn aabb(self, transform: WorldTransform) -> Result<Aabb> {
        check_capsule_helper_geometry_valid("Capsule::aabb", self)?;
        check_world_transform_valid("Capsule::aabb", transform)?;
        let raw = self.into_raw();
        let _lease = transient_native_lease()?;
        check_native_geometry_aabb("Capsule::aabb", unsafe {
            ffi::b2ComputeCapsuleAABB(&raw, transform.into_raw())
        })
    }

    #[inline]
    pub fn contains_point<P: Into<Vec2>>(self, point: P) -> Result<bool> {
        let point = point.into();
        check_capsule_helper_geometry_valid("Capsule::contains_point", self)?;
        check_valid_geometry_vec2("Capsule::contains_point", "point", point)?;
        let raw = self.into_raw();
        let _lease = transient_native_lease()?;
        Ok(unsafe { ffi::b2PointInCapsule(&raw, point.into_raw()) })
    }

    #[inline]
    pub fn ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        self,
        origin: VO,
        translation: VT,
    ) -> Result<CastOutput> {
        let input = materialize_ray_input(origin, translation);
        check_capsule_helper_geometry_valid("Capsule::ray_cast", self)?;
        check_ray_input_valid("Capsule::ray_cast", &input)?;
        let raw = self.into_raw();
        let _lease = transient_native_lease()?;
        CastOutput::from_native("Capsule::ray_cast", unsafe {
            ffi::b2RayCastCapsule(&raw, &input)
        })
    }

    #[inline]
    pub fn shape_cast(self, input: ShapeCastInput) -> Result<CastOutput> {
        check_capsule_helper_geometry_valid("Capsule::shape_cast", self)?;
        input.validate()?;
        let raw = self.into_raw();
        let input = input.into_raw();
        let _lease = transient_native_lease()?;
        CastOutput::from_native("Capsule::shape_cast", unsafe {
            ffi::b2ShapeCastCapsule(&raw, &input)
        })
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Capsule {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Repr {
            center1: Vec2,
            center2: Vec2,
            radius: f32,
        }

        let repr = <Repr as serde::Deserialize>::deserialize(deserializer)?;
        Self::new(repr.center1, repr.center2, repr.radius).map_err(serde::de::Error::custom)
    }
}
