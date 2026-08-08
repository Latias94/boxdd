//! Opaque world-bound Box2D object identifiers.
//!
//! Public identifiers are process-local storage keys branded to one Rust world registration. They
//! are neither detachable native handles nor persistence authority; every operation must acquire a
//! capability from the owning live world and revalidate the registration before entering native
//! code.

use core::fmt;
use core::num::NonZeroU64;
use core::sync::atomic::{AtomicU64, Ordering};

use boxdd_sys::ffi;

use crate::error::{Error, Result};

static NEXT_WORLD_TOKEN: AtomicU64 = AtomicU64::new(1);

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct WorldToken(NonZeroU64);

impl WorldToken {
    pub(crate) fn allocate() -> Result<Self> {
        Self::allocate_from(&NEXT_WORLD_TOKEN)
    }

    fn allocate_from(next: &AtomicU64) -> Result<Self> {
        let value = next
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1)
            })
            .map_err(|_| Error::WorldIdentityExhausted)?;
        let value = NonZeroU64::new(value).ok_or(Error::WorldIdentityExhausted)?;
        Ok(Self(value))
    }
}

impl fmt::Debug for WorldToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("WorldToken(..)")
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct ContactEpoch(u64);

impl ContactEpoch {
    pub(crate) const INITIAL: Self = Self(0);

    #[inline]
    pub(crate) const fn get(self) -> u64 {
        self.0
    }

    #[inline]
    pub(crate) fn checked_next(self) -> Result<Self> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(Error::ObjectIdentityExhausted)
    }

    #[cfg(test)]
    pub(crate) const fn new_for_test(value: u64) -> Self {
        Self(value)
    }
}

/// Identifies one Rust-side registration of a native object identity.
///
/// Box2D may restore or eventually reuse the same native `(kind, index, generation)` tuple. This
/// nonce prevents a safe identifier from silently changing which Rust registration it denotes.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct RegistrationNonce(NonZeroU64);

impl RegistrationNonce {
    #[inline]
    pub(crate) fn new(value: u64) -> Result<Self> {
        NonZeroU64::new(value)
            .map(Self)
            .ok_or(Error::ObjectIdentityExhausted)
    }
}

