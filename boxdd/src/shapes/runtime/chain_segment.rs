use super::*;

#[inline]
fn shape_has_parent_chain(id: ShapeId) -> bool {
    unsafe { ffi::b2Shape_GetParentChain(id.into_raw()) }.index1 != 0
}

/// Check the invariant shared by every operation that can replace a shape's geometry type.
///
/// Chain-owned segments are indexed by their parent chain. Changing their type would make the
/// parent lose the only native ownership marker while retaining a stale shape index.
pub(crate) fn check_orphan_shape_mutation_target(id: ShapeId) -> Result<()> {
    if shape_has_parent_chain(id) {
        Err(Error::ChainOwnedShape)
    } else {
        Ok(())
    }
}

/// Change a shape to an orphan chain segment, or update an existing orphan segment.
///
/// Box2D asserts when this operation targets a segment owned by a `b2ChainShape`. Keep that
/// ownership check in the one helper used by every Safe Rust receiver.
pub(crate) fn set_chain_segment_checked(id: ShapeId, chain_segment: &ChainSegment) -> Result<()> {
    check_chain_segment_geometry_valid(chain_segment)?;
    check_orphan_shape_mutation_target(id)?;

    let raw = chain_segment.into_raw();
    unsafe { ffi::b2Shape_SetChainSegment(id.into_raw(), &raw) };
    Ok(())
}
