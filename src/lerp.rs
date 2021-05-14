#![allow(dead_code)]

use math3d::{Matrix4x4, Normal, Point, Vector};

/// Trait for allowing a type to be linearly interpolated.
pub trait Lerp: Copy {
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
pub fn lerp_slice<T: Lerp>(s: &[T], alpha: f32) -> T {
    debug_assert!(!s.is_empty());
    debug_assert!(alpha >= 0.0);
    debug_assert!(alpha <= 1.0);

    if s.len() == 1 || alpha == 1.0 {
        *s.last().unwrap()
    } else {
        let tmp = alpha * ((s.len() - 1) as f32);
        let i1 = tmp as usize;
        let i2 = i1 + 1;
        let alpha2 = tmp - (i1 as f32);

        lerp(s[i1], s[i2], alpha2)
    }
}

pub fn lerp_slice_with<T, F>(s: &[T], alpha: f32, f: F) -> T
where
    T: Copy,
    F: Fn(T, T, f32) -> T,
{
    debug_assert!(!s.is_empty());
    debug_assert!(alpha >= 0.0);
    debug_assert!(alpha <= 1.0);

    if s.len() == 1 || alpha == 1.0 {
        *s.last().unwrap()
    } else {
        let tmp = alpha * ((s.len() - 1) as f32);
        let i1 = tmp as usize;
        let i2 = i1 + 1;
        let alpha2 = tmp - (i1 as f32);

        f(s[i1], s[i2], alpha2)
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

impl<T: Lerp> Lerp for (T, T) {
    fn lerp(self, other: (T, T), alpha: f32) -> (T, T) {
        (self.0.lerp(other.0, alpha), self.1.lerp(other.1, alpha))
    }
}

impl<T: Lerp> Lerp for [T; 2] {
    fn lerp(self, other: Self, alpha: f32) -> Self {
        [self[0].lerp(other[0], alpha), self[1].lerp(other[1], alpha)]
    }
}

impl<T: Lerp> Lerp for [T; 3] {
    fn lerp(self, other: Self, alpha: f32) -> Self {
        [
            self[0].lerp(other[0], alpha),
            self[1].lerp(other[1], alpha),
            self[2].lerp(other[2], alpha),
        ]
    }
}

impl<T: Lerp> Lerp for [T; 4] {
    fn lerp(self, other: Self, alpha: f32) -> Self {
        [
            self[0].lerp(other[0], alpha),
            self[1].lerp(other[1], alpha),
            self[2].lerp(other[2], alpha),
            self[3].lerp(other[3], alpha),
        ]
    }
}

impl Lerp for glam::Vec4 {
    fn lerp(self, other: glam::Vec4, alpha: f32) -> glam::Vec4 {
        (self * (1.0 - alpha)) + (other * alpha)
    }
}

impl Lerp for Matrix4x4 {
    fn lerp(self, other: Matrix4x4, alpha: f32) -> Matrix4x4 {
        (self * (1.0 - alpha)) + (other * alpha)
    }
}

impl Lerp for Normal {
    fn lerp(self, other: Normal, alpha: f32) -> Normal {
        (self * (1.0 - alpha)) + (other * alpha)
    }
}

impl Lerp for Point {
    fn lerp(self, other: Point, alpha: f32) -> Point {
        let s = self;
        let o = other;
        Point {
            co: (s.co * (1.0 - alpha)) + (o.co * alpha),
        }
    }
}

impl Lerp for Vector {
    fn lerp(self, other: Vector, alpha: f32) -> Vector {
        (self * (1.0 - alpha)) + (other * alpha)
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

    #[test]
    fn lerp_matrix() {
        let a = Matrix4x4::new_from_values(
            0.0, 2.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
        );
        let b = Matrix4x4::new_from_values(
            -1.0, 1.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        );

        let c1 = Matrix4x4::new_from_values(
            -0.25, 1.75, 2.25, 3.25, 4.25, 5.25, 6.25, 7.25, 8.25, 9.25, 10.25, 11.25, 12.25,
            13.25, 14.25, 15.25,
        );
        let c2 = Matrix4x4::new_from_values(
            -0.5, 1.5, 2.5, 3.5, 4.5, 5.5, 6.5, 7.5, 8.5, 9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 15.5,
        );
        let c3 = Matrix4x4::new_from_values(
            -0.75, 1.25, 2.75, 3.75, 4.75, 5.75, 6.75, 7.75, 8.75, 9.75, 10.75, 11.75, 12.75,
            13.75, 14.75, 15.75,
        );

        assert_eq!(a.lerp(b, 0.0), a);
        assert_eq!(a.lerp(b, 0.25), c1);
        assert_eq!(a.lerp(b, 0.5), c2);
        assert_eq!(a.lerp(b, 0.75), c3);
        assert_eq!(a.lerp(b, 1.0), b);
    }

    #[test]
    fn lerp_point_1() {
        let p1 = Point::new(1.0, 2.0, 1.0);
        let p2 = Point::new(-2.0, 1.0, -1.0);
        let p3 = Point::new(1.0, 2.0, 1.0);

        assert_eq!(p3, p1.lerp(p2, 0.0));
    }

    #[test]
    fn lerp_point_2() {
        let p1 = Point::new(1.0, 2.0, 1.0);
        let p2 = Point::new(-2.0, 1.0, -1.0);
        let p3 = Point::new(-2.0, 1.0, -1.0);

        assert_eq!(p3, p1.lerp(p2, 1.0));
    }

    #[test]
    fn lerp_point_3() {
        let p1 = Point::new(1.0, 2.0, 1.0);
        let p2 = Point::new(-2.0, 1.0, -1.0);
        let p3 = Point::new(-0.5, 1.5, 0.0);

        assert_eq!(p3, p1.lerp(p2, 0.5));
    }

    #[test]
    fn lerp_normal_1() {
        let n1 = Normal::new(1.0, 2.0, 1.0);
        let n2 = Normal::new(-2.0, 1.0, -1.0);
        let n3 = Normal::new(1.0, 2.0, 1.0);

        assert_eq!(n3, n1.lerp(n2, 0.0));
    }

    #[test]
    fn lerp_normal_2() {
        let n1 = Normal::new(1.0, 2.0, 1.0);
        let n2 = Normal::new(-2.0, 1.0, -1.0);
        let n3 = Normal::new(-2.0, 1.0, -1.0);

        assert_eq!(n3, n1.lerp(n2, 1.0));
    }

    #[test]
    fn lerp_normal_3() {
        let n1 = Normal::new(1.0, 2.0, 1.0);
        let n2 = Normal::new(-2.0, 1.0, -1.0);
        let n3 = Normal::new(-0.5, 1.5, 0.0);

        assert_eq!(n3, n1.lerp(n2, 0.5));
    }

    #[test]
    fn lerp_vector_1() {
        let v1 = Vector::new(1.0, 2.0, 1.0);
        let v2 = Vector::new(-2.0, 1.0, -1.0);
        let v3 = Vector::new(1.0, 2.0, 1.0);

        assert_eq!(v3, v1.lerp(v2, 0.0));
    }

    #[test]
    fn lerp_vector_2() {
        let v1 = Vector::new(1.0, 2.0, 1.0);
        let v2 = Vector::new(-2.0, 1.0, -1.0);
        let v3 = Vector::new(-2.0, 1.0, -1.0);

        assert_eq!(v3, v1.lerp(v2, 1.0));
    }

    #[test]
    fn lerp_vector_3() {
        let v1 = Vector::new(1.0, 2.0, 1.0);
        let v2 = Vector::new(-2.0, 1.0, -1.0);
        let v3 = Vector::new(-0.5, 1.5, 0.0);

        assert_eq!(v3, v1.lerp(v2, 0.5));
    }
}
