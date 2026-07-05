//! Bevy ECS components used to author and observe Box2D physics objects.

use bevy_ecs::prelude::Component;
use bevy_math::Vec2 as BevyVec2;
use boxdd::{
    ApiError, ApiResult, BodyId, BodyType, Filter, MotionLocks, ShapeDef, ShapeId, SurfaceMaterial,
};

/// Maximum number of vertices accepted by [`Collider::ConvexPolygon`].
pub const MAX_COLLIDER_POLYGON_VERTICES: usize = boxdd::MAX_POLYGON_VERTICES;

/// Body type to create for an entity that participates in the physics world.
#[derive(Component, Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum RigidBody {
    /// Immovable body used for terrain, walls, and other non-simulated geometry.
    Static,
    /// App-controlled body that can move but is not affected by gravity.
    Kinematic,
    /// Fully simulated body affected by gravity, contacts, joints, and forces.
    #[default]
    Dynamic,
}

impl From<RigidBody> for BodyType {
    fn from(value: RigidBody) -> Self {
        match value {
            RigidBody::Static => BodyType::Static,
            RigidBody::Kinematic => BodyType::Kinematic,
            RigidBody::Dynamic => BodyType::Dynamic,
        }
    }
}

/// Runtime body tuning applied to the native Box2D body.
///
/// This component is optional. Values are applied at body creation and reapplied
/// when the component changes.
#[derive(Component, Copy, Clone, Debug, PartialEq)]
pub struct BodySettings {
    /// Gravity multiplier applied to this body.
    pub gravity_scale: f32,
    /// Linear damping applied by Box2D.
    pub linear_damping: f32,
    /// Angular damping applied by Box2D.
    pub angular_damping: f32,
    /// Whether the body is allowed to sleep.
    pub sleep_enabled: bool,
    /// Whether the body uses bullet-style continuous collision handling.
    pub bullet: bool,
    /// Per-axis translation and rotation locks.
    pub motion_locks: MotionLocks,
}

impl BodySettings {
    /// Creates settings that mark a body as a bullet for continuous collision.
    pub fn bullet() -> Self {
        Self {
            bullet: true,
            ..Default::default()
        }
    }

    /// Validates finite tuning values before applying them.
    pub fn validate(self) -> ApiResult<()> {
        validate_scalar(self.gravity_scale)?;
        validate_nonnegative_scalar(self.linear_damping)?;
        validate_nonnegative_scalar(self.angular_damping)
    }
}

impl Default for BodySettings {
    fn default() -> Self {
        Self {
            gravity_scale: 1.0,
            linear_damping: 0.0,
            angular_damping: 0.0,
            sleep_enabled: true,
            bullet: false,
            motion_locks: MotionLocks::default(),
        }
    }
}

/// Shape descriptor used to create a Box2D shape for a body entity.
///
/// A collider may live on the same entity as [`RigidBody`] or on a child entity.
/// Child collider transforms are interpreted as local offsets from the parent
/// body entity.
#[derive(Component, Copy, Clone, Debug, PartialEq)]
pub enum Collider {
    /// Circle collider.
    Circle {
        /// Circle radius in local Bevy units.
        radius: f32,
        /// Local-space center relative to the body.
        center: BevyVec2,
    },
    /// Capsule collider between two local-space endpoints.
    Capsule {
        /// First endpoint of the capsule segment in local space.
        point1: BevyVec2,
        /// Second endpoint of the capsule segment in local space.
        point2: BevyVec2,
        /// Capsule radius around the segment.
        radius: f32,
    },
    /// Line segment collider. This is normally used on static bodies.
    Segment {
        /// First endpoint in local space.
        point1: BevyVec2,
        /// Second endpoint in local space.
        point2: BevyVec2,
    },
    /// Axis-aligned box collider, optionally transformed by the collider entity.
    Rectangle {
        /// Positive half extents in local space.
        half_extents: BevyVec2,
    },
    /// Rounded box collider.
    RoundedRectangle {
        /// Positive half extents in local space.
        half_extents: BevyVec2,
        /// Non-negative skin radius.
        radius: f32,
    },
    /// Convex polygon built from up to [`MAX_COLLIDER_POLYGON_VERTICES`] points.
    ConvexPolygon {
        /// Local vertices. Only the first `count` entries are used.
        vertices: [BevyVec2; MAX_COLLIDER_POLYGON_VERTICES],
        /// Number of active vertices.
        count: u8,
        /// Non-negative skin radius.
        radius: f32,
    },
}

impl Collider {
    /// Creates a circle collider centered on the body origin.
    pub const fn circle(radius: f32) -> Self {
        Self::Circle {
            radius,
            center: BevyVec2::ZERO,
        }
    }

