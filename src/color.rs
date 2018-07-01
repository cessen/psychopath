use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign};

use spectra_xyz::{spectrum_xyz_to_p_4, EQUAL_ENERGY_REFLECTANCE};

use float4::Float4;
use lerp::Lerp;
use math::fast_exp;

pub use color_util::{rec709_e_to_xyz, rec709_to_xyz, xyz_to_rec709, xyz_to_rec709_e};

// Minimum and maximum wavelength of light we care about, in nanometers
const WL_MIN: f32 = 380.0;
const WL_MAX: f32 = 700.0;
const WL_RANGE: f32 = WL_MAX - WL_MIN;
const WL_RANGE_Q: f32 = WL_RANGE / 4.0;

pub fn map_0_1_to_wavelength(n: f32) -> f32 {
    n * WL_RANGE + WL_MIN
}

pub trait Color {
    fn to_spectral_sample(&self, hero_wavelength: f32) -> SpectralSample;
}

#[inline(always)]
fn nth_wavelength(hero_wavelength: f32, n: usize) -> f32 {
    let wl = hero_wavelength + (WL_RANGE_Q * n as f32);
    if wl > WL_MAX {
        wl - WL_RANGE
    } else {
        wl
    }
}

/// Returns all wavelengths of a hero wavelength set as a Float4
#[inline(always)]
fn wavelengths(hero_wavelength: f32) -> Float4 {
    Float4::new(
        nth_wavelength(hero_wavelength, 0),
        nth_wavelength(hero_wavelength, 1),
        nth_wavelength(hero_wavelength, 2),
        nth_wavelength(hero_wavelength, 3),
    )
}

//----------------------------------------------------------------

#[derive(Copy, Clone, Debug)]
pub struct SpectralSample {
    pub e: Float4,
    hero_wavelength: f32,
}

impl SpectralSample {
    pub fn new(wavelength: f32) -> SpectralSample {
        debug_assert!(wavelength >= WL_MIN && wavelength <= WL_MAX);
        SpectralSample {
            e: Float4::splat(0.0),
            hero_wavelength: wavelength,
        }
    }

    #[allow(dead_code)]
    pub fn from_value(value: f32, wavelength: f32) -> SpectralSample {
        debug_assert!(wavelength >= WL_MIN && wavelength <= WL_MAX);
        SpectralSample {
            e: Float4::splat(value),
            hero_wavelength: wavelength,
        }
    }

