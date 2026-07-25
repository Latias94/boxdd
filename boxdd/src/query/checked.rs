use super::QueryTarget;
#[cfg(not(target_arch = "wasm32"))]
use super::raw::*;
use super::types::*;
use crate::error::ApiResult;
#[cfg(not(target_arch = "wasm32"))]
use crate::types::ShapeId;
use crate::types::{Position, Vec2};

mod common;
mod mover_queries;
#[cfg(not(target_arch = "wasm32"))]
mod overlap_queries;
mod ray_queries;
#[cfg(not(target_arch = "wasm32"))]
mod shape_casts;

pub(crate) use self::{common::*, mover_queries::*, ray_queries::*};
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use self::{overlap_queries::*, shape_casts::*};
