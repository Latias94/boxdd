use crate::error::{Error, Result};

#[cfg(feature = "double-precision")]
const B2_HUGE_FACTOR: f32 = 1.0e9;
#[cfg(not(feature = "double-precision"))]
const B2_HUGE_FACTOR: f32 = 1.0e5;
const B2_LINEAR_SLOP_FACTOR: f32 = 0.005;
const B2_DEFAULT_MAX_LINEAR_SPEED_FACTOR: f32 = 400.0;
const B2_AIR_DENSITY_FACTOR: f32 = 1.225;

/// Return whether `length_units_per_meter` keeps the constants used by Safe Box2D calls valid.
///
/// Safe ray casts always submit `maxFraction = 1`, TOI and shape casts require a positive finite
/// linear slop, solver speed clamps square the default maximum speed, and wind forces derive air
/// density from the cube of this value. Keep these checks in f32 so they match the native
/// calculations in both precision modes.
#[inline]
pub(crate) fn is_safe_length_units_per_meter(length_units_per_meter: f32) -> bool {
    if !length_units_per_meter.is_finite() || length_units_per_meter <= 0.0 {
        return false;
    }

    let huge = B2_HUGE_FACTOR * length_units_per_meter;
    let linear_slop = B2_LINEAR_SLOP_FACTOR * length_units_per_meter;
    let linear_slop_squared = linear_slop * linear_slop;
    let maximum_linear_speed = B2_DEFAULT_MAX_LINEAR_SPEED_FACTOR * length_units_per_meter;
    let maximum_linear_speed_squared = maximum_linear_speed * maximum_linear_speed;
    let volume_units = length_units_per_meter * length_units_per_meter * length_units_per_meter;
    let air_density = B2_AIR_DENSITY_FACTOR / volume_units;

    huge.is_finite()
        && huge > 1.0
        && linear_slop.is_finite()
        && linear_slop > 0.0
        && linear_slop_squared.is_finite()
        && linear_slop_squared > 0.0
        && maximum_linear_speed.is_finite()
        && maximum_linear_speed > 0.0
        && maximum_linear_speed_squared.is_finite()
        && maximum_linear_speed_squared > 0.0
        && volume_units.is_finite()
        && volume_units > 0.0
        && air_density.is_finite()
        && air_density > 0.0
}

/// Immutable provenance for defaults derived from Box2D's process-global length scale.
///
/// Keeping the IEEE-754 bits avoids silently treating nearby scales as interchangeable. Box2D
/// installs this setting globally, so a definition built for one scale must never be submitted to
/// a world created for another.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LengthScale {
    bits: u32,
}

impl LengthScale {
    #[inline]
    pub(crate) fn try_new(length_units_per_meter: f32) -> Option<Self> {
        if is_safe_length_units_per_meter(length_units_per_meter) {
            Some(Self {
                bits: length_units_per_meter.to_bits(),
            })
        } else {
            None
        }
    }

    #[inline]
    pub(crate) const fn units_per_meter(self) -> f32 {
        f32::from_bits(self.bits)
    }

    #[inline]
    pub(crate) const fn bits(self) -> u32 {
        self.bits
    }

    #[inline]
    pub(crate) fn check_definition(self, operation: &'static str, definition: Self) -> Result<()> {
        if self == definition {
            Ok(())
        } else {
            Err(Error::LengthScaleMismatch {
                operation,
                expected: self.bits(),
                actual: definition.bits(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjacent_float_scales_are_not_interchangeable() {
        let expected = LengthScale::try_new(1.0).unwrap();
        let adjacent = LengthScale::try_new(f32::from_bits(1.0_f32.to_bits() + 1)).unwrap();

        assert_eq!(
            expected.check_definition("test", adjacent),
            Err(Error::LengthScaleMismatch {
                operation: "test",
                expected: expected.bits(),
                actual: adjacent.bits(),
            })
        );
    }

    #[test]
    fn invalid_global_scales_cannot_be_represented() {
        for value in [0.0, -1.0, f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
            assert_eq!(LengthScale::try_new(value), None);
        }
    }

    #[test]
    fn scales_that_break_native_derived_calculations_cannot_be_represented() {
        let below_ray_limit = 0.5 / B2_HUGE_FACTOR;
        // This keeps the linear tolerances finite, but the native wind volume overflows.
        let above_volume_limit = 1.0e13_f32;
        // This still keeps B2_HUGE finite, but B2_LINEAR_SLOP squared overflows.
        let above_squared_slop_limit = 1.0e22_f32;
        for value in [
            f32::from_bits(1),
            below_ray_limit,
            above_volume_limit,
            above_squared_slop_limit,
            f32::MAX,
        ] {
            assert_eq!(LengthScale::try_new(value), None);
        }
        assert!(LengthScale::try_new(1.0).is_some());
    }
}
