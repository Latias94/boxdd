use super::*;
use crate::world::{BodyCall, OwnerCreation};

fn create_shape<G, R>(
    creation: OwnerCreation<'_>,
    body: BodyCall<'_>,
    def: &ShapeDef,
    geometry: &G,
    validate_geometry: impl FnOnce(&G) -> Result<()>,
    into_raw: impl FnOnce(&G) -> R,
    create_raw: impl FnOnce(ffi::b2BodyId, &ffi::b2ShapeDef, &R) -> ffi::b2ShapeId,
) -> Result<ShapeId> {
    if let Err(error) = check_shape_def_valid(def) {
        return creation.abort(error);
    }
    if let Err(error) = validate_geometry(geometry) {
        return creation.abort(error);
    }
    let pending = match body.reserve_shape_creation() {
        Ok(pending) => pending,
        Err(error) => return creation.abort(error),
    };
    let raw = into_raw(geometry);
    let prepared = def.prepare();
    let raw_id = create_raw(body.id().into_raw(), &prepared, &raw);
    let mut native = match body.claim_created_shape(raw_id, def.updates_body_mass()) {
        Ok(native) => native,
        Err(error) => return creation.abort(error),
    };
    let bound = match body.bind_created_shape(pending, raw_id) {
        Ok(bound) => bound,
        Err(error) => return creation.abort(error),
    };
    creation.finish(|| {
        let id = bound.publish();
        native.commit();
        id
    })
}

macro_rules! shape_creator {
    ($name:ident, $geometry:ty, $validate:path, $create:path) => {
        pub(crate) fn $name(
            creation: OwnerCreation<'_>,
            body: BodyCall<'_>,
            def: &ShapeDef,
            geometry: &$geometry,
        ) -> Result<ShapeId> {
            create_shape(
                creation,
                body,
                def,
                geometry,
                $validate,
                |geometry| geometry.into_raw(),
                |body, def, raw| unsafe { $create(body, def, raw) },
            )
        }
    };
}

shape_creator!(
    create_segment_shape_for_body,
    Segment,
    check_segment_geometry_valid,
    ffi::b2CreateSegmentShape
);
shape_creator!(
    create_chain_segment_shape_for_body,
    ChainSegment,
    check_chain_segment_geometry_valid,
    ffi::b2CreateChainSegmentShape
);
shape_creator!(
    create_capsule_shape_for_body,
    Capsule,
    check_capsule_geometry_valid,
    ffi::b2CreateCapsuleShape
);
shape_creator!(
    create_polygon_shape_for_body,
    Polygon,
    check_polygon_geometry_valid,
    ffi::b2CreatePolygonShape
);
shape_creator!(
    create_circle_shape_for_body,
    Circle,
    check_circle_geometry_valid,
    ffi::b2CreateCircleShape
);
