//! Internal error reporting helpers for plugin systems.

use crate::messages::BoxddErrorMessage;
use crate::resources::BoxddErrorPolicy;
use bevy_ecs::message::MessageWriter;

pub(crate) fn report_error(
    policy: &BoxddErrorPolicy,
    writer: &mut MessageWriter<'_, BoxddErrorMessage>,
    message: BoxddErrorMessage,
) {
    match *policy {
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