    /// Creates a circle collider with a local-space center.
    pub const fn circle_at(center: BevyVec2, radius: f32) -> Self {
        Self::Circle { radius, center }
    }

    /// Creates a rectangle collider from half extents.
    pub const fn rectangle(half_width: f32, half_height: f32) -> Self {
        Self::Rectangle {
            half_extents: BevyVec2::new(half_width, half_height),
        }
    }

    /// Creates a square collider from one half extent.
    pub const fn square(half_extent: f32) -> Self {
        Self::Rectangle {
            half_extents: BevyVec2::splat(half_extent),
        }
    }

    /// Creates a rounded rectangle collider from half extents and skin radius.
    pub const fn rounded_rectangle(half_width: f32, half_height: f32, radius: f32) -> Self {
        Self::RoundedRectangle {
            half_extents: BevyVec2::new(half_width, half_height),
            radius,
        }
    }

    /// Creates a capsule oriented along the local Y axis.
    pub const fn capsule_y(half_height: f32, radius: f32) -> Self {
        Self::Capsule {
            point1: BevyVec2::new(0.0, -half_height),
            point2: BevyVec2::new(0.0, half_height),
            radius,
        }
    }

    /// Creates a segment collider from two local endpoints.
    pub const fn segment(point1: BevyVec2, point2: BevyVec2) -> Self {
        Self::Segment { point1, point2 }
    }

    /// Creates a convex polygon collider from local-space vertices.
    pub fn convex_polygon<I>(points: I, radius: f32) -> ApiResult<Self>
    where
        I: IntoIterator<Item = BevyVec2>,
    {
        let mut vertices = [BevyVec2::ZERO; MAX_COLLIDER_POLYGON_VERTICES];
        let mut count = 0usize;
        for point in points {
            if count == MAX_COLLIDER_POLYGON_VERTICES {
                return Err(ApiError::InvalidArgument);
            }
            vertices[count] = point;
            count += 1;
        }

        if count == 0 || count > u8::MAX as usize {
            return Err(ApiError::InvalidArgument);
        }

        let collider = Self::ConvexPolygon {
            vertices,
            count: count as u8,
            radius,
        };
        collider.validate()?;
        Ok(collider)
    }

    /// Validates finite, positive collider parameters before native shape creation.
    pub fn validate(self) -> ApiResult<()> {
        match self {
            Self::Circle { radius, center } => {
                validate_vec2(center)?;
                validate_positive_scalar(radius)
            }
            Self::Capsule {
                point1,
                point2,
                radius,
            } => {
                validate_vec2(point1)?;
                validate_vec2(point2)?;
                validate_distinct_points(point1, point2)?;
                validate_positive_scalar(radius)
            }
            Self::Segment { point1, point2 } => {
                validate_vec2(point1)?;
                validate_vec2(point2)?;
                validate_distinct_points(point1, point2)
            }
            Self::Rectangle { half_extents } => validate_positive_vec2(half_extents),
            Self::RoundedRectangle {
                half_extents,
                radius,
            } => {
                validate_positive_vec2(half_extents)?;
                validate_nonnegative_scalar(radius)
            }
            Self::ConvexPolygon {
                vertices,
                count,
                radius,
            } => {
                let count = count as usize;
                if count == 0 || count > MAX_COLLIDER_POLYGON_VERTICES {
                    return Err(ApiError::InvalidArgument);
                }
                validate_nonnegative_scalar(radius)?;
                for point in &vertices[..count] {
                    validate_vec2(*point)?;
                }
                Ok(())
            }
        }
    }
}

/// Shape material and event flags used when creating a collider shape.
#[derive(Component, Copy, Clone, Debug, PartialEq)]
pub struct PhysicsMaterial {
    /// Shape density passed to Box2D.
    pub density: f32,
    /// Shape friction passed to Box2D.
    pub friction: f32,
    /// Shape restitution passed to Box2D.
    pub restitution: f32,
    /// Shape rolling resistance passed to Box2D.
    pub rolling_resistance: f32,
    /// Shape tangent speed passed to Box2D.
    pub tangent_speed: f32,
    /// User material id passed through Box2D events and callbacks.
    pub user_material_id: u64,
    /// Whether the shape is a sensor.
    pub is_sensor: bool,
    /// Enables contact begin/end messages for this shape.
    pub enable_contact_events: bool,
    /// Enables sensor begin/end messages for this shape.
    pub enable_sensor_events: bool,
    /// Enables contact hit messages for this shape.
    pub enable_hit_events: bool,
    /// Enables pre-solve events for advanced users.
    pub enable_pre_solve_events: bool,
    /// Box2D collision filter data.
    pub filter: Filter,
}

