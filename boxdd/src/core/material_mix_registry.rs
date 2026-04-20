use crate::core::world_core::MaterialMixCtx;
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
    (friction_a * friction_b).sqrt()
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
    let Some(core) = ctx.core.upgrade() else {
        return default_mix(value_a, value_b);
    };

    if core.callback_panicked.load(Ordering::Relaxed) {
        return default_mix(value_a, value_b);
    }

    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _g = crate::core::callback_state::CallbackGuard::enter();
        (ctx.cb)(
            MaterialMixInput::new(value_a, user_material_id_a),
            MaterialMixInput::new(value_b, user_material_id_b),
        )
    })) {
        Ok(v) => v,
        Err(payload) => {
            if !core.callback_panicked.swap(true, Ordering::SeqCst) {
                *core
                    .callback_panic
                    .lock()
                    .expect("callback_panic mutex poisoned") = Some(payload);
            }
            default_mix(value_a, value_b)
        }
    }
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

macro_rules! define_material_mix_trampolines {
    ($(($friction_name:ident, $restitution_name:ident, $idx:expr)),* $(,)?) => {
        $(
            unsafe extern "C" fn $friction_name(
                friction_a: f32,
                user_material_id_a: u64,
                friction_b: f32,
                user_material_id_b: u64,
            ) -> f32 {
                unsafe {
                    invoke_friction_callback(
                        $idx,
                        friction_a,
                        user_material_id_a,
                        friction_b,
                        user_material_id_b,
                    )
                }
            }

            unsafe extern "C" fn $restitution_name(
                restitution_a: f32,
                user_material_id_a: u64,
                restitution_b: f32,
                user_material_id_b: u64,
            ) -> f32 {
                unsafe {
                    invoke_restitution_callback(
                        $idx,
                        restitution_a,
                        user_material_id_a,
                        restitution_b,
                        user_material_id_b,
                    )
                }
            }
        )*

        static FRICTION_TRAMPOLINES: [unsafe extern "C" fn(f32, u64, f32, u64) -> f32; MATERIAL_MIX_SLOT_COUNT] = [
            $($friction_name),*
        ];

        static RESTITUTION_TRAMPOLINES: [unsafe extern "C" fn(f32, u64, f32, u64) -> f32; MATERIAL_MIX_SLOT_COUNT] = [
            $($restitution_name),*
        ];
    };
}

define_material_mix_trampolines!(
    (friction_slot_0, restitution_slot_0, 0),
    (friction_slot_1, restitution_slot_1, 1),
    (friction_slot_2, restitution_slot_2, 2),
    (friction_slot_3, restitution_slot_3, 3),
    (friction_slot_4, restitution_slot_4, 4),
    (friction_slot_5, restitution_slot_5, 5),
    (friction_slot_6, restitution_slot_6, 6),
    (friction_slot_7, restitution_slot_7, 7),
    (friction_slot_8, restitution_slot_8, 8),
    (friction_slot_9, restitution_slot_9, 9),
    (friction_slot_10, restitution_slot_10, 10),
    (friction_slot_11, restitution_slot_11, 11),
    (friction_slot_12, restitution_slot_12, 12),
    (friction_slot_13, restitution_slot_13, 13),
    (friction_slot_14, restitution_slot_14, 14),
    (friction_slot_15, restitution_slot_15, 15),
    (friction_slot_16, restitution_slot_16, 16),
    (friction_slot_17, restitution_slot_17, 17),
    (friction_slot_18, restitution_slot_18, 18),
    (friction_slot_19, restitution_slot_19, 19),
    (friction_slot_20, restitution_slot_20, 20),
    (friction_slot_21, restitution_slot_21, 21),
    (friction_slot_22, restitution_slot_22, 22),
    (friction_slot_23, restitution_slot_23, 23),
    (friction_slot_24, restitution_slot_24, 24),
    (friction_slot_25, restitution_slot_25, 25),
    (friction_slot_26, restitution_slot_26, 26),
    (friction_slot_27, restitution_slot_27, 27),
    (friction_slot_28, restitution_slot_28, 28),
    (friction_slot_29, restitution_slot_29, 29),
    (friction_slot_30, restitution_slot_30, 30),
    (friction_slot_31, restitution_slot_31, 31),
    (friction_slot_32, restitution_slot_32, 32),
    (friction_slot_33, restitution_slot_33, 33),
    (friction_slot_34, restitution_slot_34, 34),
    (friction_slot_35, restitution_slot_35, 35),
    (friction_slot_36, restitution_slot_36, 36),
    (friction_slot_37, restitution_slot_37, 37),
    (friction_slot_38, restitution_slot_38, 38),
    (friction_slot_39, restitution_slot_39, 39),
    (friction_slot_40, restitution_slot_40, 40),
    (friction_slot_41, restitution_slot_41, 41),
    (friction_slot_42, restitution_slot_42, 42),
    (friction_slot_43, restitution_slot_43, 43),
    (friction_slot_44, restitution_slot_44, 44),
    (friction_slot_45, restitution_slot_45, 45),
    (friction_slot_46, restitution_slot_46, 46),
    (friction_slot_47, restitution_slot_47, 47),
    (friction_slot_48, restitution_slot_48, 48),
    (friction_slot_49, restitution_slot_49, 49),
    (friction_slot_50, restitution_slot_50, 50),
    (friction_slot_51, restitution_slot_51, 51),
    (friction_slot_52, restitution_slot_52, 52),
    (friction_slot_53, restitution_slot_53, 53),
    (friction_slot_54, restitution_slot_54, 54),
    (friction_slot_55, restitution_slot_55, 55),
    (friction_slot_56, restitution_slot_56, 56),
    (friction_slot_57, restitution_slot_57, 57),
    (friction_slot_58, restitution_slot_58, 58),
    (friction_slot_59, restitution_slot_59, 59),
    (friction_slot_60, restitution_slot_60, 60),
    (friction_slot_61, restitution_slot_61, 61),
    (friction_slot_62, restitution_slot_62, 62),
    (friction_slot_63, restitution_slot_63, 63),
);

#[inline]
pub(crate) fn friction_callback(slot: usize) -> ffi::b2FrictionCallback {
    Some(FRICTION_TRAMPOLINES[slot])
}

#[inline]
pub(crate) fn restitution_callback(slot: usize) -> ffi::b2RestitutionCallback {
    Some(RESTITUTION_TRAMPOLINES[slot])
}
