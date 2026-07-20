//! World-bound and unbound Box2D object identifiers.
//!
//! Live identifiers are capabilities bound to one Rust world instance. Raw identifiers are
//! serializable value surrogates and must be validated by a target world before use.

use core::fmt;
use core::hash::Hash;
use core::num::NonZeroU64;
use core::sync::atomic::{AtomicU64, Ordering};
use std::collections::hash_map::RandomState;
use std::hash::BuildHasher;
use std::sync::OnceLock;

use boxdd_sys::ffi;

use crate::error::{ApiError, ApiResult};

const RAW_ID_VERSION: u8 = 2;
const RAW_ID_AUTH_DOMAIN: &str = "boxdd.raw-id.process-local.v2";
static NEXT_WORLD_TOKEN: AtomicU64 = AtomicU64::new(1);
static RAW_ID_AUTH_STATE: OnceLock<RandomState> = OnceLock::new();

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct WorldToken(NonZeroU64);

impl WorldToken {
    pub(crate) fn allocate() -> ApiResult<Self> {
        Self::allocate_from(&NEXT_WORLD_TOKEN)
    }

    fn allocate_from(next: &AtomicU64) -> ApiResult<Self> {
        let value = next
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1)
            })
            .map_err(|_| ApiError::WorldIdentityExhausted)?;
        let value = NonZeroU64::new(value).ok_or(ApiError::WorldIdentityExhausted)?;
        Ok(Self(value))
    }
}

impl fmt::Debug for WorldToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("WorldToken(..)")
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct ContactEpoch(u64);

impl ContactEpoch {
    pub(crate) const INITIAL: Self = Self(0);

    #[inline]
    pub(crate) const fn get(self) -> u64 {
        self.0
    }

    #[inline]
    pub(crate) fn checked_next(self) -> ApiResult<Self> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(ApiError::ObjectIdentityExhausted)
    }

    #[cfg(test)]
    pub(crate) const fn new_for_test(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Hash)]
struct RawIdAuthPayload<G> {
    domain: &'static str,
    version: u8,
    kind: RawIdKind,
    index1: i32,
    world0: u16,
    object_generation: G,
    world_generation: u16,
    token: WorldToken,
    contact_epoch: Option<ContactEpoch>,
}

#[allow(clippy::too_many_arguments)]
fn raw_id_auth<G: Hash>(
    version: u8,
    kind: RawIdKind,
    index1: i32,
    world0: u16,
    object_generation: G,
    world_generation: u16,
    token: WorldToken,
    contact_epoch: Option<ContactEpoch>,
) -> u64 {
    RAW_ID_AUTH_STATE
        .get_or_init(RandomState::new)
        .hash_one(RawIdAuthPayload {
            domain: RAW_ID_AUTH_DOMAIN,
            version,
            kind,
            index1,
            world0,
            object_generation,
            world_generation,
            token,
            contact_epoch,
        })
}

/// The object family encoded in an unbound identifier.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RawIdKind {
    Body,
    Shape,
    Joint,
    Chain,
    Contact,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct IdBrand {
    token: WorldToken,
    world0: u16,
    world_generation: u16,
}

impl IdBrand {
    pub(crate) fn new(world: ffi::b2WorldId, token: WorldToken) -> ApiResult<Self> {
        let world0 = world
            .index1
            .checked_sub(1)
            .ok_or(ApiError::InvalidArgument)?;
        Ok(Self {
            token,
            world0,
            world_generation: world.generation,
        })
    }

    #[inline]
    pub(crate) const fn token(self) -> WorldToken {
        self.token
    }

    #[inline]
    pub(crate) const fn world0(self) -> u16 {
        self.world0
    }

    #[inline]
    pub(crate) const fn world_generation(self) -> u16 {
        self.world_generation
    }

    #[inline]
    pub(crate) const fn body(self, raw: ffi::b2BodyId) -> BodyId {
        BodyId::bind(raw, self)
    }

    #[inline]
    pub(crate) fn try_body(self, raw: ffi::b2BodyId) -> ApiResult<BodyId> {
        if raw.index1 <= 0 {
            Err(ApiError::InvalidBodyId)
        } else if raw.world0 != self.world0 {
            Err(ApiError::WrongWorld)
        } else {
            Ok(self.body(raw))
        }
    }

