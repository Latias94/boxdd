use crate::core::callback_state::MaterialMixCtx;
use crate::world::MaterialMixInput;
use boxdd_sys::ffi;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

pub(crate) const MATERIAL_MIX_SLOT_COUNT: usize = 64;

struct MaterialMixSlot {
    in_use: AtomicBool,
    friction: AtomicPtr<MaterialMixCtx>,
    restitution: AtomicPtr<MaterialMixCtx>,
}

impl MaterialMixSlot {
    const fn new() -> Self {
        Self {
            in_use: AtomicBool::new(false),
            friction: AtomicPtr::new(core::ptr::null_mut()),
            restitution: AtomicPtr::new(core::ptr::null_mut()),
        }
    }
}

static MATERIAL_MIX_SLOTS: [MaterialMixSlot; MATERIAL_MIX_SLOT_COUNT] =
    [const { MaterialMixSlot::new() }; MATERIAL_MIX_SLOT_COUNT];

#[inline]
fn slot_ref(slot: usize) -> &'static MaterialMixSlot {
    &MATERIAL_MIX_SLOTS[slot]
}

pub(crate) fn acquire_slot() -> Option<usize> {
    for (idx, slot) in MATERIAL_MIX_SLOTS.iter().enumerate() {
        if slot
            .in_use
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            return Some(idx);
        }
    }
    None
}

pub(crate) fn release_slot(slot: usize) {
    debug_assert!(
        slot_ref(slot).friction.load(Ordering::Acquire).is_null(),
        "friction callback pointer must be cleared before releasing slot"
    );
    debug_assert!(
        slot_ref(slot).restitution.load(Ordering::Acquire).is_null(),
        "restitution callback pointer must be cleared before releasing slot"
    );
    slot_ref(slot).in_use.store(false, Ordering::Release);
}

#[inline]
pub(crate) fn set_friction_ptr(slot: usize, ptr: *mut MaterialMixCtx) {
    slot_ref(slot).friction.store(ptr, Ordering::Release);
}

#[inline]
pub(crate) fn set_restitution_ptr(slot: usize, ptr: *mut MaterialMixCtx) {
    slot_ref(slot).restitution.store(ptr, Ordering::Release);
}

#[inline]
pub(crate) fn has_any_callback(slot: usize) -> bool {
    let slot = slot_ref(slot);
    !slot.friction.load(Ordering::Acquire).is_null()
        || !slot.restitution.load(Ordering::Acquire).is_null()
}

#[inline]
fn default_friction_mix(friction_a: f32, friction_b: f32) -> f32 {
    let maximum = friction_a.max(friction_b);
    if maximum == 0.0 {
        0.0
    } else {
        maximum * (friction_a.min(friction_b) / maximum).sqrt()
    }
}

#[inline]
fn default_restitution_mix(restitution_a: f32, restitution_b: f32) -> f32 {
    restitution_a.max(restitution_b)
}

unsafe fn invoke_mix_callback(
    ctx_ptr: *mut MaterialMixCtx,
    value_a: f32,
    user_material_id_a: u64,
    value_b: f32,
    user_material_id_b: u64,
    default_mix: fn(f32, f32) -> f32,
) -> f32 {
    if ctx_ptr.is_null() {
        return default_mix(value_a, value_b);
    }

    let ctx = unsafe { &*ctx_ptr };
    let fallback = default_mix(value_a, value_b);
    crate::core::callback_state::invoke_worker_callback(&ctx.worker, fallback, || {
        let mixed = (ctx.cb)(
            MaterialMixInput::new(value_a, user_material_id_a),
            MaterialMixInput::new(value_b, user_material_id_b),
        );
        assert!(
            mixed.is_finite() && mixed >= 0.0,
            "material mixing callback must return a finite non-negative coefficient, got {mixed}"
        );
        mixed
    })
}

#[inline]
unsafe fn invoke_friction_callback(
    slot: usize,
    friction_a: f32,
    user_material_id_a: u64,
    friction_b: f32,
    user_material_id_b: u64,
) -> f32 {
    let ctx_ptr = slot_ref(slot).friction.load(Ordering::Acquire);
    unsafe {
        invoke_mix_callback(
            ctx_ptr,
            friction_a,
            user_material_id_a,
            friction_b,
            user_material_id_b,
            default_friction_mix,
        )
    }
}

