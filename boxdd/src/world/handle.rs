use super::*;

mod body_reads;
mod shape_reads;
mod world_reads;

/// A cheap, cloneable handle to a world's shared Rust state.
///
/// Unlike `&World`, this does not borrow the world, which makes it convenient to store inside other
/// objects (e.g. debug draw implementations). It is still `!Send`/`!Sync` to match Box2D's thread
/// safety guarantees.
/// Dropping the owning [`World`] ends the native lifetime; fallible handle methods then report
/// [`crate::ApiError::WorldDestroyed`].
///
/// `WorldHandle` intentionally focuses on stored read-only world/body/shape/joint queries and
/// diagnostics plus owned event snapshots. Borrowed/raw step-local event buffer views remain on
/// [`World`] because they are tied to Box2D's completed-step event buffers plus deferred-destroy
/// flushing behavior.
#[derive(Clone)]
pub struct WorldHandle {
    core: Rc<WorldCore>,
    events: Rc<crate::events::EventCache>,
}