    #[inline]
    pub(crate) const fn shape(self, raw: ffi::b2ShapeId) -> ShapeId {
        ShapeId::bind(raw, self)
    }

    #[inline]
    pub(crate) fn try_shape(self, raw: ffi::b2ShapeId) -> ApiResult<ShapeId> {
        if raw.index1 <= 0 {
            Err(ApiError::InvalidShapeId)
        } else if raw.world0 != self.world0 {
            Err(ApiError::WrongWorld)
        } else {
            Ok(self.shape(raw))
        }
    }

    #[inline]
    pub(crate) const fn joint(self, raw: ffi::b2JointId) -> JointId {
        JointId::bind(raw, self)
    }

    #[inline]
    pub(crate) fn try_joint(self, raw: ffi::b2JointId) -> ApiResult<JointId> {
        if raw.index1 <= 0 {
            Err(ApiError::InvalidJointId)
        } else if raw.world0 != self.world0 {
            Err(ApiError::WrongWorld)
        } else {
            Ok(self.joint(raw))
        }
    }

    #[inline]
    pub(crate) const fn chain(self, raw: ffi::b2ChainId) -> ChainId {
        ChainId::bind(raw, self)
    }

    #[inline]
    pub(crate) fn try_chain(self, raw: ffi::b2ChainId) -> ApiResult<ChainId> {
        if raw.index1 <= 0 {
            Err(ApiError::InvalidChainId)
        } else if raw.world0 != self.world0 {
            Err(ApiError::WrongWorld)
        } else {
            Ok(self.chain(raw))
        }
    }

    #[inline]
    pub(crate) const fn contact(self, raw: ffi::b2ContactId, epoch: ContactEpoch) -> ContactId {
        ContactId::bind(raw, self, epoch)
    }

    #[inline]
    pub(crate) fn try_contact(
        self,
        raw: ffi::b2ContactId,
        epoch: ContactEpoch,
    ) -> ApiResult<ContactId> {
        if raw.index1 <= 0 {
            Err(ApiError::InvalidContactId)
        } else if raw.world0 != self.world0 {
            Err(ApiError::WrongWorld)
        } else {
            Ok(self.contact(raw, epoch))
        }
    }
}

macro_rules! raw_object_id {
    ($name:ident, $native:path, $kind:ident) => {
        #[doc = "An authenticated, process-local Box2D object identifier."]
        #[doc = ""]
        #[doc = "Only a live identifier's `unbind` method issues trusted values. Serde input is"]
        #[doc = "untrusted and is authenticated when bound to its original live `World`. The"]
        #[doc = "representation is deliberately not portable across process boundaries."]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
        pub struct $name {
            version: u8,
            kind: RawIdKind,
            pub index1: i32,
            pub world0: u16,
            pub generation: u16,
            pub world_generation: u16,
            token: WorldToken,
            auth: u64,
        }

        impl $name {
            #[inline]
            fn issue(raw: $native, brand: IdBrand) -> Self {
                let auth = raw_id_auth(
                    RAW_ID_VERSION,
                    RawIdKind::$kind,
                    raw.index1,
                    raw.world0,
                    raw.generation,
                    brand.world_generation,
                    brand.token,
                    None,
                );
                Self {
                    version: RAW_ID_VERSION,
                    kind: RawIdKind::$kind,
                    index1: raw.index1,
                    world0: raw.world0,
                    generation: raw.generation,
                    world_generation: brand.world_generation,
                    token: brand.token,
                    auth,
                }
            }

            #[inline]
            pub const fn into_ffi(self) -> $native {
                $native {
                    index1: self.index1,
                    world0: self.world0,
                    generation: self.generation,
                }
            }

            #[inline]
            pub(crate) fn validate_for(self, brand: IdBrand) -> ApiResult<()> {
                let expected_auth = raw_id_auth(
                    self.version,
                    self.kind,
                    self.index1,
                    self.world0,
                    self.generation,
                    self.world_generation,
                    self.token,
                    None,
                );
                if self.auth != expected_auth || self.version != RAW_ID_VERSION {
                    Err(ApiError::InvalidRawId)
                } else if !matches!(self.kind, RawIdKind::$kind) {
                    Err(ApiError::WrongIdKind)
                } else if self.token != brand.token
                    || self.world0 != brand.world0
                    || self.world_generation != brand.world_generation
                {
                    Err(ApiError::WrongWorld)
                } else {
                    Ok(())
                }
            }
        }
    };
}

