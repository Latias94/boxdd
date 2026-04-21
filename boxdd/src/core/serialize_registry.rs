use boxdd_sys::ffi;

use crate::{
    Filter,
    shapes::SurfaceMaterial,
    types::{BodyId, ChainId, ShapeId, Vec2},
};

#[derive(Copy, Clone, Debug)]
pub struct ShapeFlagsRecord {
    pub enable_custom_filtering: bool,
    pub enable_sensor_events: bool,
    pub enable_contact_events: bool,
    pub enable_hit_events: bool,
    pub enable_pre_solve_events: bool,
    pub invoke_contact_creation: bool,
}

/// Recorded chain material configuration captured at chain creation time.
#[derive(Clone, Debug)]
pub enum ChainMaterialsRecord {
    /// Use Box2D's default chain material.
    Default,
    /// Use one material for the entire chain.
    Single(SurfaceMaterial),
    /// Use one material per chain segment.
    Multiple(Vec<SurfaceMaterial>),
}

impl ChainMaterialsRecord {
    fn from_raw_slice(materials: &[ffi::b2SurfaceMaterial]) -> Self {
        match materials {
            [] => Self::Default,
            [material] => Self::Single(SurfaceMaterial::from_raw(*material)),
            _ => Self::Multiple(
                materials
                    .iter()
                    .copied()
                    .map(SurfaceMaterial::from_raw)
                    .collect(),
            ),
        }
    }
}

/// Recorded chain creation parameters captured by `World::chain_records()`.
#[derive(Clone, Debug)]
pub struct ChainCreateRecord {
    pub body: BodyId,
    pub is_loop: bool,
    pub filter: Filter,
    pub enable_sensor_events: bool,
    pub points: Vec<Vec2>,
    pub materials: ChainMaterialsRecord,
}

#[derive(Clone)]
pub(crate) struct ChainCreateMeta {
    pub(crate) body: BodyId,
    pub(crate) is_loop: bool,
    pub(crate) filter: Filter,
    pub(crate) enable_sensor_events: bool,
    pub(crate) points: Vec<Vec2>,
    pub(crate) materials: ChainMaterialsRecord,
}

impl ChainCreateMeta {
    pub(crate) fn from_def(body: BodyId, def: &crate::shapes::chain::ChainDef) -> Self {
        Self {
            body,
            is_loop: def.def.isLoop,
            filter: Filter::from_raw(def.def.filter),
            enable_sensor_events: def.def.enableSensorEvents,
            points: def
                .points_raw_slice()
                .iter()
                .copied()
                .map(Vec2::from_raw)
                .collect(),
            materials: ChainMaterialsRecord::from_raw_slice(def.materials_raw_slice()),
        }
    }

    pub(crate) fn to_record(&self) -> ChainCreateRecord {
        ChainCreateRecord {
            body: self.body,
            is_loop: self.is_loop,
            filter: self.filter,
            enable_sensor_events: self.enable_sensor_events,
            points: self.points.clone(),
            materials: self.materials.clone(),
        }
    }
}

#[derive(Default)]
pub(crate) struct Registries {
    bodies: Vec<BodyId>,
    chains: Vec<(ChainId, ChainCreateMeta)>,
    shape_flags: Vec<(ShapeId, ShapeFlagsRecord)>,
}

#[inline]
fn eq_body(a: BodyId, b: BodyId) -> bool {
    a.index1 == b.index1 && a.world0 == b.world0 && a.generation == b.generation
}

#[inline]
fn eq_shape(a: ShapeId, b: ShapeId) -> bool {
    a.index1 == b.index1 && a.world0 == b.world0 && a.generation == b.generation
}

#[inline]
fn eq_chain(a: ChainId, b: ChainId) -> bool {
    a.index1 == b.index1 && a.world0 == b.world0 && a.generation == b.generation
}

impl Registries {
    pub(crate) fn record_body(&mut self, id: BodyId) {
        self.bodies.push(id);
    }

    pub(crate) fn remove_body(&mut self, id: BodyId) {
        self.bodies.retain(|&x| !eq_body(x, id));
    }

    pub(crate) fn record_chain(&mut self, id: ChainId, meta: ChainCreateMeta) {
        self.chains.push((id, meta));
    }

    pub(crate) fn remove_chain(&mut self, id: ChainId) {
        self.chains.retain(|(x, _)| !eq_chain(*x, id));
    }

    pub(crate) fn remove_chains_for_body(&mut self, body: BodyId) {
        self.chains.retain(|(_, meta)| !eq_body(meta.body, body));
    }

    pub(crate) fn record_shape_flags(&mut self, sid: ShapeId, def: &ffi::b2ShapeDef) {
        let rec = ShapeFlagsRecord {
            enable_custom_filtering: def.enableCustomFiltering,
            enable_sensor_events: def.enableSensorEvents,
            enable_contact_events: def.enableContactEvents,
            enable_hit_events: def.enableHitEvents,
            enable_pre_solve_events: def.enablePreSolveEvents,
            invoke_contact_creation: def.invokeContactCreation,
        };
        if let Some(slot) = self
            .shape_flags
            .iter_mut()
            .find(|(id, _)| eq_shape(*id, sid))
        {
            *slot = (sid, rec);
        } else {
            self.shape_flags.push((sid, rec));
        }
    }

    pub(crate) fn remove_shape_flags(&mut self, sid: ShapeId) {
        self.shape_flags.retain(|(x, _)| !eq_shape(*x, sid));
    }

    pub(crate) fn remove_shape_flags_for_body(&mut self, body: BodyId) {
        // Enumerate shapes on this body while it is still valid.
        let raw_body = body.into_raw();
        let count = unsafe { ffi::b2Body_GetShapeCount(raw_body) }.max(0) as usize;
        if count == 0 {
            return;
        }
        let arr = unsafe {
            crate::core::ffi_vec::read_from_ffi(count, |ptr: *mut ShapeId, count| {
                ffi::b2Body_GetShapes(raw_body, ptr.cast(), count)
            })
        };
        for sid in arr {
            self.remove_shape_flags(sid);
        }
    }

    pub(crate) fn body_ids(&self) -> Vec<BodyId> {
        let mut out = Vec::new();
        self.body_ids_into(&mut out);
        out
    }

    pub(crate) fn body_ids_into(&self, out: &mut Vec<BodyId>) {
        out.clear();
        out.extend(
            self.bodies
                .iter()
                .copied()
                .filter(|&bid| unsafe { ffi::b2Body_IsValid(bid.into_raw()) }),
        );
    }

    pub(crate) fn chain_records(&self) -> Vec<ChainCreateRecord> {
        let mut out = Vec::new();
        self.chain_records_into(&mut out);
        out
    }

    pub(crate) fn chain_records_into(&self, out: &mut Vec<ChainCreateRecord>) {
        out.clear();
        out.extend(self.chains.iter().filter_map(|(id, meta)| {
            if unsafe { ffi::b2Chain_IsValid(id.into_raw()) } {
                Some(meta.to_record())
            } else {
                None
            }
        }));
    }

    pub(crate) fn shape_flags(&self, sid: ShapeId) -> Option<ShapeFlagsRecord> {
        self.shape_flags
            .iter()
            .find_map(|(id, rec)| if eq_shape(*id, sid) { Some(*rec) } else { None })
    }
}
