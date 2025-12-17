use boxdd_sys::ffi;

use crate::types::{BodyId, ChainId, ShapeId};

#[derive(Copy, Clone, Debug)]
pub struct ShapeFlagsRecord {
    pub enable_custom_filtering: bool,
    pub enable_sensor_events: bool,
    pub enable_contact_events: bool,
    pub enable_hit_events: bool,
    pub enable_pre_solve_events: bool,
    pub invoke_contact_creation: bool,
}

#[derive(Clone)]
pub struct ChainCreateRecord {
    pub body: ffi::b2BodyId,
    pub is_loop: bool,
    pub filter: ffi::b2Filter,
    pub enable_sensor_events: bool,
    pub points: Vec<ffi::b2Vec2>,
    pub materials: Vec<ffi::b2SurfaceMaterial>,
}

#[derive(Clone)]
pub(crate) struct ChainCreateMeta {
    pub(crate) body: ffi::b2BodyId,
    pub(crate) is_loop: bool,
    pub(crate) filter: ffi::b2Filter,
    pub(crate) enable_sensor_events: bool,
    pub(crate) points: Vec<ffi::b2Vec2>,
    pub(crate) materials: Vec<ffi::b2SurfaceMaterial>,
}

impl ChainCreateMeta {
    pub(crate) fn from_def(body: ffi::b2BodyId, def: &crate::shapes::chain::ChainDef) -> Self {
        Self {
            body,
            is_loop: def.def.isLoop,
            filter: def.def.filter,
            enable_sensor_events: def.def.enableSensorEvents,
            points: def.points_vec(),
            materials: def.materials_vec(),
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
        let count = unsafe { ffi::b2Body_GetShapeCount(body) }.max(0) as usize;
        if count == 0 {
            return;
        }
        let mut arr: Vec<ShapeId> = Vec::with_capacity(count);
        let wrote =
            unsafe { ffi::b2Body_GetShapes(body, arr.as_mut_ptr(), count as i32) }.max(0) as usize;
        unsafe { arr.set_len(wrote.min(count)) };
        for sid in arr {
            self.remove_shape_flags(sid);
        }
    }

    pub(crate) fn body_ids(&self) -> Vec<BodyId> {
        self.bodies
            .iter()
            .copied()
            .filter(|&bid| unsafe { ffi::b2Body_IsValid(bid) })
            .collect()
    }

    pub(crate) fn chain_records(&self) -> Vec<ChainCreateRecord> {
        self.chains
            .iter()
            .filter_map(|(id, meta)| {
                if unsafe { ffi::b2Chain_IsValid(*id) } {
                    Some(meta.to_record())
                } else {
                    None
                }
            })
            .collect()
    }

    pub(crate) fn shape_flags(&self, sid: ShapeId) -> Option<ShapeFlagsRecord> {
        self.shape_flags
            .iter()
            .find_map(|(id, rec)| if eq_shape(*id, sid) { Some(*rec) } else { None })
    }
}
