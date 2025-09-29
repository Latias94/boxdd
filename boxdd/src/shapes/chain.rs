use std::marker::PhantomData;

use crate::body::Body;
use crate::shapes::SurfaceMaterial;
use crate::types::ShapeId;
use boxdd_sys::ffi;

/// A chain shape attached to a body.
pub struct Chain<'b, 'w> {
    pub(crate) id: ffi::b2ChainId,
    _owner: PhantomData<&'b Body<'w>>,
}

impl<'b, 'w> Chain<'b, 'w> {
    pub fn id(&self) -> ffi::b2ChainId {
        self.id
    }
    pub fn segment_count(&self) -> i32 {
        unsafe { ffi::b2Chain_GetSegmentCount(self.id) }
    }
    // Upstream Box2D header declares b2Chain_GetSurfaceMaterialCount but some revisions
    // lack a definition. To avoid linker issues, fall back to segment_count(). The spec says
    // this count is either 1 or the segment count. If you need exact distinction later,
    // prefer storing what you set in ChainDef or revisit once upstream exposes the getter.
    pub fn surface_material_count(&self) -> i32 {
        self.segment_count()
    }

    /// Collect all segment shape ids for this chain.
    pub fn segments(&self) -> Vec<ShapeId> {
        let count = self.segment_count().max(0) as usize;
        if count == 0 {
            return Vec::new();
        }
        let mut vec: Vec<ShapeId> = Vec::with_capacity(count);
        // Safety: create temporary buffer to be filled by C, then set_len to returned count (clamped)
        let wrote = unsafe { ffi::b2Chain_GetSegments(self.id, vec.as_mut_ptr(), count as i32) }
            .max(0) as usize;
        unsafe { vec.set_len(wrote.min(count)) };
        vec
    }

    pub fn set_surface_material(&mut self, index: i32, material: &SurfaceMaterial) {
        unsafe { ffi::b2Chain_SetSurfaceMaterial(self.id, &material.0, index) }
    }

    pub fn surface_material(&self, index: i32) -> SurfaceMaterial {
        SurfaceMaterial(unsafe { ffi::b2Chain_GetSurfaceMaterial(self.id, index) })
    }
}

impl<'b, 'w> Drop for Chain<'b, 'w> {
    fn drop(&mut self) {
        if unsafe { ffi::b2Chain_IsValid(self.id) } {
            unsafe { ffi::b2DestroyChain(self.id) }
        }
    }
}

/// Chain shape definition. Holds optional owned data for points and materials.
#[derive(Clone, Debug)]
pub struct ChainDef {
    pub(crate) def: ffi::b2ChainDef,
    points: Vec<ffi::b2Vec2>,
    materials: Vec<ffi::b2SurfaceMaterial>,
}

impl Default for ChainDef {
    fn default() -> Self {
        Self {
            def: unsafe { ffi::b2DefaultChainDef() },
            points: Vec::new(),
            materials: Vec::new(),
        }
    }
}

impl ChainDef {
    pub fn builder() -> ChainDefBuilder {
        ChainDefBuilder {
            inner: Self::default(),
        }
    }
    #[cfg(feature = "serialize")]
    pub fn points_vec(&self) -> Vec<ffi::b2Vec2> {
        self.points.clone()
    }
    #[cfg(feature = "serialize")]
    pub fn materials_vec(&self) -> Vec<ffi::b2SurfaceMaterial> {
        self.materials.clone()
    }
}

#[derive(Clone, Debug)]
pub struct ChainDefBuilder {
    inner: ChainDef,
}

impl ChainDefBuilder {
    pub fn points<I, P>(mut self, points: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<crate::types::Vec2>,
    {
        self.inner.points = points
            .into_iter()
            .map(|p| ffi::b2Vec2::from(p.into()))
            .collect();
        self.inner.def.points = if self.inner.points.is_empty() {
            core::ptr::null()
        } else {
            self.inner.points.as_ptr()
        };
        self.inner.def.count = self.inner.points.len() as i32;
        self
    }
    pub fn is_loop(mut self, v: bool) -> Self {
        self.inner.def.isLoop = v;
        self
    }
    pub fn filter(mut self, f: ffi::b2Filter) -> Self {
        self.inner.def.filter = f;
        self
    }
    pub fn enable_sensor_events(mut self, v: bool) -> Self {
        self.inner.def.enableSensorEvents = v;
        self
    }
    pub fn single_material(mut self, m: &SurfaceMaterial) -> Self {
        self.inner.materials.clear();
        self.inner.def.materials = &m.0;
        self.inner.def.materialCount = 1;
        self
    }
    pub fn materials(mut self, mats: &[SurfaceMaterial]) -> Self {
        self.inner.materials = mats.iter().map(|m| m.0).collect();
        if self.inner.materials.is_empty() {
            self.inner.def.materials = core::ptr::null();
            self.inner.def.materialCount = 0;
        } else {
            self.inner.def.materials = self.inner.materials.as_ptr();
            self.inner.def.materialCount = self.inner.materials.len() as i32;
        }
        self
    }
    #[must_use]
    pub fn build(mut self) -> ChainDef {
        if self.inner.def.count == 0 {
            // ensure sane default
            self.inner.points.clear();
            self.inner.def.points = core::ptr::null();
        }
        self.inner
    }
}

impl<'w> Body<'w> {
    /// Create a chain shape attached to this body. Points/materials are cloned internally by Box2D.
    pub fn create_chain<'b>(&'b mut self, def: &ChainDef) -> Chain<'b, 'w> {
        let id = unsafe { ffi::b2CreateChain(self.id, &def.def) };
        Chain {
            id,
            _owner: PhantomData,
        }
    }
}
