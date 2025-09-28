//! Event snapshots and zero-copy visitors.
//!
//! - Snapshot getters like `body_events`, `sensor_events`, `contact_events`, `joint_events`
//!   copy event data into owned Rust collections and are safe to keep after stepping.
//! - Zero-copy visitors like `with_*` variants pass FFI slices valid only for the
//!   duration of the closure and current step.

mod body;
mod contact;
mod joint;
mod sensor;

pub use body::BodyMoveEvent;
pub use contact::{ContactBeginTouchEvent, ContactEndTouchEvent, ContactEvents, ContactHitEvent};
pub use joint::JointEvent;
pub use sensor::{SensorBeginTouchEvent, SensorEndTouchEvent, SensorEvents};
