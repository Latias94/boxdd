use super::*;

mod body_reads;
mod callback_world;
mod shape_reads;
mod world_reads;

/// A cheap, cloneable handle to a world that keeps it alive via the internal reference-counted core.
///
/// Unlike `&World`, this does not borrow the world, which makes it convenient to store inside other
/// objects (e.g. debug draw implementations). It is still `!Send`/`!Sync` to match Box2D's thread
/// safety guarantees.
///
/// `WorldHandle` intentionally focuses on stored read-only world/body/shape/joint queries and
/// diagnostics plus owned event snapshots. Borrowed/raw step-local event buffer views remain on
/// [`World`] because they are tied to Box2D's completed-step event buffers plus deferred-destroy
/// flushing behavior.
#[derive(Clone)]
pub struct WorldHandle {
    core: Arc<WorldCore>,
    _not_send_sync: core::marker::PhantomData<Rc<()>>,
}

/// A lightweight, thread-safe context passed to Box2D callbacks.
///
/// This type intentionally exposes only APIs that do not call into Box2D while the world is locked.
#[derive(Clone)]
pub struct CallbackWorld {
    core: Arc<WorldCore>,
}
