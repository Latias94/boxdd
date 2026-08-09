mod definition;
mod runtime;
mod scoped;
mod validation;

pub use definition::{BodyBuilder, BodyDef, BodyType};
pub use scoped::Body;

/// Maximum UTF-8 byte length accepted by Box2D for a body name.
///
/// The native body stores `B2_NAME_LENGTH + 1` bytes: ten name bytes plus the trailing NUL.
pub const MAX_BODY_NAME_BYTES: usize = 10;

pub(crate) use definition::check_body_def_valid;
pub(crate) use runtime::*;
pub(crate) use validation::*;
