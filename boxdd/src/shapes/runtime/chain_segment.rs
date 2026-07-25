use super::*;

#[inline]
fn shape_has_parent_chain(id: ShapeId) -> bool {
    unsafe { ffi::b2Shape_GetParentChain(id.into_raw()) }.index1 != 0
}

/// Check the invariant shared by every operation that can replace a shape's geometry type.
///
/// Chain-owned segments are indexed by their parent chain. Changing their type would make the
/// parent lose the only native ownership marker while retaining a stale shape index.
pub(crate) fn try_check_orphan_shape_mutation_target(
    core: &crate::core::world_core::WorldCore,
    id: ShapeId,
) -> ApiResult<()> {
    try_check_orphan_shape_mutation_target_with_access(
        core,
        id,
        crate::core::world_core::WorldAccess::Idle,
    )
}

pub(crate) fn try_check_orphan_shape_mutation_target_with_access(
    core: &crate::core::world_core::WorldCore,
    id: ShapeId,
    access: crate::core::world_core::WorldAccess,
) -> ApiResult<()> {
    crate::core::callback_state::check_not_in_callback()?;
    core.check_shape_with_access(id, access)?;
    if shape_has_parent_chain(id) {
        Err(ApiError::ChainOwnedShape)
    } else {
        Ok(())
    }
}

#[track_caller]
pub(crate) fn assert_orphan_shape_mutation_target(
    core: &crate::core::world_core::WorldCore,
    id: ShapeId,
) {
    try_check_orphan_shape_mutation_target(core, id)
        .expect("shape must be live and not owned by a chain");
}

/// Change a shape to an orphan chain segment, or update an existing orphan segment.
///
/// Box2D asserts when this operation targets a segment owned by a `b2ChainShape`. Keep that
/// ownership check in the one helper used by every Safe Rust receiver.
pub(crate) fn try_shape_set_chain_segment_checked_impl(
    core: &crate::core::world_core::WorldCore,
    id: ShapeId,
    chain_segment: &ChainSegment,
) -> ApiResult<()> {
    try_shape_set_chain_segment_checked_with_access(
        core,
        id,
        chain_segment,
        crate::core::world_core::WorldAccess::Idle,
    )
}

pub(crate) fn try_shape_set_chain_segment_checked_with_access(
    core: &crate::core::world_core::WorldCore,
    id: ShapeId,
    chain_segment: &ChainSegment,
    access: crate::core::world_core::WorldAccess,
) -> ApiResult<()> {
    check_chain_segment_geometry_valid(chain_segment)?;
    try_check_orphan_shape_mutation_target_with_access(core, id, access)?;

    let raw = chain_segment.into_raw();
    unsafe { ffi::b2Shape_SetChainSegment(id.into_raw(), &raw) };
    Ok(())
}

#[track_caller]
pub(crate) fn shape_set_chain_segment_checked_impl(
    core: &crate::core::world_core::WorldCore,
    id: ShapeId,
    chain_segment: &ChainSegment,
) {
    try_shape_set_chain_segment_checked_impl(core, id, chain_segment)
        .expect("shape must be live, orphaned, and have valid chain-segment geometry");
}
