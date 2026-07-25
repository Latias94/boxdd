use boxdd::prelude::*;
use boxdd::shapes;
use std::collections::HashSet;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

struct TaskSystemState {
    task_panicked: AtomicBool,
}

struct MultithreadedWorld {
    // Rust drops fields in declaration order, so the raw task context always outlives the world.
    world: World,
    task_state: Option<Box<TaskSystemState>>,
}

#[derive(Copy, Clone)]
enum TaskSystem {
    BuiltIn,
    External,
}

/// A test-only task system that gives each native task a real Rust OS thread.
///
/// The production safe API intentionally does not expose an executor. This raw callback is used
/// here to force Box2D's worker callback path to run concurrently and to verify the panic boundary.
unsafe extern "C" fn threaded_enqueue(
    task: boxdd_sys::ffi::b2TaskCallback,
    task_context: *mut c_void,
    _user_context: *mut c_void,
) -> *mut c_void {
    let Some(task) = task else {
        return core::ptr::null_mut();
    };
    let task_context = task_context as usize;
    let handle = thread::spawn(move || {
        // SAFETY: Box2D supplies the task callback and context, and invokes each task exactly once.
        unsafe { task(task_context as *mut c_void) };
    });
    Box::into_raw(Box::new(handle)) as *mut c_void
}

unsafe extern "C" fn threaded_finish(user_task: *mut c_void, user_context: *mut c_void) {
    if user_task.is_null() {
        return;
    }
    // SAFETY: `threaded_enqueue` returns exactly this allocation for every non-null task handle,
    // and Box2D calls finish exactly once before returning from the native operation.
    let handle = unsafe { Box::from_raw(user_task as *mut JoinHandle<()>) };
    if let Err(payload) = handle.join() {
        // SAFETY: the boxed state outlives the world and therefore every finish callback.
        let state = unsafe { &*(user_context as *const TaskSystemState) };
        state.task_panicked.store(true, Ordering::SeqCst);
        // Never drop an arbitrary panic payload on a C -> Rust callback stack.
        std::mem::forget(payload);
    }
}

fn multithreaded_contact_world(task_system: TaskSystem) -> MultithreadedWorld {
    let mut definition = WorldDef::builder()
        .gravity([0.0_f32, 0.0])
        .worker_count(WorkerCount::new(4).unwrap())
        .build();
    let mut task_state = None;
    if matches!(task_system, TaskSystem::External) {
        let mut state = Box::new(TaskSystemState {
            task_panicked: AtomicBool::new(false),
        });
        let state_ptr = (&mut *state) as *mut TaskSystemState as *mut c_void;
        // SAFETY: callbacks are process-static. `task_state` owns a stable boxed allocation that is
        // kept alive until after the world is explicitly dropped by the test.
        unsafe {
            definition.set_task_system_raw(
                4,
                Some(threaded_enqueue),
                Some(threaded_finish),
                state_ptr,
            );
        }
        task_state = Some(state);
    }
    let mut world = World::new(definition).expect("multithreaded world should be created");

    // The collision update pass uses a 64-contact minimum range. 256 isolated overlapping pairs
    // therefore produce at least four native tasks with worker_count=4.
    let static_shape = ShapeDef::builder()
        .density(0.0)
        .enable_pre_solve_events(true)
        .build();
    let dynamic_shape = ShapeDef::builder()
        .density(1.0)
        .enable_pre_solve_events(true)
        .build();
    let polygon = shapes::box_polygon(0.45, 0.45);
    for index in 0..256 {
        let x = index as f32 * 2.0;
        let static_body = world.create_body_id(BodyBuilder::new().position([x, 0.0]).build());
        let dynamic_body = world.create_body_id(
            BodyBuilder::new()
                .body_type(BodyType::Dynamic)
                .position([x, 0.0])
                .build(),
        );
        let _ = world.create_polygon_shape_for(static_body, &static_shape, &polygon);
        let _ = world.create_polygon_shape_for(dynamic_body, &dynamic_shape, &polygon);
    }

    // Establish the contacts before installing the panicking callback. The next step then enters
    // the parallel contact-update path deterministically instead of depending on pair creation.
    world.step(1.0 / 60.0, 1);
    MultithreadedWorld { world, task_state }
}

fn assert_worker_callback_panic_returns_at_owner_boundary(task_system: TaskSystem) {
    let mut harness = multithreaded_contact_world(task_system);
    let entered = Arc::new(AtomicUsize::new(0));
    let callback_threads = Arc::new(Mutex::new(HashSet::new()));
    harness.world.set_pre_solve({
        let entered = Arc::clone(&entered);
        let callback_threads = Arc::clone(&callback_threads);
        move |_, _, _, _| {
            let ordinal = entered.fetch_add(1, Ordering::SeqCst) + 1;
            callback_threads
                .lock()
                .expect("callback thread set mutex")
                .insert(thread::current().id());

            // Let other native tasks cross the C -> Rust boundary before the first callback
            // unwinds. The timeout keeps the test bounded if a platform schedules only one task.
            let deadline = Instant::now() + Duration::from_millis(250);
            while entered.load(Ordering::SeqCst) < 4 && Instant::now() < deadline {
                thread::yield_now();
            }
            panic!("concurrent pre-solve panic #{ordinal}");
        }
    });

    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        harness.world.step(1.0 / 60.0, 1);
    }))
    .expect_err("worker callback panic must resume at the owner boundary");
    let message = panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&str>().copied())
        .unwrap_or_default();
    assert!(
        message.starts_with("concurrent pre-solve panic #"),
        "unexpected panic payload: {message:?}"
    );

    let callback_thread_count = callback_threads
        .lock()
        .expect("callback thread set mutex")
        .len();
    assert!(
        entered.load(Ordering::SeqCst) >= 2,
        "expected competing worker callbacks, got {}",
        entered.load(Ordering::SeqCst)
    );
    assert!(
        callback_thread_count >= 2,
        "expected callbacks on multiple native workers, got {callback_thread_count} thread(s)"
    );

    // The first worker panic is consumed by `step`; no panic state or task handle may poison the
    // world. Clearing the callback and stepping again verifies normal teardown remains possible.
    harness.world.clear_pre_solve();
    harness
        .world
        .try_step(1.0 / 60.0, 1)
        .expect("world should remain usable after worker callback panic");

    if let Some(state) = &harness.task_state {
        assert!(
            !state.task_panicked.load(Ordering::SeqCst),
            "native task panicked outside the contained worker callback boundary"
        );
    }
    drop(harness);
}

#[test]
fn built_in_scheduler_worker_callback_panic_returns_at_owner_boundary() {
    assert_worker_callback_panic_returns_at_owner_boundary(TaskSystem::BuiltIn);
}

#[test]
fn external_scheduler_worker_callback_panic_returns_at_owner_boundary() {
    assert_worker_callback_panic_returns_at_owner_boundary(TaskSystem::External);
}
