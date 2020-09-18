//! Functions for storing triplets of floating point values in a
//! shared-exponent format.
//!
//! The motivating use-case for this is compactly storing HDR RGB colors.  But
//! it may be useful for other things as well.

pub mod signed48;
pub mod unsigned32;

//===========================================================================
// Some shared functions used by the other modules in this crate.

/// Calculates 2.0^exp using IEEE bit fiddling.
///
/// Only works for integer exponents in the range [-126, 127]
/// due to IEEE 32-bit float limits.
#[inline(always)]
fn fiddle_exp2(exp: i32) -> f32 {
    use std::f32;
    f32::from_bits(((exp + 127) as u32) << 23)
}

/// Calculates a floor(log2(n)) using IEEE bit fiddling.
///
/// Because of IEEE floating point format, infinity and NaN
/// floating point values return 128, and subnormal numbers always
/// return -127.  These particular behaviors are not, of course,
/// mathemetically correct, but are actually desireable for the
/// calculations in this library.
#[inline(always)]
fn fiddle_log2(n: f32) -> i32 {
    use std::f32;
    ((f32::to_bits(n) >> 23) & 0b1111_1111) as i32 - 127
}