raw_object_id!(RawBodyId, ffi::b2BodyId, Body);
raw_object_id!(RawShapeId, ffi::b2ShapeId, Shape);
raw_object_id!(RawJointId, ffi::b2JointId, Joint);
raw_object_id!(RawChainId, ffi::b2ChainId, Chain);

/// An authenticated, process-local Box2D contact identifier.
///
/// Only [`ContactId::unbind`] issues trusted values. Serde input is untrusted and is authenticated
/// when bound to its original live [`crate::World`] and contact epoch. The representation is not
/// portable across process boundaries. Native padding is deliberately not represented.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RawContactId {
    version: u8,
    kind: RawIdKind,
    pub index1: i32,
    pub world0: u16,
    pub generation: u32,
    pub world_generation: u16,
    token: WorldToken,
    contact_epoch: ContactEpoch,
    auth: u64,
}

impl RawContactId {
    #[inline]
    fn issue(raw: ffi::b2ContactId, brand: IdBrand, contact_epoch: ContactEpoch) -> Self {
        let auth = raw_id_auth(
            RAW_ID_VERSION,
            RawIdKind::Contact,
            raw.index1,
            raw.world0,
            raw.generation,
            brand.world_generation,
            brand.token,
            Some(contact_epoch),
        );
        Self {
            version: RAW_ID_VERSION,
            kind: RawIdKind::Contact,
            index1: raw.index1,
            world0: raw.world0,
            generation: raw.generation,
            world_generation: brand.world_generation,
            token: brand.token,
            contact_epoch,
            auth,
        }
    }

    #[inline]
    pub const fn into_ffi(self) -> ffi::b2ContactId {
        ffi::b2ContactId {
            index1: self.index1,
            world0: self.world0,
            padding: 0,
            generation: self.generation,
        }
    }

    #[inline]
    pub(crate) fn validate_for(self, brand: IdBrand, contact_epoch: ContactEpoch) -> ApiResult<()> {
        let expected_auth = raw_id_auth(
            self.version,
            self.kind,
            self.index1,
            self.world0,
            self.generation,
            self.world_generation,
            self.token,
            Some(self.contact_epoch),
        );
        if self.auth != expected_auth || self.version != RAW_ID_VERSION {
            Err(ApiError::InvalidRawId)
        } else if !matches!(self.kind, RawIdKind::Contact) {
            Err(ApiError::WrongIdKind)
        } else if self.token != brand.token
            || self.world0 != brand.world0
            || self.world_generation != brand.world_generation
        {
            Err(ApiError::WrongWorld)
        } else if self.contact_epoch != contact_epoch {
            Err(ApiError::InvalidContactId)
        } else {
            Ok(())
        }
    }
}

macro_rules! branded_object_id {
    ($name:ident, $raw_name:ident, $native:path) => {
        #[doc = "A live Box2D object identifier bound to one Rust world instance."]
        #[derive(Copy, Clone, PartialEq, Eq, Hash)]
        pub struct $name {
            index1: i32,
            generation: u16,
            brand: IdBrand,
        }

        impl $name {
            #[inline]
            const fn bind(raw: $native, brand: IdBrand) -> Self {
                Self {
                    index1: raw.index1,
                    generation: raw.generation,
                    brand,
                }
            }

            #[inline]
            pub fn unbind(self) -> $raw_name {
                $raw_name::issue(self.into_raw(), self.brand)
            }

            #[inline]
            pub(crate) const fn into_raw(self) -> $native {
                $native {
                    index1: self.index1,
                    world0: self.brand.world0,
                    generation: self.generation,
                }
            }

            #[inline]
            pub(crate) const fn brand(self) -> IdBrand {
                self.brand
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct(stringify!($name))
                    .field("index1", &self.index1)
                    .field("world0", &self.brand.world0)
                    .field("generation", &self.generation)
                    .field("world_generation", &self.brand.world_generation)
                    .finish()
            }
        }
    };
}

branded_object_id!(BodyId, RawBodyId, ffi::b2BodyId);
branded_object_id!(ShapeId, RawShapeId, ffi::b2ShapeId);
branded_object_id!(JointId, RawJointId, ffi::b2JointId);
branded_object_id!(ChainId, RawChainId, ffi::b2ChainId);

