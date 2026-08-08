use crate::body::Body;
use crate::error::{Error, Result};
use crate::shapes::SurfaceMaterial;
use crate::types::{ChainId, ShapeId, Vec2};
use crate::world::{ChainCall, ChainProof};
use boxdd_sys::ffi;

/// How a `ChainDef` provides surface materials to Box2D.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ChainDefMaterialLayout<'a> {
    /// Use Box2D's default chain material.
    Default(SurfaceMaterial),
    /// Use one material for the entire chain.
    Single(SurfaceMaterial),
    /// Use one material entry for every stored chain point.
    Multiple(&'a [SurfaceMaterial]),
}

impl<'a> ChainDefMaterialLayout<'a> {
    /// Number of material entries visible to Box2D.
    pub const fn count(&self) -> usize {
        match self {
            Self::Default(_) | Self::Single(_) => 1,
            Self::Multiple(materials) => materials.len(),
        }
    }
}

/// A scoped chain handle tied to a mutable borrow of the world.
pub struct Chain<'w> {
    proof: ChainProof<'w>,
}

#[inline]
fn raw_chain_id(id: ChainId) -> ffi::b2ChainId {
    id.into_raw()
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum ChainMaterialAccess {
    Single,
    PerSegment,
    PerPoint,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct ChainRuntimeSurfaceMaterialLayout {
    segment_count: i32,
    access: ChainMaterialAccess,
}

impl ChainRuntimeSurfaceMaterialLayout {
    #[inline]
    const fn visible_count(self) -> i32 {
        match self.access {
            ChainMaterialAccess::Single => 1,
            ChainMaterialAccess::PerSegment | ChainMaterialAccess::PerPoint => self.segment_count,
        }
    }
}

#[inline]
fn check_native_chain_segment_count(operation: &'static str, count: i32) -> Result<i32> {
    if count > 0 {
        Ok(count)
    } else {
        Err(Error::InvalidNativeOutput {
            operation,
            output: "segment_count",
            constraint: "a positive native int",
        })
    }
}

#[inline]
fn check_native_chain_material_layout(
    operation: &'static str,
    raw_material_count: i32,
    segment_count: i32,
) -> Result<ChainRuntimeSurfaceMaterialLayout> {
    let segment_count = check_native_chain_segment_count(operation, segment_count)?;
    let access = if raw_material_count == 1 {
        ChainMaterialAccess::Single
    } else if raw_material_count == segment_count {
        ChainMaterialAccess::PerSegment
    } else if segment_count.checked_add(3) == Some(raw_material_count) {
        ChainMaterialAccess::PerPoint
    } else {
        return Err(Error::InvalidNativeOutput {
            operation,
            output: "surface_material_count",
            constraint: "1, segment_count, or segment_count + 3 for an open chain",
        });
    };
    Ok(ChainRuntimeSurfaceMaterialLayout {
        segment_count,
        access,
    })
}

#[inline]
fn check_native_chain_segment_output_len(
    operation: &'static str,
    segment_count: i32,
    actual_len: usize,
) -> Result<()> {
    if usize::try_from(segment_count) == Ok(actual_len) {
        Ok(())
    } else {
        Err(Error::InvalidNativeOutput {
            operation,
            output: "segments",
            constraint: "exactly segment_count entries",
        })
    }
}

unsafe fn try_read_segment_output(
    resolver: &crate::core::identity_registry::OutputIdentityResolver<'_>,
    requested: i32,
    fill: impl FnOnce(*mut ffi::b2ShapeId, i32) -> i32,
) -> Result<Vec<ShapeId>> {
    unsafe {
        crate::core::ffi_vec::try_read_mapped_from_ffi(requested, fill, |raw| resolver.shape(raw))
    }
}

fn chain_segments_in_impl(operation: &'static str, chain: ChainCall<'_>) -> Result<Vec<ShapeId>> {
    let id = raw_chain_id(chain.id());
    let count =
        check_native_chain_segment_count(operation, unsafe { ffi::b2Chain_GetSegmentCount(id) })?;
    let segments = chain.with_output_identity_resolver(|resolver| unsafe {
        try_read_segment_output(resolver, count, |ptr, count| {
            ffi::b2Chain_GetSegments(id, ptr, count)
        })
    })?;
    check_native_chain_segment_output_len(operation, count, segments.len())?;
    Ok(segments)
}

#[cfg(test)]
mod segment_output_tests {
    use super::*;

    fn test_registry() -> (
        crate::id::IdBrand,
        std::sync::Arc<crate::core::identity_registry::ActiveIdentityRegistry>,
    ) {
        let brand = crate::id::IdBrand::new(
            ffi::b2WorldId {
                index1: 4,
                generation: 7,
            },
            crate::id::WorldToken::allocate().unwrap(),
        )
        .unwrap();
        let registry = crate::core::identity_registry::ActiveIdentityRegistry::new(brand);
        let body = registry
            .register_body(ffi::b2BodyId {
                index1: 1,
                world0: brand.world0(),
                generation: 1,
            })
            .unwrap();
        for index1 in [2, 9] {
            registry
                .register_shape(raw_shape(index1, brand.world0()), body)
                .unwrap();
        }
        (brand, registry)
    }

    fn raw_shape(index1: i32, world0: u16) -> ffi::b2ShapeId {
        ffi::b2ShapeId {
            index1,
            world0,
            generation: 1,
        }
    }

    #[test]
    fn segment_output_rejects_invalid_ids() {
        let (brand, registry) = test_registry();
        let error = registry
            .with_output_resolver(|resolver| unsafe {
                try_read_segment_output(resolver, 1, |ptr, _capacity| {
                    ptr.write(raw_shape(1, brand.world0().wrapping_add(1)));
                    1
                })
            })
            .unwrap_err();
        assert_eq!(error, Error::WrongWorld);

        let error = registry
            .with_output_resolver(|resolver| unsafe {
                try_read_segment_output(resolver, 1, |ptr, _capacity| {
                    ptr.write(raw_shape(0, brand.world0()));
                    1
                })
            })
            .unwrap_err();
        assert_eq!(error, Error::InvalidShapeId);

        let out = registry
            .with_output_resolver(|resolver| unsafe {
                try_read_segment_output(resolver, 1, |ptr, _capacity| {
                    ptr.write(raw_shape(2, brand.world0()));
                    1
                })
            })
            .unwrap();
        let raw = out[0].into_raw();
        assert_eq!(raw.index1, 2);
        assert_eq!(raw.world0, brand.world0());
        assert_eq!(raw.generation, 1);
    }

    #[test]
    fn native_chain_counts_and_material_layout_fail_closed() {
        assert_eq!(
            check_native_chain_segment_count("Chain::segment_count", 0),
            Err(Error::InvalidNativeOutput {
                operation: "Chain::segment_count",
                output: "segment_count",
                constraint: "a positive native int",
            })
        );
        assert_eq!(
            check_native_chain_material_layout("Chain::surface_material_count", 5, 1),
            Err(Error::InvalidNativeOutput {
                operation: "Chain::surface_material_count",
                output: "surface_material_count",
                constraint: "1, segment_count, or segment_count + 3 for an open chain",
            })
        );
        assert_eq!(
            check_native_chain_segment_output_len("Chain::segments", 2, 1),
            Err(Error::InvalidNativeOutput {
                operation: "Chain::segments",
                output: "segments",
                constraint: "exactly segment_count entries",
            })
        );

        assert_eq!(
            check_native_chain_material_layout("Chain::surface_material_count", 1, 4),
            Ok(ChainRuntimeSurfaceMaterialLayout {
                segment_count: 4,
                access: ChainMaterialAccess::Single,
            })
        );
        assert_eq!(
            check_native_chain_material_layout("Chain::surface_material_count", 4, 4),
            Ok(ChainRuntimeSurfaceMaterialLayout {
                segment_count: 4,
                access: ChainMaterialAccess::PerSegment,
            })
        );
        assert_eq!(
            check_native_chain_material_layout("Chain::surface_material_count", 4, 1),
            Ok(ChainRuntimeSurfaceMaterialLayout {
                segment_count: 1,
                access: ChainMaterialAccess::PerPoint,
            })
        );
    }
}

#[inline]
fn chain_segment_count_impl(operation: &'static str, id: ChainId) -> Result<i32> {
    check_native_chain_segment_count(operation, unsafe {
        ffi::b2Chain_GetSegmentCount(raw_chain_id(id))
    })
}

#[inline]
fn chain_runtime_surface_material_layout_impl(
    operation: &'static str,
    id: ChainId,
) -> Result<ChainRuntimeSurfaceMaterialLayout> {
    let raw_count = unsafe { ffi::b2Chain_GetSurfaceMaterialCount(raw_chain_id(id)) };
    let segment_count = unsafe { ffi::b2Chain_GetSegmentCount(raw_chain_id(id)) };
    check_native_chain_material_layout(operation, raw_count, segment_count)
}

#[inline]
fn chain_surface_material_count_impl(operation: &'static str, id: ChainId) -> Result<i32> {
    Ok(chain_runtime_surface_material_layout_impl(operation, id)?.visible_count())
}

#[inline]
fn chain_set_surface_material_impl(
    operation: &'static str,
    chain: ChainCall<'_>,
    index: i32,
    material: &SurfaceMaterial,
    layout: ChainRuntimeSurfaceMaterialLayout,
) -> Result<()> {
    let id = chain.id();
    match layout.access {
        ChainMaterialAccess::Single => unsafe {
            ffi::b2Chain_SetSurfaceMaterial(raw_chain_id(id), &material.0, 0)
        },
        ChainMaterialAccess::PerSegment => unsafe {
            ffi::b2Chain_SetSurfaceMaterial(raw_chain_id(id), &material.0, index)
        },
        ChainMaterialAccess::PerPoint => {
            let segments = chain_segments_in_impl(operation, chain)?;
            let segment =
                segments
                    .get(index as usize)
                    .copied()
                    .ok_or(Error::InvalidNativeOutput {
                        operation,
                        output: "segments",
                        constraint: "an entry for every valid material index",
                    })?;
            crate::shapes::shape_set_surface_material_impl(segment, material);
        }
    }
    Ok(())
}

#[inline]
fn chain_surface_material_impl(
    operation: &'static str,
    chain: ChainCall<'_>,
    index: i32,
    layout: ChainRuntimeSurfaceMaterialLayout,
) -> Result<SurfaceMaterial> {
    let id = chain.id();
    match layout.access {
        ChainMaterialAccess::Single => SurfaceMaterial::from_raw(unsafe {
            ffi::b2Chain_GetSurfaceMaterial(raw_chain_id(id), 0)
        })
        .map_err(|_| Error::InvalidNativeOutput {
            operation,
            output: "surface_material",
            constraint: "a valid finite surface material",
        }),
        ChainMaterialAccess::PerSegment => SurfaceMaterial::from_raw(unsafe {
            ffi::b2Chain_GetSurfaceMaterial(raw_chain_id(id), index)
        })
        .map_err(|_| Error::InvalidNativeOutput {
            operation,
            output: "surface_material",
            constraint: "a valid finite surface material",
        }),
        ChainMaterialAccess::PerPoint => {
            let segments = chain_segments_in_impl(operation, chain)?;
            let segment =
                segments
                    .get(index as usize)
                    .copied()
                    .ok_or(Error::InvalidNativeOutput {
                        operation,
                        output: "segments",
                        constraint: "an entry for every valid material index",
                    })?;
            crate::shapes::shape_surface_material_impl(segment)
        }
    }
}

fn check_chain_surface_material_index_in_range(
    operation: &'static str,
    index: i32,
    count: i32,
) -> Result<()> {
    if 0 <= index && index < count {
        Ok(())
    } else {
        Err(Error::index_out_of_range(
            operation,
            i64::from(index),
            count as usize,
        ))
    }
}

impl<'w> Chain<'w> {
    pub(crate) fn new(proof: ChainProof<'w>) -> Self {
        Self { proof }
    }

    pub fn id(&self) -> ChainId {
        self.proof.id()
    }

    pub fn segment_count(&self) -> Result<i32> {
        self.proof
            .call(|chain| chain_segment_count_impl("Chain::segment_count", chain.id()))
    }

    pub fn segments(&self) -> Result<Vec<ShapeId>> {
        self.proof
            .call(|chain| chain_segments_in_impl("Chain::segments", chain))
    }

    pub fn surface_material_count(&self) -> Result<i32> {
        self.proof.call(|chain| {
            chain_surface_material_count_impl("Chain::surface_material_count", chain.id())
        })
    }

    pub fn set_surface_material(&mut self, index: i32, material: &SurfaceMaterial) -> Result<()> {
        self.proof.call(|chain| {
            const OPERATION: &str = "Chain::set_surface_material";
            crate::shapes::check_surface_material_valid(OPERATION, material)?;
            let layout = chain_runtime_surface_material_layout_impl(OPERATION, chain.id())?;
            check_chain_surface_material_index_in_range(OPERATION, index, layout.visible_count())?;
            chain_set_surface_material_impl(OPERATION, chain, index, material, layout)
        })
    }

    pub fn surface_material(&self, index: i32) -> Result<SurfaceMaterial> {
        self.proof.call(|chain| {
            const OPERATION: &str = "Chain::surface_material";
            let layout = chain_runtime_surface_material_layout_impl(OPERATION, chain.id())?;
            check_chain_surface_material_index_in_range(OPERATION, index, layout.visible_count())?;
            chain_surface_material_impl(OPERATION, chain, index, layout)
        })
    }

    /// Destroy this chain immediately.
    pub fn destroy(self) -> Result<()> {
        self.proof.call(|chain| chain.destroy())
    }
}

/// Pure Rust chain definition.
#[derive(Clone, Debug)]
pub struct ChainDef {
    points: Vec<Vec2>,
    materials: Vec<SurfaceMaterial>,
    material_source: ChainDefMaterialSource,
    filter: crate::filter::Filter,
    is_loop: bool,
    enable_sensor_events: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum ChainDefMaterialSource {
    Default,
    Explicit,
}

struct PreparedChainDef {
    raw: ffi::b2ChainDef,
    _points: Vec<ffi::b2Vec2>,
    _materials: Vec<ffi::b2SurfaceMaterial>,
}

impl PreparedChainDef {
    fn as_raw(&self) -> &ffi::b2ChainDef {
        &self.raw
    }
}

impl ChainDef {
    fn draft() -> Self {
        Self {
            points: Vec::new(),
            materials: [SurfaceMaterial::default()].into(),
            material_source: ChainDefMaterialSource::Default,
            filter: crate::filter::Filter::default(),
            is_loop: false,
            enable_sensor_events: false,
        }
    }

    /// Start building a new `ChainDef` from defaults.
    pub fn builder() -> ChainDefBuilder {
        ChainDefBuilder {
            inner: Self::draft(),
        }
    }

    /// Stored chain points, including Box2D's ghost points.
    pub fn points(&self) -> &[Vec2] {
        &self.points
    }

    /// Whether the chain is closed into a loop.
    pub const fn is_loop(&self) -> bool {
        self.is_loop
    }

    /// Collision filter used by the chain.
    pub const fn filter(&self) -> crate::filter::Filter {
        self.filter
    }

    /// Whether sensor begin/end events are enabled for the chain.
    pub const fn sensor_events_enabled(&self) -> bool {
        self.enable_sensor_events
    }

    /// Inspect the material layout supplied to the chain definition.
    pub fn material_layout(&self) -> ChainDefMaterialLayout<'_> {
        match (self.material_source, self.materials.as_slice()) {
            (ChainDefMaterialSource::Default, [material]) => {
                ChainDefMaterialLayout::Default(*material)
            }
            (ChainDefMaterialSource::Explicit, [material]) => {
                ChainDefMaterialLayout::Single(*material)
            }
            (ChainDefMaterialSource::Explicit, materials) => {
                ChainDefMaterialLayout::Multiple(materials)
            }
            (ChainDefMaterialSource::Default, _) => {
                unreachable!("default chain material storage must contain exactly one entry")
            }
        }
    }

    /// Number of material entries visible to Box2D.
    pub fn material_count(&self) -> usize {
        self.material_layout().count()
    }

    fn native_layout_is_valid(&self) -> bool {
        let point_count = self.points.len();
        let material_count = self.materials.len();
        point_count >= 4
            && point_count <= i32::MAX as usize
            && material_count <= i32::MAX as usize
            && (material_count == 1 || material_count == point_count)
    }

    fn prepare(&self) -> Result<PreparedChainDef> {
        self.validate()?;
        let points: Vec<ffi::b2Vec2> = self.points.iter().copied().map(Vec2::into_raw).collect();
        let materials: Vec<ffi::b2SurfaceMaterial> = self
            .materials
            .iter()
            .copied()
            .map(SurfaceMaterial::into_raw)
            .collect();
        let mut raw: ffi::b2ChainDef = crate::core::native_defaults::chain_def(materials.as_ptr());
        raw.points = points.as_ptr();
        raw.count = i32::try_from(points.len()).expect("validated chain point count");
        raw.materialCount = i32::try_from(materials.len()).expect("validated chain material count");
        raw.filter = self.filter.into_raw();
        raw.isLoop = self.is_loop;
        raw.enableSensorEvents = self.enable_sensor_events;
        Ok(PreparedChainDef {
            raw,
            _points: points,
            _materials: materials,
        })
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
        self.inner.points = points.into_iter().map(Into::into).collect();
        self
    }
    pub fn is_loop(mut self, v: bool) -> Self {
        self.inner.is_loop = v;
        self
    }
    pub fn filter(mut self, f: crate::filter::Filter) -> Self {
        self.inner.filter = f;
        self
    }
    pub fn enable_sensor_events(mut self, v: bool) -> Self {
        self.inner.enable_sensor_events = v;
        self
    }
    pub fn single_material(self, material: &SurfaceMaterial) -> Self {
        self.materials(core::slice::from_ref(material))
    }
    pub fn materials(mut self, mats: &[SurfaceMaterial]) -> Self {
        if mats.is_empty() {
            self.inner.materials = [SurfaceMaterial::default()].into();
            self.inner.material_source = ChainDefMaterialSource::Default;
        } else {
            self.inner.materials = mats.to_vec();
            self.inner.material_source = ChainDefMaterialSource::Explicit;
        }
        self
    }
    pub fn build(self) -> Result<ChainDef> {
        self.inner.validate()?;
        Ok(self.inner)
    }
}

impl From<ChainDef> for ChainDefBuilder {
    fn from(def: ChainDef) -> Self {
        Self { inner: def }
    }
}

pub(crate) fn check_chain_def_valid(def: &ChainDef) -> Result<()> {
    if !def.native_layout_is_valid() {
        return Err(Error::InvalidChainDef);
    }
    if !def.points.iter().copied().all(Vec2::is_valid) {
        return Err(Error::InvalidChainDef);
    }
    let length_units_per_meter = crate::core::foundation::current_length_units_per_meter()?;
    if !crate::shapes::geometry::points_have_minimum_pairwise_separation(
        &def.points,
        length_units_per_meter,
    )
    .map_err(|_| Error::InvalidChainDef)?
    {
        return Err(Error::InvalidChainDef);
    }

    if def.material_source == ChainDefMaterialSource::Default && def.materials.len() != 1 {
        return Err(Error::InvalidChainDef);
    }
    for material in &def.materials {
        crate::shapes::check_surface_material_valid("ChainDef::validate", material)
            .map_err(|_| Error::InvalidChainDef)?;
    }

    Ok(())
}

fn create_chain_for_body(
    creation: crate::world::OwnerCreation<'_>,
    body: crate::world::BodyCall<'_>,
    def: &ChainDef,
) -> Result<ChainId> {
    let prepared = match def.prepare() {
        Ok(prepared) => prepared,
        Err(error) => return creation.abort(error),
    };
    let segment_count = if def.is_loop() {
        def.points.len()
    } else {
        def.points.len() - 3
    };
    let pending = match body.reserve_chain_creation(segment_count) {
        Ok(pending) => pending,
        Err(error) => return creation.abort(error),
    };
    let raw = unsafe { ffi::b2CreateChain(body.id().into_raw(), prepared.as_raw()) };
    let mut native = match body.claim_created_chain(raw) {
        Ok(native) => native,
        Err(error) => return creation.abort(error),
    };
    let bound = match body.bind_created_chain(pending, raw) {
        Ok(bound) => bound,
        Err(error) => return creation.abort(error),
    };
    creation.finish(|| {
        let id = bound.publish();
        native.commit();
        id
    })
}

impl ChainDef {
    pub fn validate(&self) -> Result<()> {
        check_chain_def_valid(self)
    }
}

impl Body<'_> {
    /// Create a chain attached to this body and return its storage id.
    pub fn create_chain(&mut self, def: &ChainDef) -> Result<ChainId> {
        let (creation, body) = self.proof.begin_creation()?;
        create_chain_for_body(creation, body, def)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_creation_registers_chain_and_segment_identities_before_returning() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let body = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let def = ChainDef::builder()
            .points([
                Vec2::new(-2.0, 0.0),
                Vec2::new(-1.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(2.0, 0.0),
            ])
            .build()
            .unwrap();
        let chain = world.body(body).unwrap().create_chain(&def).unwrap();
        let segments = world.chain(chain).unwrap().segments().unwrap();
        let core = world.core();

        assert_eq!(core.check_chain(chain), Ok(()));
        assert!(!segments.is_empty());
        assert!(
            segments
                .iter()
                .all(|&segment| core.check_shape(segment).is_ok())
        );
    }

    #[test]
    fn chain_builder_rejects_nonadjacent_points_within_linear_slop_at_build() {
        let _foundation = crate::Foundation::initialize_default().unwrap();
        assert_eq!(
            ChainDef::builder()
                .points([
                    Vec2::new(0.0, 0.0),
                    Vec2::new(1.0, 0.0),
                    Vec2::new(0.001, 0.0),
                    Vec2::new(2.0, 0.0),
                ])
                .build()
                .unwrap_err(),
            Error::InvalidChainDef
        );
    }
}
