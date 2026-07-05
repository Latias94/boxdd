//! Internal error reporting helpers for plugin systems.

use crate::messages::BoxddErrorMessage;
use crate::resources::{BoxddErrorPolicy, BoxddPhysicsSettings};
use bevy_ecs::message::MessageWriter;

pub(crate) fn report_error(
    settings: &BoxddPhysicsSettings,
    writer: &mut MessageWriter<'_, BoxddErrorMessage>,
    message: BoxddErrorMessage,
) {
    match settings.error_policy {
        BoxddErrorPolicy::MessageOnly => {
            writer.write(message);
        }
        BoxddErrorPolicy::MessageAndLog => {
            log::error!("{message:?}");
            writer.write(message);
        }
        BoxddErrorPolicy::Panic => {
            panic!("{message:?}");
        }
    }
}