/// A live Box2D contact identifier bound to one Rust world and simulation-step epoch.
///
/// A contact id is valid only until the next world step begins. Bind or inspect it before stepping
/// the world again; stale contact ids are rejected without calling Box2D.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct ContactId {
    index1: i32,
    generation: u32,
    brand: IdBrand,
    contact_epoch: ContactEpoch,
}

impl ContactId {
    #[inline]
    const fn bind(raw: ffi::b2ContactId, brand: IdBrand, contact_epoch: ContactEpoch) -> Self {
        Self {
            index1: raw.index1,
            generation: raw.generation,
            brand,
            contact_epoch,
        }
    }

    #[inline]
    pub fn unbind(self) -> RawContactId {
        RawContactId::issue(self.into_raw(), self.brand, self.contact_epoch)
    }

    #[inline]
    pub(crate) const fn into_raw(self) -> ffi::b2ContactId {
        ffi::b2ContactId {
            index1: self.index1,
            world0: self.brand.world0,
            padding: 0,
            generation: self.generation,
        }
    }

    #[inline]
    pub(crate) const fn brand(self) -> IdBrand {
        self.brand
    }

    #[inline]
    pub(crate) const fn contact_epoch(self) -> ContactEpoch {
        self.contact_epoch
    }
}

impl fmt::Debug for ContactId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ContactId")
            .field("index1", &self.index1)
            .field("world0", &self.brand.world0)
            .field("generation", &self.generation)
            .field("world_generation", &self.brand.world_generation)
            .field("contact_epoch", &self.contact_epoch.get())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_tokens_are_monotonic_and_nonzero() {
        let first = WorldToken::allocate().unwrap();
        let second = WorldToken::allocate().unwrap();
        assert!(second.0.get() > first.0.get());
    }

    #[test]
    fn world_token_exhaustion_is_reported_without_wrapping() {
        let next = AtomicU64::new(u64::MAX);

        assert_eq!(
            WorldToken::allocate_from(&next),
            Err(ApiError::WorldIdentityExhausted)
        );
        assert_eq!(next.load(Ordering::Relaxed), u64::MAX);
    }

    #[test]
    fn unbound_ids_preserve_native_and_world_generations() {
        let token = WorldToken::allocate().unwrap();
        let brand = IdBrand::new(
            ffi::b2WorldId {
                index1: 4,
                generation: 13,
            },
            token,
        )
        .unwrap();
        let id = brand.body(ffi::b2BodyId {
            index1: 7,
            world0: 3,
            generation: 11,
        });

        let raw = id.unbind();
        assert_eq!(raw.index1, 7);
        assert_eq!(raw.world0, 3);
        assert_eq!(raw.generation, 11);
        assert_eq!(raw.world_generation, 13);
        assert_eq!(raw.validate_for(brand), Ok(()));
    }

    #[test]
    fn contact_identity_ignores_native_padding() {
        let token = WorldToken::allocate().unwrap();
        let brand = IdBrand::new(
            ffi::b2WorldId {
                index1: 2,
                generation: 5,
            },
            token,
        )
        .unwrap();
        let epoch = ContactEpoch::new_for_test(7);
        let first = brand.contact(
            ffi::b2ContactId {
                index1: 3,
                world0: 1,
                padding: -1,
                generation: 8,
            },
            epoch,
        );
        let second = brand.contact(
            ffi::b2ContactId {
                index1: 3,
                world0: 1,
                padding: 42,
                generation: 8,
            },
            epoch,
        );

        assert_eq!(first, second);
        assert_eq!(first.into_raw().padding, 0);
    }

    #[test]
    fn body_output_binding_rejects_null_negative_and_foreign_ids() {
        let token = WorldToken::allocate().unwrap();
        let brand = IdBrand::new(
            ffi::b2WorldId {
                index1: 4,
                generation: 13,
            },
            token,
        )
        .unwrap();

        assert_eq!(
            brand.try_body(ffi::b2BodyId {
                index1: 0,
                world0: 0,
                generation: 0,
            }),
            Err(ApiError::InvalidBodyId)
        );
        assert_eq!(
            brand.try_body(ffi::b2BodyId {
                index1: -1,
                world0: brand.world0(),
                generation: 0,
            }),
            Err(ApiError::InvalidBodyId)
        );
        assert_eq!(
            brand.try_body(ffi::b2BodyId {
                index1: 1,
                world0: brand.world0() + 1,
                generation: 0,
            }),
            Err(ApiError::WrongWorld)
        );
    }

    #[test]
    fn shape_output_binding_rejects_null_negative_and_foreign_ids() {
        let token = WorldToken::allocate().unwrap();
        let brand = IdBrand::new(
            ffi::b2WorldId {
                index1: 4,
                generation: 13,
            },
            token,
        )
        .unwrap();

        assert_eq!(
            brand.try_shape(ffi::b2ShapeId {
                index1: 0,
                world0: 0,
                generation: 0,
            }),
            Err(ApiError::InvalidShapeId)
        );
        assert_eq!(
            brand.try_shape(ffi::b2ShapeId {
                index1: -1,
                world0: brand.world0(),
                generation: 0,
            }),
            Err(ApiError::InvalidShapeId)
        );
        assert_eq!(
            brand.try_shape(ffi::b2ShapeId {
                index1: 1,
                world0: brand.world0() + 1,
                generation: 0,
            }),
            Err(ApiError::WrongWorld)
        );
    }

    #[test]
    fn joint_output_binding_rejects_null_negative_and_foreign_ids() {
        let token = WorldToken::allocate().unwrap();
        let brand = IdBrand::new(
            ffi::b2WorldId {
                index1: 4,
                generation: 13,
            },
            token,
        )
        .unwrap();

        assert_eq!(
            brand.try_joint(ffi::b2JointId {
                index1: 0,
                world0: 0,
                generation: 0,
            }),
            Err(ApiError::InvalidJointId)
        );
        assert_eq!(
            brand.try_joint(ffi::b2JointId {
                index1: -1,
                world0: brand.world0(),
                generation: 0,
            }),
            Err(ApiError::InvalidJointId)
        );
        assert_eq!(
            brand.try_joint(ffi::b2JointId {
                index1: 1,
                world0: brand.world0() + 1,
                generation: 0,
            }),
            Err(ApiError::WrongWorld)
        );
    }

    #[test]
    fn chain_output_binding_rejects_null_negative_and_foreign_ids() {
        let token = WorldToken::allocate().unwrap();
        let brand = IdBrand::new(
            ffi::b2WorldId {
                index1: 4,
                generation: 13,
            },
            token,
        )
        .unwrap();

        assert_eq!(
            brand.try_chain(ffi::b2ChainId {
                index1: 0,
                world0: 0,
                generation: 0,
            }),
            Err(ApiError::InvalidChainId)
        );
        assert_eq!(
            brand.try_chain(ffi::b2ChainId {
                index1: -1,
                world0: brand.world0(),
                generation: 0,
            }),
            Err(ApiError::InvalidChainId)
        );
        assert_eq!(
            brand.try_chain(ffi::b2ChainId {
                index1: 1,
                world0: brand.world0() + 1,
                generation: 0,
            }),
            Err(ApiError::WrongWorld)
        );
    }

    #[test]
    fn contact_output_binding_rejects_null_negative_and_foreign_ids() {
        let token = WorldToken::allocate().unwrap();
        let brand = IdBrand::new(
            ffi::b2WorldId {
                index1: 4,
                generation: 13,
            },
            token,
        )
        .unwrap();
        let epoch = ContactEpoch::new_for_test(7);

        assert_eq!(
            brand.try_contact(
                ffi::b2ContactId {
                    index1: 0,
                    world0: 0,
                    padding: 0,
                    generation: 0,
                },
                epoch
            ),
            Err(ApiError::InvalidContactId)
        );
        assert_eq!(
            brand.try_contact(
                ffi::b2ContactId {
                    index1: -1,
                    world0: brand.world0(),
                    padding: 0,
                    generation: 0,
                },
                epoch
            ),
            Err(ApiError::InvalidContactId)
        );
        assert_eq!(
            brand.try_contact(
                ffi::b2ContactId {
                    index1: 1,
                    world0: brand.world0() + 1,
                    padding: 0,
                    generation: 0,
                },
                epoch
            ),
            Err(ApiError::WrongWorld)
        );
    }

    #[test]
    fn raw_id_authentication_covers_every_identity_field() {
        let brand = IdBrand::new(
            ffi::b2WorldId {
                index1: 4,
                generation: 13,
            },
            WorldToken::allocate().unwrap(),
        )
        .unwrap();
        let raw = brand
            .body(ffi::b2BodyId {
                index1: 7,
                world0: 3,
                generation: 11,
            })
            .unbind();

        let mut candidates = [raw; 8];
        candidates[0].version = raw.version.wrapping_add(1);
        candidates[1].kind = RawIdKind::Shape;
        candidates[2].index1 = raw.index1.wrapping_add(1);
        candidates[3].world0 = raw.world0.wrapping_add(1);
        candidates[4].generation = raw.generation.wrapping_add(1);
        candidates[5].world_generation = raw.world_generation.wrapping_add(1);
        candidates[6].token = WorldToken::allocate().unwrap();
        candidates[7].auth ^= 1;

        for candidate in candidates {
            assert_eq!(candidate.validate_for(brand), Err(ApiError::InvalidRawId));
        }
    }

    #[test]
    fn authentic_raw_id_is_bound_to_its_issuing_world_token() {
        let world = ffi::b2WorldId {
            index1: 4,
            generation: 13,
        };
        let source = IdBrand::new(world, WorldToken::allocate().unwrap()).unwrap();
        let target = IdBrand::new(world, WorldToken::allocate().unwrap()).unwrap();
        let raw = source
            .body(ffi::b2BodyId {
                index1: 7,
                world0: 3,
                generation: 11,
            })
            .unbind();

        assert_eq!(raw.validate_for(source), Ok(()));
        assert_eq!(raw.validate_for(target), Err(ApiError::WrongWorld));
    }

    #[test]
    fn contact_epoch_is_checked_and_cannot_wrap() {
        let brand = IdBrand::new(
            ffi::b2WorldId {
                index1: 2,
                generation: 5,
            },
            WorldToken::allocate().unwrap(),
        )
        .unwrap();
        let epoch = ContactEpoch::new_for_test(41);
        let next = epoch.checked_next().unwrap();
        let contact = brand.contact(
            ffi::b2ContactId {
                index1: 3,
                world0: 1,
                padding: 0,
                generation: 8,
            },
            epoch,
        );
        let raw = contact.unbind();

        assert_eq!(raw.validate_for(brand, epoch), Ok(()));
        assert_eq!(
            raw.validate_for(brand, next),
            Err(ApiError::InvalidContactId)
        );
        assert_eq!(
            ContactEpoch::new_for_test(u64::MAX).checked_next(),
            Err(ApiError::ObjectIdentityExhausted)
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_field_tampering_invalidates_raw_id_authentication() {
        let brand = IdBrand::new(
            ffi::b2WorldId {
                index1: 4,
                generation: 13,
            },
            WorldToken::allocate().unwrap(),
        )
        .unwrap();
        let raw = brand
            .body(ffi::b2BodyId {
                index1: 7,
                world0: 3,
                generation: 11,
            })
            .unbind();
        let encoded = serde_json::to_value(raw).unwrap();

        for (field, replacement) in [
            ("version", serde_json::json!(raw.version.wrapping_add(1))),
            ("kind", serde_json::json!("Shape")),
            ("index1", serde_json::json!(raw.index1.wrapping_add(1))),
            ("world0", serde_json::json!(raw.world0.wrapping_add(1))),
            (
                "generation",
                serde_json::json!(raw.generation.wrapping_add(1)),
            ),
            (
                "world_generation",
                serde_json::json!(raw.world_generation.wrapping_add(1)),
            ),
            (
                "token",
                serde_json::json!(raw.token.0.get().wrapping_add(1)),
            ),
            ("auth", serde_json::json!(raw.auth ^ 1)),
        ] {
            let mut tampered = encoded.clone();
            tampered[field] = replacement;
            let candidate: RawBodyId = serde_json::from_value(tampered).unwrap();
            assert_eq!(candidate.validate_for(brand), Err(ApiError::InvalidRawId));
        }

        let authentic_shape = brand
            .shape(ffi::b2ShapeId {
                index1: 9,
                world0: 3,
                generation: 12,
            })
            .unbind();
        let authentic_wrong_kind: RawBodyId =
            serde_json::from_value(serde_json::to_value(authentic_shape).unwrap()).unwrap();
        assert_eq!(
            authentic_wrong_kind.validate_for(brand),
            Err(ApiError::WrongIdKind)
        );
    }
}