impl fmt::Debug for RegistrationNonce {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RegistrationNonce(..)")
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct IdBrand {
    token: WorldToken,
    world0: u16,
    world_generation: u16,
}

impl IdBrand {
    pub(crate) fn new(world: ffi::b2WorldId, token: WorldToken) -> Result<Self> {
        let world0 = world
            .index1
            .checked_sub(1)
            .ok_or(Error::InvalidNativeOutput {
                operation: "IdBrand::new",
                output: "world_id.index1",
                constraint: "a non-zero native world slot",
            })?;
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
    pub(crate) const fn body(
        self,
        raw: ffi::b2BodyId,
        registration_nonce: RegistrationNonce,
    ) -> BodyId {
        BodyId {
            index1: raw.index1,
            generation: raw.generation,
            brand: self,
            registration_nonce,
        }
    }

    #[inline]
    pub(crate) fn check_body_raw(self, raw: ffi::b2BodyId) -> Result<()> {
        if raw.index1 <= 0 {
            Err(Error::InvalidBodyId)
        } else if raw.world0 != self.world0 {
            Err(Error::WrongWorld)
        } else {
            Ok(())
        }
    }

    #[inline]
    pub(crate) const fn shape(
        self,
        raw: ffi::b2ShapeId,
        registration_nonce: RegistrationNonce,
    ) -> ShapeId {
        ShapeId {
            index1: raw.index1,
            generation: raw.generation,
            brand: self,
            registration_nonce,
        }
    }

    #[inline]
    pub(crate) fn check_shape_raw(self, raw: ffi::b2ShapeId) -> Result<()> {
        if raw.index1 <= 0 {
            Err(Error::InvalidShapeId)
        } else if raw.world0 != self.world0 {
            Err(Error::WrongWorld)
        } else {
            Ok(())
        }
    }

    #[inline]
    pub(crate) const fn joint(
        self,
        raw: ffi::b2JointId,
        registration_nonce: RegistrationNonce,
    ) -> JointId {
        JointId {
            index1: raw.index1,
            generation: raw.generation,
            brand: self,
            registration_nonce,
        }
    }

    #[inline]
    pub(crate) fn check_joint_raw(self, raw: ffi::b2JointId) -> Result<()> {
        if raw.index1 <= 0 {
            Err(Error::InvalidJointId)
        } else if raw.world0 != self.world0 {
            Err(Error::WrongWorld)
        } else {
            Ok(())
        }
    }

    #[inline]
    pub(crate) const fn chain(
        self,
        raw: ffi::b2ChainId,
        registration_nonce: RegistrationNonce,
    ) -> ChainId {
        ChainId {
            index1: raw.index1,
            generation: raw.generation,
            brand: self,
            registration_nonce,
        }
    }

    #[inline]
    pub(crate) fn check_chain_raw(self, raw: ffi::b2ChainId) -> Result<()> {
        if raw.index1 <= 0 {
            Err(Error::InvalidChainId)
        } else if raw.world0 != self.world0 {
            Err(Error::WrongWorld)
        } else {
            Ok(())
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
    ) -> Result<ContactId> {
        if raw.index1 <= 0 {
            Err(Error::InvalidContactId)
        } else if raw.world0 != self.world0 {
            Err(Error::WrongWorld)
        } else {
            Ok(self.contact(raw, epoch))
        }
    }
}

macro_rules! branded_object_id {
    ($name:ident, $native:path) => {
        #[doc = "A live Box2D object identifier bound to one Rust world registration."]
        #[doc = ""]
        #[doc = "Its registration nonce keeps the identifier distinct even if Box2D later reuses"]
        #[doc = "the same native index and generation tuple."]
        #[derive(Copy, Clone, PartialEq, Eq, Hash)]
        pub struct $name {
            index1: i32,
            generation: u16,
            brand: IdBrand,
            registration_nonce: RegistrationNonce,
        }

        impl $name {
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

            #[inline]
            pub(crate) const fn registration_nonce(self) -> RegistrationNonce {
                self.registration_nonce
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct(stringify!($name))
                    .field("index1", &self.index1)
                    .field("world0", &self.brand.world0)
                    .field("generation", &self.generation)
                    .field("world_generation", &self.brand.world_generation)
                    .field("registration_nonce", &self.registration_nonce)
                    .finish()
            }
        }
    };
}

branded_object_id!(BodyId, ffi::b2BodyId);
branded_object_id!(ShapeId, ffi::b2ShapeId);
branded_object_id!(JointId, ffi::b2JointId);
branded_object_id!(ChainId, ffi::b2ChainId);

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

    fn test_nonce() -> RegistrationNonce {
        RegistrationNonce::new(1).unwrap()
    }

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
            Err(Error::WorldIdentityExhausted)
        );
        assert_eq!(next.load(Ordering::Relaxed), u64::MAX);
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
            brand.check_body_raw(ffi::b2BodyId {
                index1: 0,
                world0: 0,
                generation: 0,
            }),
            Err(Error::InvalidBodyId)
        );
        assert_eq!(
            brand.check_body_raw(ffi::b2BodyId {
                index1: -1,
                world0: brand.world0(),
                generation: 0,
            }),
            Err(Error::InvalidBodyId)
        );
        assert_eq!(
            brand.check_body_raw(ffi::b2BodyId {
                index1: 1,
                world0: brand.world0() + 1,
                generation: 0,
            }),
            Err(Error::WrongWorld)
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
            brand.check_shape_raw(ffi::b2ShapeId {
                index1: 0,
                world0: 0,
                generation: 0,
            }),
            Err(Error::InvalidShapeId)
        );
        assert_eq!(
            brand.check_shape_raw(ffi::b2ShapeId {
                index1: -1,
                world0: brand.world0(),
                generation: 0,
            }),
            Err(Error::InvalidShapeId)
        );
        assert_eq!(
            brand.check_shape_raw(ffi::b2ShapeId {
                index1: 1,
                world0: brand.world0() + 1,
                generation: 0,
            }),
            Err(Error::WrongWorld)
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
            brand.check_joint_raw(ffi::b2JointId {
                index1: 0,
                world0: 0,
                generation: 0,
            }),
            Err(Error::InvalidJointId)
        );
        assert_eq!(
            brand.check_joint_raw(ffi::b2JointId {
                index1: -1,
                world0: brand.world0(),
                generation: 0,
            }),
            Err(Error::InvalidJointId)
        );
        assert_eq!(
            brand.check_joint_raw(ffi::b2JointId {
                index1: 1,
                world0: brand.world0() + 1,
                generation: 0,
            }),
            Err(Error::WrongWorld)
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
            brand.check_chain_raw(ffi::b2ChainId {
                index1: 0,
                world0: 0,
                generation: 0,
            }),
            Err(Error::InvalidChainId)
        );
        assert_eq!(
            brand.check_chain_raw(ffi::b2ChainId {
                index1: -1,
                world0: brand.world0(),
                generation: 0,
            }),
            Err(Error::InvalidChainId)
        );
        assert_eq!(
            brand.check_chain_raw(ffi::b2ChainId {
                index1: 1,
                world0: brand.world0() + 1,
                generation: 0,
            }),
            Err(Error::WrongWorld)
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
            Err(Error::InvalidContactId)
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
            Err(Error::InvalidContactId)
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
            Err(Error::WrongWorld)
        );
    }

    #[test]
    fn branded_id_retains_its_issuing_world_token() {
        let world = ffi::b2WorldId {
            index1: 4,
            generation: 13,
        };
        let source = IdBrand::new(world, WorldToken::allocate().unwrap()).unwrap();
        let target = IdBrand::new(world, WorldToken::allocate().unwrap()).unwrap();
        let id = source.body(
            ffi::b2BodyId {
                index1: 7,
                world0: 3,
                generation: 11,
            },
            test_nonce(),
        );

        assert_eq!(id.brand(), source);
        assert_ne!(id.brand(), target);
    }

    #[test]
    fn contact_epoch_cannot_wrap() {
        assert_eq!(
            ContactEpoch::new_for_test(u64::MAX).checked_next(),
            Err(Error::ObjectIdentityExhausted)
        );
    }
}