    pub fn from_parts(e: Float4, wavelength: f32) -> SpectralSample {
        debug_assert!(wavelength >= WL_MIN && wavelength <= WL_MAX);
        SpectralSample {
            e: e,
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

impl Add for SpectralSample {
    type Output = SpectralSample;
    fn add(self, rhs: SpectralSample) -> Self::Output {
        assert_eq!(self.hero_wavelength, rhs.hero_wavelength);
        SpectralSample {
            e: self.e + rhs.e,
            hero_wavelength: self.hero_wavelength,
        }
    }
}

impl AddAssign for SpectralSample {
    fn add_assign(&mut self, rhs: SpectralSample) {
        assert_eq!(self.hero_wavelength, rhs.hero_wavelength);
        self.e = self.e + rhs.e;
    }
}

impl Mul for SpectralSample {
    type Output = SpectralSample;
    fn mul(self, rhs: SpectralSample) -> Self::Output {
        assert_eq!(self.hero_wavelength, rhs.hero_wavelength);
        SpectralSample {
            e: self.e * rhs.e,
            hero_wavelength: self.hero_wavelength,
        }
    }
}

impl MulAssign for SpectralSample {
    fn mul_assign(&mut self, rhs: SpectralSample) {
        assert_eq!(self.hero_wavelength, rhs.hero_wavelength);
        self.e = self.e * rhs.e;
    }
}

impl Mul<f32> for SpectralSample {
    type Output = SpectralSample;
    fn mul(self, rhs: f32) -> Self::Output {
        SpectralSample {
            e: self.e * rhs,
            hero_wavelength: self.hero_wavelength,
        }
    }
}

impl MulAssign<f32> for SpectralSample {
    fn mul_assign(&mut self, rhs: f32) {
        self.e = self.e * rhs;
    }
}

impl Div<f32> for SpectralSample {
    type Output = SpectralSample;
    fn div(self, rhs: f32) -> Self::Output {
        SpectralSample {
            e: self.e / rhs,
            hero_wavelength: self.hero_wavelength,
        }
    }
}

impl DivAssign<f32> for SpectralSample {
    fn div_assign(&mut self, rhs: f32) {
        self.e = self.e / rhs;
    }
}

//----------------------------------------------------------------

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
        let xyz0 = XYZ::from_wavelength(ss.wl_n(0), ss.e.get_0());
        let xyz1 = XYZ::from_wavelength(ss.wl_n(1), ss.e.get_1());
        let xyz2 = XYZ::from_wavelength(ss.wl_n(2), ss.e.get_2());
        let xyz3 = XYZ::from_wavelength(ss.wl_n(3), ss.e.get_3());
        (xyz0 + xyz1 + xyz2 + xyz3) * 0.75
    }

    pub fn to_tuple(&self) -> (f32, f32, f32) {
        (self.x, self.y, self.z)
    }
}

impl Color for XYZ {
    fn to_spectral_sample(&self, hero_wavelength: f32) -> SpectralSample {
        SpectralSample {
            e: xyz_to_spectrum_4((self.x, self.y, self.z), wavelengths(hero_wavelength)),
            hero_wavelength: hero_wavelength,
        }
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

//----------------------------------------------------------------

/// Samples an CIE 1931 XYZ color at a particular set of wavelengths, according to
/// the method in the paper "Physically Meaningful Rendering using Tristimulus
/// Colours" by Meng et al.
#[inline(always)]
fn xyz_to_spectrum_4(xyz: (f32, f32, f32), wavelengths: Float4) -> Float4 {
    spectrum_xyz_to_p_4(wavelengths, xyz) * Float4::splat(1.0 / EQUAL_ENERGY_REFLECTANCE)
}

/// Close analytic approximations of the CIE 1931 XYZ color curves.
/// From the paper "Simple Analytic Approximations to the CIE XYZ Color Matching
/// Functions" by Wyman et al.
pub fn x_1931(wavelength: f32) -> f32 {
    let t1 = (wavelength - 442.0) * (if wavelength < 442.0 { 0.0624 } else { 0.0374 });
    let t2 = (wavelength - 599.8) * (if wavelength < 599.8 { 0.0264 } else { 0.0323 });
    let t3 = (wavelength - 501.1) * (if wavelength < 501.1 { 0.0490 } else { 0.0382 });
    (0.362 * fast_exp(-0.5 * t1 * t1)) + (1.056 * fast_exp(-0.5 * t2 * t2))
        - (0.065 * fast_exp(-0.5 * t3 * t3))
}

pub fn y_1931(wavelength: f32) -> f32 {
    let t1 = (wavelength - 568.8) * (if wavelength < 568.8 { 0.0213 } else { 0.0247 });
    let t2 = (wavelength - 530.9) * (if wavelength < 530.9 { 0.0613 } else { 0.0322 });
    (0.821 * fast_exp(-0.5 * t1 * t1)) + (0.286 * fast_exp(-0.5 * t2 * t2))
}

pub fn z_1931(wavelength: f32) -> f32 {
    let t1 = (wavelength - 437.0) * (if wavelength < 437.0 { 0.0845 } else { 0.0278 });
    let t2 = (wavelength - 459.0) * (if wavelength < 459.0 { 0.0385 } else { 0.0725 });
    (1.217 * fast_exp(-0.5 * t1 * t1)) + (0.681 * fast_exp(-0.5 * t2 * t2))
}