impl PhysicsMaterial {
    /// Converts the component into the `boxdd` shape definition used by the plugin.
    pub fn shape_def(self) -> ShapeDef {
        let material = SurfaceMaterial::default()
            .with_friction(self.friction)
            .with_restitution(self.restitution)
            .with_rolling_resistance(self.rolling_resistance)
            .with_tangent_speed(self.tangent_speed)
            .with_user_material_id(self.user_material_id);

        ShapeDef::builder()
            .material(material)
            .density(self.density)
            .sensor(self.is_sensor)
            .enable_contact_events(self.enable_contact_events)
            .enable_sensor_events(self.enable_sensor_events)
            .enable_hit_events(self.enable_hit_events)
            .enable_pre_solve_events(self.enable_pre_solve_events)
            .filter(self.filter)
            .build()
    }

    /// Validates finite material fields before shape creation.
    pub fn validate(self) -> ApiResult<()> {
        validate_nonnegative_scalar(self.density)?;
        self.shape_def().validate()
    }
}

impl Default for PhysicsMaterial {
    fn default() -> Self {
        Self {
            density: 1.0,
            friction: 0.6,
            restitution: 0.0,
            rolling_resistance: 0.0,
            tangent_speed: 0.0,
            user_material_id: 0,
            is_sensor: false,
            enable_contact_events: false,
            enable_sensor_events: false,
            enable_hit_events: false,
            enable_pre_solve_events: false,
            filter: Filter::default(),
        }
    }
}

/// Native Box2D body id inserted after the plugin creates a body.
#[derive(Component, Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct BoxddBody(pub BodyId);

impl BoxddBody {
    /// Returns the native Box2D body id.
    pub const fn id(self) -> BodyId {
        self.0
    }
}

/// Native Box2D shape id inserted after the plugin creates a shape.
#[derive(Component, Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct BoxddShape(pub ShapeId);

impl BoxddShape {
    /// Returns the native Box2D shape id.
    pub const fn id(self) -> ShapeId {
        self.0
    }
}

/// Direction used when synchronizing Bevy and Box2D transforms.
#[derive(Component, Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum TransformSyncMode {
    /// Read the Box2D transform after stepping and write it into Bevy.
    #[default]
    PhysicsToBevy,
    /// Read the Bevy transform before stepping and write it into Box2D.
    BevyToPhysics,
    /// Disable automatic transform synchronization for this entity.
    None,
}

/// Linear velocity command applied to a body before each physics step.
#[derive(Component, Copy, Clone, Debug, Default, PartialEq)]
pub struct LinearVelocity(pub BevyVec2);

/// Angular velocity command applied to a body before each physics step.
#[derive(Component, Copy, Clone, Debug, Default, PartialEq)]
pub struct AngularVelocity(pub f32);

/// One-shot linear impulse applied at the body center before the next physics step.
#[derive(Component, Copy, Clone, Debug, PartialEq)]
pub struct LinearImpulse {
    /// Impulse vector in physics units.
    pub impulse: BevyVec2,
    /// Whether applying the impulse should wake a sleeping body.
    pub wake: bool,
}

impl LinearImpulse {
    /// Creates a one-shot impulse that wakes the body.
    pub const fn new(impulse: BevyVec2) -> Self {
        Self {
            impulse,
            wake: true,
        }
    }
}

/// One-shot angular impulse applied before the next physics step.
#[derive(Component, Copy, Clone, Debug, PartialEq)]
pub struct AngularImpulse {
    /// Angular impulse in physics units.
    pub impulse: f32,
    /// Whether applying the impulse should wake a sleeping body.
    pub wake: bool,
}

impl AngularImpulse {
    /// Creates a one-shot angular impulse that wakes the body.
    pub const fn new(impulse: f32) -> Self {
        Self {
            impulse,
            wake: true,
        }
    }
}

fn validate_vec2(value: BevyVec2) -> ApiResult<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(ApiError::InvalidArgument)
    }
}

fn validate_scalar(value: f32) -> ApiResult<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(ApiError::InvalidArgument)
    }
}

fn validate_positive_scalar(value: f32) -> ApiResult<()> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(ApiError::InvalidArgument)
    }
}

fn validate_nonnegative_scalar(value: f32) -> ApiResult<()> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(ApiError::InvalidArgument)
    }
}

fn validate_positive_vec2(value: BevyVec2) -> ApiResult<()> {
    if value.is_finite() && value.x > 0.0 && value.y > 0.0 {
        Ok(())
    } else {
        Err(ApiError::InvalidArgument)
    }
}

fn validate_distinct_points(a: BevyVec2, b: BevyVec2) -> ApiResult<()> {
    if a.distance_squared(b) > 0.0 {
        Ok(())
    } else {
        Err(ApiError::InvalidArgument)
    }
}
