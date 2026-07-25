use crate::error::ApiResult;
#[cfg(not(target_arch = "wasm32"))]
use crate::types::ShapeId;
use crate::types::{Position, Vec2};
use crate::world::{World, WorldHandle};

use super::checked::*;
use super::types::*;

mod handle_queries;
mod world_queries;
