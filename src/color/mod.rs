mod spectra_xyz;

use std::ops::{Add, AddAssign, Mul, MulAssign, Div, DivAssign};

use lerp::Lerp;
use self::spectra_xyz::{spectrum_xyz_to_p, EQUAL_ENERGY_REFLECTANCE};

// Minimum and maximum wavelength of light we care about, in nanometers
const WL_MIN: f32 = 380.0;
const WL_MAX: f32 = 700.0;
const WL_RANGE: f32 = WL_MAX - WL_MIN;
const WL_RANGE_Q: f32 = WL_RANGE / 4.0;

pub fn map_0_1_to_wavelength(n: f32) -> f32 {
    n * WL_RANGE + WL_MIN
}

pub trait Color {
    fn sample_spectrum(&self, wavelength: f32) -> f32;

    fn to_spectral_sample(&self, hero_wavelength: f32) -> SpectralSample {
        SpectralSample {
            e: [self.sample_spectrum(nth_wavelength(hero_wavelength, 0)),
                self.sample_spectrum(nth_wavelength(hero_wavelength, 1)),
                self.sample_spectrum(nth_wavelength(hero_wavelength, 2)),
                self.sample_spectrum(nth_wavelength(hero_wavelength, 3))],

            hero_wavelength: hero_wavelength,
        }
    }
}

fn nth_wavelength(hero_wavelength: f32, n: usize) -> f32 {
    let wl = hero_wavelength + (WL_RANGE_Q * n as f32);
    if wl > WL_MAX {
        wl - WL_RANGE
    } else {
        wl
    }
}


#[derive(Copy, Clone, Debug)]
pub struct SpectralSample {
    e: [f32; 4],
    hero_wavelength: f32,
}

impl SpectralSample {
    pub fn new(wavelength: f32) -> SpectralSample {
        debug_assert!(wavelength >= WL_MIN && wavelength <= WL_MAX);
        SpectralSample {
            e: [0.0; 4],
            hero_wavelength: wavelength,
        }
    }

    /// Returns the nth wavelength
    fn wl_n(&self, n: usize) -> f32 {
        let wl = self.hero_wavelength + (WL_RANGE_Q * n as f32);
        if wl > WL_MAX {
            wl - WL_RANGE
        } else {
            wl
        }
    }
}


#[derive(Copy, Clone, Debug)]
pub struct XYZ {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl XYZ {
    pub fn new(x: f32, y: f32, z: f32) -> XYZ {
        XYZ { x: x, y: y, z: z }
    }

    pub fn from_tuple(xyz: (f32, f32, f32)) -> XYZ {
        XYZ {
            x: xyz.0,
            y: xyz.1,
            z: xyz.2,
        }
    }

    pub fn from_wavelength(wavelength: f32, intensity: f32) -> XYZ {
        XYZ {
            x: x_1931(wavelength) * intensity,
            y: y_1931(wavelength) * intensity,
            z: z_1931(wavelength) * intensity,
        }
    }

    pub fn from_spectral_sample(ss: &SpectralSample) -> XYZ {
        let xyz0 = XYZ::from_wavelength(ss.wl_n(0), ss.e[0]);
        let xyz1 = XYZ::from_wavelength(ss.wl_n(1), ss.e[1]);
        let xyz2 = XYZ::from_wavelength(ss.wl_n(2), ss.e[2]);
        let xyz3 = XYZ::from_wavelength(ss.wl_n(3), ss.e[3]);
        (xyz0 + xyz1 + xyz2 + xyz3) * 0.75
    }

