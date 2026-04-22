//! Shapes API
//!
//! Safe wrappers around Box2D shapes. Shapes are attached to bodies and can be
//! modified at runtime. Use `ShapeDef` plus `Body::create_*_shape` or
//! `OwnedBody::create_*_shape` helpers to create shapes.
use std::marker::PhantomData;

pub mod chain;
mod creation;
mod definition;
pub mod geometry;
pub mod helpers;
mod owned;
mod runtime;
mod scoped;

use crate::body::{Body, OwnedBody};
use crate::collision::CastOutput;
use crate::error::{ApiError, ApiResult};
use crate::filter::Filter;
use crate::query::Aabb;
use crate::types::{BodyId, ChainId, ContactData, MassData, ShapeId, Vec2};
use crate::world::World;
use boxdd_sys::ffi;
use std::os::raw::c_void;
use std::sync::Arc;

pub(crate) use runtime::*;

pub use definition::{ShapeDef, ShapeDefBuilder, SurfaceMaterial};
pub use geometry::{
    Capsule, ChainSegment, Circle, MAX_POLYGON_VERTICES, Polygon, Segment, box_polygon, capsule,
    chain_segment, circle, offset_box_polygon, offset_polygon_from_points,
    offset_rounded_box_polygon, polygon_from_points, polygon_hull_is_valid, rounded_box_polygon,
    segment, square_polygon, try_box_polygon, try_offset_box_polygon,
    try_offset_polygon_from_points, try_offset_rounded_box_polygon, try_polygon_from_points,
    try_rounded_box_polygon, try_square_polygon,
};
pub use owned::OwnedShape;
pub use scoped::Shape;

/// Shape kinds reported by Box2D.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ShapeType {
    Circle,
    Capsule,
    Segment,
    Polygon,
    ChainSegment,
}

impl ShapeType {
    #[inline]
    pub const fn from_raw(raw: ffi::b2ShapeType) -> Option<Self> {
        match raw {
            ffi::b2ShapeType_b2_circleShape => Some(Self::Circle),
            ffi::b2ShapeType_b2_capsuleShape => Some(Self::Capsule),
            ffi::b2ShapeType_b2_segmentShape => Some(Self::Segment),
            ffi::b2ShapeType_b2_polygonShape => Some(Self::Polygon),
            ffi::b2ShapeType_b2_chainSegmentShape => Some(Self::ChainSegment),
            _ => None,
        }
    }

    #[inline]
    pub const fn into_raw(self) -> ffi::b2ShapeType {
        match self {
            Self::Circle => ffi::b2ShapeType_b2_circleShape,
            Self::Capsule => ffi::b2ShapeType_b2_capsuleShape,
            Self::Segment => ffi::b2ShapeType_b2_segmentShape,
            Self::Polygon => ffi::b2ShapeType_b2_polygonShape,
            Self::ChainSegment => ffi::b2ShapeType_b2_chainSegmentShape,
        }
    }
}

impl TryFrom<ffi::b2ShapeType> for ShapeType {
    type Error = ffi::b2ShapeType;

    #[inline]
    fn try_from(value: ffi::b2ShapeType) -> Result<Self, Self::Error> {
        Self::from_raw(value).ok_or(value)
    }
}
