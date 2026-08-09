use crate::core::identity_registry::OutputIdentityResolver;
use crate::core::world_core::WorldCore;
use crate::error::Result;
use crate::id::ContactEpoch;
use crate::types::{ContactId, Position, ShapeId, Vec2};
use boxdd_sys::ffi;

#[derive(Clone, Debug)]
pub struct ContactBeginTouchEvent {
    pub shape_a: ShapeId,
    pub shape_b: ShapeId,
    pub contact_id: ContactId,
}

impl ContactBeginTouchEvent {
    fn from_raw(
        identities: &OutputIdentityResolver<'_>,
        contact_epoch: ContactEpoch,
        raw: ffi::b2ContactBeginTouchEvent,
    ) -> Result<Self> {
        Ok(Self {
            shape_a: identities.shape(raw.shapeIdA)?,
            shape_b: identities.shape(raw.shapeIdB)?,
            contact_id: identities.contact(raw.contactId, contact_epoch)?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct ContactEndTouchEvent {
    pub shape_a: ShapeId,
    pub shape_b: ShapeId,
    pub contact_id: ContactId,
}

impl ContactEndTouchEvent {
    fn from_raw(
        identities: &OutputIdentityResolver<'_>,
        contact_epoch: ContactEpoch,
        raw: ffi::b2ContactEndTouchEvent,
    ) -> Result<Self> {
        Ok(Self {
            shape_a: identities.shape(raw.shapeIdA)?,
            shape_b: identities.shape(raw.shapeIdB)?,
            contact_id: identities.contact(raw.contactId, contact_epoch)?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct ContactHitEvent {
    pub shape_a: ShapeId,
    pub shape_b: ShapeId,
    pub contact_id: ContactId,
    pub point: Position,
    pub normal: Vec2,
    pub approach_speed: f32,
}

impl ContactHitEvent {
    fn from_raw(
        identities: &OutputIdentityResolver<'_>,
        contact_epoch: ContactEpoch,
        raw: ffi::b2ContactHitEvent,
    ) -> Result<Self> {
        let (point, normal, approach_speed) = validate_native_hit_geometry(raw)?;
        Ok(Self {
            shape_a: identities.shape(raw.shapeIdA)?,
            shape_b: identities.shape(raw.shapeIdB)?,
            contact_id: identities.contact(raw.contactId, contact_epoch)?,
            point,
            normal,
            approach_speed,
        })
    }
}

fn validate_native_hit_geometry(raw: ffi::b2ContactHitEvent) -> Result<(Position, Vec2, f32)> {
    const OPERATION: &str = "CompletedStep::contact_events";
    let point = Position::from_raw(raw.point);
    if !point.is_valid() {
        return Err(crate::Error::InvalidNativeOutput {
            operation: OPERATION,
            output: "hit.point",
            constraint: "a finite world position",
        });
    }

    let normal = Vec2::from_raw(raw.normal);
    let normal_length_squared = normal.x * normal.x + normal.y * normal.y;
    if !normal.is_valid()
        || !normal_length_squared.is_finite()
        || (1.0 - normal_length_squared).abs() >= 100.0 * f32::EPSILON
    {
        return Err(crate::Error::InvalidNativeOutput {
            operation: OPERATION,
            output: "hit.normal",
            constraint: "a finite unit vector",
        });
    }

    if !raw.approachSpeed.is_finite() || raw.approachSpeed <= 0.0 {
        return Err(crate::Error::InvalidNativeOutput {
            operation: OPERATION,
            output: "hit.approach_speed",
            constraint: "a finite positive value",
        });
    }

    Ok((point, normal, raw.approachSpeed))
}

#[derive(Clone, Debug, Default)]
pub struct ContactEvents {
    pub begin: Vec<ContactBeginTouchEvent>,
    pub end: Vec<ContactEndTouchEvent>,
    pub hit: Vec<ContactHitEvent>,
}

/// A borrowed view of contact events from one completed step.
pub struct ContactEventsView<'step> {
    events: &'step ContactEvents,
}

impl<'step> ContactEventsView<'step> {
    pub(super) fn new(events: &'step ContactEvents) -> Self {
        Self { events }
    }

    pub fn begin(&self) -> &[ContactBeginTouchEvent] {
        &self.events.begin
    }

    pub fn end(&self) -> &[ContactEndTouchEvent] {
        &self.events.end
    }

    pub fn hit(&self) -> &[ContactHitEvent] {
        &self.events.hit
    }

    pub fn is_empty(&self) -> bool {
        self.events.begin.is_empty() && self.events.end.is_empty() && self.events.hit.is_empty()
    }

    pub fn to_owned(&self) -> Result<ContactEvents> {
        let mut out = ContactEvents::default();
        self.clone_into(&mut out)?;
        Ok(out)
    }

    pub fn clone_into(&self, out: &mut ContactEvents) -> Result<()> {
        let result = (|| {
            super::clone_into(&self.events.begin, &mut out.begin)?;
            super::clone_into(&self.events.end, &mut out.end)?;
            super::clone_into(&self.events.hit, &mut out.hit)
        })();
        if result.is_err() {
            out.begin.clear();
            out.end.clear();
            out.hit.clear();
        }
        result
    }
}

pub(super) fn capture(
    out: &mut ContactEvents,
    raw: ffi::b2ContactEvents,
    core: &WorldCore,
    contact_epoch: ContactEpoch,
) -> Result<()> {
    // SAFETY: The completed-step capability prevents mutation while these slices are mapped.
    let begin = unsafe { super::ffi_slice(raw.beginEvents, raw.beginCount) }?;
    // SAFETY: Same completed-step lifetime as `begin`.
    let end = unsafe { super::ffi_slice(raw.endEvents, raw.endCount) }?;
    // SAFETY: Same completed-step lifetime as `begin`.
    let hit = unsafe { super::ffi_slice(raw.hitEvents, raw.hitCount) }?;

    if begin.is_empty() && end.is_empty() && hit.is_empty() {
        out.begin.clear();
        out.end.clear();
        out.hit.clear();
        return Ok(());
    }

    let result = (|| {
        super::prepare_mapped(&mut out.begin, begin.len())?;
        super::prepare_mapped(&mut out.end, end.len())?;
        super::prepare_mapped(&mut out.hit, hit.len())?;
        core.with_output_identity_resolver(|identities| {
            super::extend_mapped(&mut out.begin, begin, |event| {
                ContactBeginTouchEvent::from_raw(identities, contact_epoch, *event)
            })?;
            super::extend_mapped(&mut out.end, end, |event| {
                ContactEndTouchEvent::from_raw(identities, contact_epoch, *event)
            })?;
            super::extend_mapped(&mut out.hit, hit, |event| {
                ContactHitEvent::from_raw(identities, contact_epoch, *event)
            })
        })
    })();
    if result.is_err() {
        out.begin.clear();
        out.end.clear();
        out.hit.clear();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_hit() -> ffi::b2ContactHitEvent {
        ffi::b2ContactHitEvent {
            shapeIdA: ffi::b2ShapeId {
                index1: 0,
                world0: 0,
                generation: 0,
            },
            shapeIdB: ffi::b2ShapeId {
                index1: 0,
                world0: 0,
                generation: 0,
            },
            contactId: ffi::b2ContactId {
                index1: 0,
                world0: 0,
                padding: 0,
                generation: 0,
            },
            point: Position::ZERO.into_raw(),
            normal: ffi::b2Vec2 { x: 1.0, y: 0.0 },
            approachSpeed: 1.0,
        }
    }

    #[test]
    fn native_hit_geometry_fails_closed() {
        assert!(validate_native_hit_geometry(valid_hit()).is_ok());

        let mut invalid_point = valid_hit();
        invalid_point.point.x = crate::WorldScalar::NAN;
        assert!(matches!(
            validate_native_hit_geometry(invalid_point),
            Err(crate::Error::InvalidNativeOutput {
                output: "hit.point",
                ..
            })
        ));

        let mut invalid_normal = valid_hit();
        invalid_normal.normal.x = 2.0;
        assert!(matches!(
            validate_native_hit_geometry(invalid_normal),
            Err(crate::Error::InvalidNativeOutput {
                output: "hit.normal",
                ..
            })
        ));

        let mut invalid_speed = valid_hit();
        invalid_speed.approachSpeed = 0.0;
        assert!(matches!(
            validate_native_hit_geometry(invalid_speed),
            Err(crate::Error::InvalidNativeOutput {
                output: "hit.approach_speed",
                ..
            })
        ));
    }
}