#[inline]
unsafe fn invoke_restitution_callback(
    slot: usize,
    restitution_a: f32,
    user_material_id_a: u64,
    restitution_b: f32,
    user_material_id_b: u64,
) -> f32 {
    let ctx_ptr = slot_ref(slot).restitution.load(Ordering::Acquire);
    unsafe {
        invoke_mix_callback(
            ctx_ptr,
            restitution_a,
            user_material_id_a,
            restitution_b,
            user_material_id_b,
            default_restitution_mix,
        )
    }
}

type MaterialMixTrampoline = unsafe extern "C" fn(f32, u64, f32, u64) -> f32;

unsafe extern "C" fn friction_trampoline<const SLOT: usize>(
    friction_a: f32,
    user_material_id_a: u64,
    friction_b: f32,
    user_material_id_b: u64,
) -> f32 {
    unsafe {
        invoke_friction_callback(
            SLOT,
            friction_a,
            user_material_id_a,
            friction_b,
            user_material_id_b,
        )
    }
}

unsafe extern "C" fn restitution_trampoline<const SLOT: usize>(
    restitution_a: f32,
    user_material_id_a: u64,
    restitution_b: f32,
    user_material_id_b: u64,
) -> f32 {
    unsafe {
        invoke_restitution_callback(
            SLOT,
            restitution_a,
            user_material_id_a,
            restitution_b,
            user_material_id_b,
        )
    }
}

static FRICTION_TRAMPOLINES: [MaterialMixTrampoline; MATERIAL_MIX_SLOT_COUNT] = [
    friction_trampoline::<0>,
    friction_trampoline::<1>,
    friction_trampoline::<2>,
    friction_trampoline::<3>,
    friction_trampoline::<4>,
    friction_trampoline::<5>,
    friction_trampoline::<6>,
    friction_trampoline::<7>,
    friction_trampoline::<8>,
    friction_trampoline::<9>,
    friction_trampoline::<10>,
    friction_trampoline::<11>,
    friction_trampoline::<12>,
    friction_trampoline::<13>,
    friction_trampoline::<14>,
    friction_trampoline::<15>,
    friction_trampoline::<16>,
    friction_trampoline::<17>,
    friction_trampoline::<18>,
    friction_trampoline::<19>,
    friction_trampoline::<20>,
    friction_trampoline::<21>,
    friction_trampoline::<22>,
    friction_trampoline::<23>,
    friction_trampoline::<24>,
    friction_trampoline::<25>,
    friction_trampoline::<26>,
    friction_trampoline::<27>,
    friction_trampoline::<28>,
    friction_trampoline::<29>,
    friction_trampoline::<30>,
    friction_trampoline::<31>,
    friction_trampoline::<32>,
    friction_trampoline::<33>,
    friction_trampoline::<34>,
    friction_trampoline::<35>,
    friction_trampoline::<36>,
    friction_trampoline::<37>,
    friction_trampoline::<38>,
    friction_trampoline::<39>,
    friction_trampoline::<40>,
    friction_trampoline::<41>,
    friction_trampoline::<42>,
    friction_trampoline::<43>,
    friction_trampoline::<44>,
    friction_trampoline::<45>,
    friction_trampoline::<46>,
    friction_trampoline::<47>,
    friction_trampoline::<48>,
    friction_trampoline::<49>,
    friction_trampoline::<50>,
    friction_trampoline::<51>,
    friction_trampoline::<52>,
    friction_trampoline::<53>,
    friction_trampoline::<54>,
    friction_trampoline::<55>,
    friction_trampoline::<56>,
    friction_trampoline::<57>,
    friction_trampoline::<58>,
    friction_trampoline::<59>,
    friction_trampoline::<60>,
    friction_trampoline::<61>,
    friction_trampoline::<62>,
    friction_trampoline::<63>,
];

