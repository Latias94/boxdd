mod definition;
mod owned;
mod runtime;
mod scoped;
mod validation;

pub use definition::{BodyBuilder, BodyDef, BodyType};
pub use owned::OwnedBody;
pub use scoped::Body;

/// Maximum UTF-8 byte length accepted by Box2D for a body name.
///
/// The native body stores `B2_NAME_LENGTH + 1` bytes: ten name bytes plus the trailing NUL.
pub const MAX_BODY_NAME_BYTES: usize = 10;

pub(crate) use definition::{
    assert_body_def_valid, assert_mass_data_valid, assert_non_negative_finite_body_scalar,
    check_body_def_valid, check_mass_data_valid, check_non_negative_finite_body_scalar,
};
pub(crate) use runtime::*;
pub(crate) use validation::*;