    pub fn to_tuple(&self) -> (f32, f32, f32) {
        (self.x, self.y, self.z)
    }
}

impl Color for XYZ {
    fn sample_spectrum(&self, wavelength: f32) -> f32 {
        xyz_to_spectrum((self.x, self.y, self.z), wavelength)
    }
}

impl Lerp for XYZ {
    fn lerp(self, other: XYZ, alpha: f32) -> XYZ {
        (self * (1.0 - alpha)) + (other * alpha)
    }
}

impl Add for XYZ {
    type Output = XYZ;
    fn add(self, rhs: XYZ) -> Self::Output {
        XYZ {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl AddAssign for XYZ {
    fn add_assign(&mut self, rhs: XYZ) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}

impl Mul<f32> for XYZ {
    type Output = XYZ;
    fn mul(self, rhs: f32) -> Self::Output {
        XYZ {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}

impl MulAssign<f32> for XYZ {
    fn mul_assign(&mut self, rhs: f32) {
        self.x *= rhs;
        self.y *= rhs;
        self.z *= rhs;
    }
}

impl Div<f32> for XYZ {
    type Output = XYZ;
    fn div(self, rhs: f32) -> Self::Output {
        XYZ {
            x: self.x / rhs,
            y: self.y / rhs,
            z: self.z / rhs,
        }
    }
}

impl DivAssign<f32> for XYZ {
    fn div_assign(&mut self, rhs: f32) {
        self.x /= rhs;
        self.y /= rhs;
        self.z /= rhs;
    }
}

/// Converts a color in XYZ colorspace to Rec.709 colorspace.
/// Note: this can result in negative values, because the positive Rec.709
/// colorspace cannot represent all colors in the XYZ colorspace.
#[allow(dead_code)]
pub fn xyz_to_rec709(xyz: (f32, f32, f32)) -> (f32, f32, f32) {
    ((xyz.0 * 3.2404542) + (xyz.1 * -1.5371385) + (xyz.2 * -0.4985314),
     (xyz.0 * -0.9692660) + (xyz.1 * 1.8760108) + (xyz.2 * 0.0415560),
     (xyz.0 * 0.0556434) + (xyz.1 * -0.2040259) + (xyz.2 * 1.0572252))
}

/// Converts a color in Rec.709 colorspace to XYZ colorspace.
#[allow(dead_code)]
pub fn rec709_to_xyz(rec: (f32, f32, f32)) -> (f32, f32, f32) {
    ((rec.0 * 0.4124564) + (rec.1 * 0.3575761) + (rec.2 * 0.1804375),
     (rec.0 * 0.2126729) + (rec.1 * 0.7151522) + (rec.2 * 0.0721750),
     (rec.0 * 0.0193339) + (rec.1 * 0.1191920) + (rec.2 * 0.9503041))
}

/// Converts a color in XYZ colorspace to an adjusted Rec.709 colorspace
/// with whitepoint E.
/// Note: this is lossy, as negative resulting values are clamped to zero.
#[allow(dead_code)]
pub fn xyz_to_rec709e(xyz: (f32, f32, f32)) -> (f32, f32, f32) {
    ((xyz.0 * 3.0799600) + (xyz.1 * -1.5371400) + (xyz.2 * -0.5428160),
     (xyz.0 * -0.9212590) + (xyz.1 * 1.8760100) + (xyz.2 * 0.0452475),
     (xyz.0 * 0.0528874) + (xyz.1 * -0.2040260) + (xyz.2 * 1.1511400))
}

/// Converts a color in an adjusted Rec.709 colorspace with whitepoint E to
/// XYZ colorspace.
#[allow(dead_code)]
pub fn rec709e_to_xyz(rec: (f32, f32, f32)) -> (f32, f32, f32) {
    ((rec.0 * 0.4339499) + (rec.1 * 0.3762098) + (rec.2 * 0.1898403),
     (rec.0 * 0.2126729) + (rec.1 * 0.7151522) + (rec.2 * 0.0721750),
     (rec.0 * 0.0177566) + (rec.1 * 0.1094680) + (rec.2 * 0.8727755))
}


/// Samples an CIE 1931 XYZ color at a particular wavelength, according to
/// the method in the paper "Physically Meaningful Rendering using Tristimulus
/// Colours" by Meng et al.
fn xyz_to_spectrum(xyz: (f32, f32, f32), wavelength: f32) -> f32 {
    spectrum_xyz_to_p(wavelength, xyz) * (1.0 / EQUAL_ENERGY_REFLECTANCE)
}


/// Close analytic approximations of the CIE 1931 XYZ color curves.
/// From the paper "Simple Analytic Approximations to the CIE XYZ Color Matching
/// Functions" by Wyman et al.
#[allow(dead_code)]
fn x_1931(wavelength: f32) -> f32 {
    let t1 = (wavelength - 442.0) *
             (if wavelength < 442.0 {
        0.0624
    } else {
        0.0374
    });
    let t2 = (wavelength - 599.8) *
             (if wavelength < 599.8 {
        0.0264
    } else {
        0.0323
    });
    let t3 = (wavelength - 501.1) *
             (if wavelength < 501.1 {
        0.0490
    } else {
        0.0382
    });
    (0.362 * (-0.5 * t1 * t1).exp()) + (1.056 * (-0.5 * t2 * t2).exp()) -
    (0.065 * (-0.5 * t3 * t3).exp())
}

#[allow(dead_code)]
fn y_1931(wavelength: f32) -> f32 {
    let t1 = (wavelength - 568.8) *
             (if wavelength < 568.8 {
        0.0213
    } else {
        0.0247
    });
    let t2 = (wavelength - 530.9) *
             (if wavelength < 530.9 {
        0.0613
    } else {
        0.0322
    });
    (0.821 * (-0.5 * t1 * t1).exp()) + (0.286 * (-0.5 * t2 * t2).exp())
}

#[allow(dead_code)]
fn z_1931(wavelength: f32) -> f32 {
    let t1 = (wavelength - 437.0) *
             (if wavelength < 437.0 {
        0.0845
    } else {
        0.0278
    });
    let t2 = (wavelength - 459.0) *
             (if wavelength < 459.0 {
        0.0385
    } else {
        0.0725
    });
    (1.217 * (-0.5 * t1 * t1).exp()) + (0.681 * (-0.5 * t2 * t2).exp())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn abs_diff_tri(a: (f32, f32, f32), b: (f32, f32, f32)) -> (f32, f32, f32) {
        ((a.0 - b.0).abs(), (a.1 - b.1).abs(), (a.2 - b.2).abs())
    }

    #[test]
    fn rec709_xyz_01() {
        let c1 = (1.0, 1.0, 1.0);
        let c2 = rec709_to_xyz(c1);
        let c3 = xyz_to_rec709(c2);

        let diff = abs_diff_tri(c1, c3);

        assert!(diff.0 < 0.00001);
        assert!(diff.1 < 0.00001);
        assert!(diff.2 < 0.00001);
    }

    #[test]
    fn rec709_xyz_02() {
        let c1 = (1.0, 1.0, 1.0);
        let c2 = xyz_to_rec709(c1);
        let c3 = rec709_to_xyz(c2);

        let diff = abs_diff_tri(c1, c3);

        assert!(diff.0 < 0.00001);
        assert!(diff.1 < 0.00001);
        assert!(diff.2 < 0.00001);
    }

    #[test]
    fn rec709_xyz_03() {
        let c1 = (0.9, 0.05, 0.8);
        let c2 = rec709_to_xyz(c1);
        let c3 = xyz_to_rec709(c2);

        let diff = abs_diff_tri(c1, c3);

        assert!(diff.0 < 0.00001);
        assert!(diff.1 < 0.00001);
        assert!(diff.2 < 0.00001);
    }

    #[test]
    fn rec709_xyz_04() {
        let c1 = (0.9, 0.05, 0.8);
        let c2 = xyz_to_rec709(c1);
        let c3 = rec709_to_xyz(c2);

        let diff = abs_diff_tri(c1, c3);

        assert!(diff.0 < 0.00001);
        assert!(diff.1 < 0.00001);
        assert!(diff.2 < 0.00001);
    }

    #[test]
    fn rec709e_xyz_01() {
        let c1 = (1.0, 1.0, 1.0);
        let c2 = rec709e_to_xyz(c1);
        let c3 = xyz_to_rec709e(c2);

        let diff = abs_diff_tri(c1, c3);

        assert!(diff.0 < 0.00001);
        assert!(diff.1 < 0.00001);
        assert!(diff.2 < 0.00001);
    }

    #[test]
    fn rec709e_xyz_02() {
        let c1 = (1.0, 1.0, 1.0);
        let c2 = xyz_to_rec709e(c1);
        let c3 = rec709e_to_xyz(c2);

        let diff = abs_diff_tri(c1, c3);

        assert!(diff.0 < 0.00001);
        assert!(diff.1 < 0.00001);
        assert!(diff.2 < 0.00001);
    }

    #[test]
    fn rec709e_xyz_03() {
        let c1 = (1.0, 1.0, 1.0);
        let c2 = rec709e_to_xyz(c1);

        let diff = abs_diff_tri(c1, c2);

        assert!(diff.0 < 0.00001);
        assert!(diff.1 < 0.00001);
        assert!(diff.2 < 0.00001);
    }

    #[test]
    fn rec709e_xyz_04() {
        let c1 = (1.0, 1.0, 1.0);
        let c2 = xyz_to_rec709e(c1);

        let diff = abs_diff_tri(c1, c2);

        assert!(diff.0 < 0.00001);
        assert!(diff.1 < 0.00001);
        assert!(diff.2 < 0.00001);
    }

    #[test]
    fn rec709e_xyz_05() {
        let c1 = (0.9, 0.05, 0.8);
        let c2 = rec709e_to_xyz(c1);
        let c3 = xyz_to_rec709e(c2);

        let diff = abs_diff_tri(c1, c3);

        assert!(diff.0 < 0.00001);
        assert!(diff.1 < 0.00001);
        assert!(diff.2 < 0.00001);
    }

    #[test]
    fn rec709e_xyz_06() {
        let c1 = (0.9, 0.05, 0.8);
        let c2 = xyz_to_rec709e(c1);
        let c3 = rec709e_to_xyz(c2);

        let diff = abs_diff_tri(c1, c3);

        assert!(diff.0 < 0.00001);
        assert!(diff.1 < 0.00001);
        assert!(diff.2 < 0.00001);
    }
}