static RESTITUTION_TRAMPOLINES: [MaterialMixTrampoline; MATERIAL_MIX_SLOT_COUNT] = [
    restitution_trampoline::<0>,
    restitution_trampoline::<1>,
    restitution_trampoline::<2>,
    restitution_trampoline::<3>,
    restitution_trampoline::<4>,
    restitution_trampoline::<5>,
    restitution_trampoline::<6>,
    restitution_trampoline::<7>,
    restitution_trampoline::<8>,
    restitution_trampoline::<9>,
    restitution_trampoline::<10>,
    restitution_trampoline::<11>,
    restitution_trampoline::<12>,
    restitution_trampoline::<13>,
    restitution_trampoline::<14>,
    restitution_trampoline::<15>,
    restitution_trampoline::<16>,
    restitution_trampoline::<17>,
    restitution_trampoline::<18>,
    restitution_trampoline::<19>,
    restitution_trampoline::<20>,
    restitution_trampoline::<21>,
    restitution_trampoline::<22>,
    restitution_trampoline::<23>,
    restitution_trampoline::<24>,
    restitution_trampoline::<25>,
    restitution_trampoline::<26>,
    restitution_trampoline::<27>,
    restitution_trampoline::<28>,
    restitution_trampoline::<29>,
    restitution_trampoline::<30>,
    restitution_trampoline::<31>,
    restitution_trampoline::<32>,
    restitution_trampoline::<33>,
    restitution_trampoline::<34>,
    restitution_trampoline::<35>,
    restitution_trampoline::<36>,
    restitution_trampoline::<37>,
    restitution_trampoline::<38>,
    restitution_trampoline::<39>,
    restitution_trampoline::<40>,
    restitution_trampoline::<41>,
    restitution_trampoline::<42>,
    restitution_trampoline::<43>,
    restitution_trampoline::<44>,
    restitution_trampoline::<45>,
    restitution_trampoline::<46>,
    restitution_trampoline::<47>,
    restitution_trampoline::<48>,
    restitution_trampoline::<49>,
    restitution_trampoline::<50>,
    restitution_trampoline::<51>,
    restitution_trampoline::<52>,
    restitution_trampoline::<53>,
    restitution_trampoline::<54>,
    restitution_trampoline::<55>,
    restitution_trampoline::<56>,
    restitution_trampoline::<57>,
    restitution_trampoline::<58>,
    restitution_trampoline::<59>,
    restitution_trampoline::<60>,
    restitution_trampoline::<61>,
    restitution_trampoline::<62>,
    restitution_trampoline::<63>,
];

#[inline]
pub(crate) fn friction_callback(slot: usize) -> ffi::b2FrictionCallback {
    Some(FRICTION_TRAMPOLINES[slot])
}

#[inline]
pub(crate) fn restitution_callback(slot: usize) -> ffi::b2RestitutionCallback {
    Some(RESTITUTION_TRAMPOLINES[slot])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn null_callbacks_preserve_box2d_default_mixing_rules() {
        let friction = unsafe {
            invoke_mix_callback(
                core::ptr::null_mut(),
                0.25,
                11,
                0.81,
                22,
                default_friction_mix,
            )
        };
        let restitution = unsafe {
            invoke_mix_callback(
                core::ptr::null_mut(),
                0.25,
                11,
                0.81,
                22,
                default_restitution_mix,
            )
        };

        assert_eq!(friction, (0.25_f32 * 0.81).sqrt());
        assert_eq!(restitution, 0.81);
    }

    #[test]
    fn panicking_material_callbacks_use_exact_defaults_and_worker_reuse() {
        let world = crate::World::new(crate::WorldDef::default()).unwrap();
        let worker = Arc::clone(&world.core().worker_callbacks);
        let context = Box::new(MaterialMixCtx {
            worker: Arc::clone(&worker),
            cb: Box::new(|_, _| -> f32 { panic!("friction mix test panic") }),
        });
        let context = Box::into_raw(context);

        let first =
            unsafe { invoke_mix_callback(context, 0.25, 11, 0.81, 22, default_friction_mix) };
        let second =
            unsafe { invoke_mix_callback(context, 0.25, 11, 0.81, 22, default_friction_mix) };
        let expected_friction = (0.25_f32 * 0.81).sqrt();
        assert_eq!(first, expected_friction);
        assert_eq!(second, expected_friction);

        worker.clear_panic();
        // SAFETY: `context` is no longer used after both callback invocations.
        drop(unsafe { Box::from_raw(context) });

        let context = Box::new(MaterialMixCtx {
            worker: Arc::clone(&worker),
            cb: Box::new(|_, _| -> f32 { panic!("restitution mix test panic") }),
        });
        let context = Box::into_raw(context);
        let first =
            unsafe { invoke_mix_callback(context, 0.25, 11, 0.81, 22, default_restitution_mix) };
        let second =
            unsafe { invoke_mix_callback(context, 0.25, 11, 0.81, 22, default_restitution_mix) };
        assert_eq!(first, 0.81);
        assert_eq!(second, 0.81);

        worker.clear_panic();
        // SAFETY: `context` is no longer used after both callback invocations.
        drop(unsafe { Box::from_raw(context) });
    }
}
