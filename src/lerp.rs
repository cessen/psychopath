#![allow(dead_code)]

/// Trait for allowing a type to be linearly interpolated.
pub trait Lerp
{
    fn lerp(self, other: Self, alpha: f32) -> Self;
}


/// Interpolates between two instances of a Lerp types.
pub fn lerp<T: Lerp>(a: T, b: T, alpha: f32) -> T {
    debug_assert!(alpha >= 0.0);
    debug_assert!(alpha <= 1.0);

    a.lerp(b, alpha)
}


/// Interpolates a slice of data as if each adjecent pair of elements
/// represent a linear segment.
pub fn lerp_slice<T: Lerp + Copy>(s: &[T], alpha: f32) -> T {
    debug_assert!(s.len() > 0);
    debug_assert!(alpha >= 0.0);
    debug_assert!(alpha <= 1.0);

    if alpha == 1.0 || s.len() == 1 {
        return *s.last().unwrap();
    } else {
        let tmp = alpha * ((s.len() - 1) as f32);
        let i1 = tmp as usize;
        let i2 = i1 + 1;
        let alpha2 = tmp - (i1 as f32);

        return lerp(s[i1], s[i2], alpha2);
    }
}


impl Lerp for f32 {
    fn lerp(self, other: f32, alpha: f32) -> f32 {
        (self * (1.0 - alpha)) + (other * alpha)
    }
}

impl Lerp for f64 {
    fn lerp(self, other: f64, alpha: f32) -> f64 {
        (self * (1.0 - alpha as f64)) + (other * alpha as f64)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lerp1() {
        let a = 1.0f32;
        let b = 2.0f32;
        let alpha = 0.0f32;

        assert_eq!(1.0, lerp(a, b, alpha));
    }

    #[test]
    fn lerp2() {
        let a = 1.0f32;
        let b = 2.0f32;
        let alpha = 1.0f32;

        assert_eq!(2.0, lerp(a, b, alpha));
    }

    #[test]
    fn lerp3() {
        let a = 1.0f32;
        let b = 2.0f32;
        let alpha = 0.5f32;

        assert_eq!(1.5, lerp(a, b, alpha));
    }

    #[test]
    fn lerp_slice1() {
        let s = [0.0f32, 1.0, 2.0, 3.0, 4.0];
        let alpha = 0.0f32;

        assert_eq!(0.0, lerp_slice(&s[..], alpha));
    }

    #[test]
    fn lerp_slice2() {
        let s = [0.0f32, 1.0, 2.0, 3.0, 4.0];
        let alpha = 1.0f32;

        assert_eq!(4.0, lerp_slice(&s[..], alpha));
    }

    #[test]
    fn lerp_slice3() {
        let s = [0.0f32, 1.0, 2.0, 3.0, 4.0];
        let alpha = 0.5f32;

        assert_eq!(2.0, lerp_slice(&s[..], alpha));
    }

    #[test]
    fn lerp_slice4() {
        let s = [0.0f32, 1.0, 2.0, 3.0, 4.0];
        let alpha = 0.25f32;

        assert_eq!(1.0, lerp_slice(&s[..], alpha));
    }

    #[test]
    fn lerp_slice5() {
        let s = [0.0f32, 1.0, 2.0, 3.0, 4.0];
        let alpha = 0.75f32;

        assert_eq!(3.0, lerp_slice(&s[..], alpha));
    }

    #[test]
    fn lerp_slice6() {
        let s = [0.0f32, 1.0, 2.0, 3.0, 4.0];
        let alpha = 0.625f32;

        assert_eq!(2.5, lerp_slice(&s[..], alpha));
    }
}
