use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign};

pub use color::{
    rec709_e_to_xyz, rec709_to_xyz, xyz_to_aces_ap0, xyz_to_aces_ap0_e, xyz_to_rec709,
    xyz_to_rec709_e,
};
use glam::Vec4;
use half::f16;
use spectral_upsampling::meng::{spectrum_xyz_to_p_4, EQUAL_ENERGY_REFLECTANCE};
use trifloat::signed48;

use crate::{lerp::Lerp, math::fast_exp};

// Minimum and maximum wavelength of light we care about, in nanometers
const WL_MIN: f32 = 380.0;
const WL_MAX: f32 = 700.0;
const WL_RANGE: f32 = WL_MAX - WL_MIN;
const WL_RANGE_Q: f32 = WL_RANGE / 4.0;

pub fn map_0_1_to_wavelength(n: f32) -> f32 {
    n * WL_RANGE + WL_MIN
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

/// Returns all wavelengths of a hero wavelength set as a Vec4
#[inline(always)]
fn wavelengths(hero_wavelength: f32) -> Vec4 {
    Vec4::new(
        nth_wavelength(hero_wavelength, 0),
        nth_wavelength(hero_wavelength, 1),
        nth_wavelength(hero_wavelength, 2),
        nth_wavelength(hero_wavelength, 3),
    )
}

//----------------------------------------------------------------

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Color {
    XYZ(f32, f32, f32),
    Blackbody {
        temperature: f32, // In kelvin
        factor: f32,      // Brightness multiplier
    },
    // Same as Blackbody except with the spectrum's energy roughly
    // normalized.
    Temperature {
        temperature: f32, // In kelvin
        factor: f32,      // Brightness multiplier
    },
}

impl Color {
    #[inline(always)]
    pub fn new_xyz(xyz: (f32, f32, f32)) -> Self {
        Color::XYZ(xyz.0, xyz.1, xyz.2)
    }

    #[inline(always)]
    pub fn new_blackbody(temp: f32, fac: f32) -> Self {
        Color::Blackbody {
            temperature: temp,
            factor: fac,
        }
    }

    #[inline(always)]
    pub fn new_temperature(temp: f32, fac: f32) -> Self {
        Color::Temperature {
            temperature: temp,
            factor: fac,
        }
    }

    pub fn to_spectral_sample(self, hero_wavelength: f32) -> SpectralSample {
        let wls = wavelengths(hero_wavelength);
        match self {
            Color::XYZ(x, y, z) => SpectralSample {
                e: xyz_to_spectrum_4((x, y, z), wls),
                hero_wavelength: hero_wavelength,
            },
            Color::Blackbody {
                temperature,
                factor,
            } => {
                SpectralSample::from_parts(
                    // TODO: make this SIMD
                    Vec4::new(
                        plancks_law(temperature, wls.x()) * factor,
                        plancks_law(temperature, wls.y()) * factor,
                        plancks_law(temperature, wls.z()) * factor,
                        plancks_law(temperature, wls.w()) * factor,
                    ),
                    hero_wavelength,
                )
            }
            Color::Temperature {
                temperature,
                factor,
            } => {
                SpectralSample::from_parts(
                    // TODO: make this SIMD
                    Vec4::new(
                        plancks_law_normalized(temperature, wls.x()) * factor,
                        plancks_law_normalized(temperature, wls.y()) * factor,
                        plancks_law_normalized(temperature, wls.z()) * factor,
                        plancks_law_normalized(temperature, wls.w()) * factor,
                    ),
                    hero_wavelength,
                )
            }
        }
    }

    /// Calculates an approximate total spectral energy of the color.
    ///
    /// Note: this really is very _approximate_.
    pub fn approximate_energy(self) -> f32 {
        // TODO: better approximation for Blackbody and Temperature.
        match self {
            Color::XYZ(_, y, _) => y,

            Color::Blackbody {
                temperature,
                factor,
            } => {
                let t2 = temperature * temperature;
                t2 * t2 * factor
            }

            Color::Temperature { factor, .. } => factor,
        }
    }

    /// Returns the post-compression size of this color.
    pub fn compressed_size(&self) -> usize {
        match self {
            Color::XYZ(_, _, _) => 7,

            Color::Blackbody { .. } => 5,

            Color::Temperature { .. } => 5,
        }
    }

    /// Writes the compressed form of this color to `out_data`.
    ///
    /// `out_data` must be at least `compressed_size()` bytes long, otherwise
    /// this method will panic.
    ///
    /// Returns the number of bytes written.
    pub fn write_compressed(&self, out_data: &mut [u8]) -> usize {
        match *self {
            Color::XYZ(x, y, z) => {
                out_data[0] = 0; // Discriminant
                let col = signed48::encode((x, y, z));
                let col = col.to_le_bytes();
                (&mut out_data[1..7]).copy_from_slice(&col[0..6]);
            }

            Color::Blackbody {
                temperature,
                factor,
            } => {
                out_data[0] = 1; // Discriminant
                let tmp = (temperature.min(std::u16::MAX as f32) as u16).to_le_bytes();
                let fac = f16::from_f32(factor).to_bits().to_le_bytes();
                out_data[1] = tmp[0];
                out_data[2] = tmp[1];
                out_data[3] = fac[0];
                out_data[4] = fac[1];
            }

            Color::Temperature {
                temperature,
                factor,
            } => {
                out_data[0] = 2; // Discriminant
                let tmp = (temperature.min(std::u16::MAX as f32) as u16).to_le_bytes();
                let fac = f16::from_f32(factor).to_bits().to_le_bytes();
                out_data[1] = tmp[0];
                out_data[2] = tmp[1];
                out_data[3] = fac[0];
                out_data[4] = fac[1];
            }
        }
        self.compressed_size()
    }

    /// Constructs a Color from compressed color data, and also returns the
    /// number of bytes consumed from `in_data`.
    pub fn from_compressed(in_data: &[u8]) -> (Color, usize) {
        match in_data[0] {
            0 => {
                // XYZ
                let mut bytes = [0u8; 8];
                (&mut bytes[0..6]).copy_from_slice(&in_data[1..7]);
                let (x, y, z) = signed48::decode(u64::from_le_bytes(bytes));
                (Color::XYZ(x, y, z), 7)
            }

            1 => {
                // Blackbody
                let mut tmp = [0u8; 2];
                let mut fac = [0u8; 2];
                tmp[0] = in_data[1];
                tmp[1] = in_data[2];
                fac[0] = in_data[3];
                fac[1] = in_data[4];
                let tmp = u16::from_le_bytes(tmp);
                let fac = f16::from_bits(u16::from_le_bytes(fac));
                (
                    Color::Blackbody {
                        temperature: tmp as f32,
                        factor: fac.into(),
                    },
                    5,
                )
            }

            2 => {
                // Temperature
                let mut tmp = [0u8; 2];
                let mut fac = [0u8; 2];
                tmp[0] = in_data[1];
                tmp[1] = in_data[2];
                fac[0] = in_data[3];
                fac[1] = in_data[4];
                let tmp = u16::from_le_bytes(tmp);
                let fac = f16::from_bits(u16::from_le_bytes(fac));
                (
                    Color::Temperature {
                        temperature: tmp as f32,
                        factor: fac.into(),
                    },
                    5,
                )
            }

            _ => unreachable!(),
        }
    }
}

impl Mul<f32> for Color {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self {
        match self {
            Color::XYZ(x, y, z) => Color::XYZ(x * rhs, y * rhs, z * rhs),

            Color::Blackbody {
                temperature,
                factor,
            } => Color::Blackbody {
                temperature: temperature,
                factor: factor * rhs,
            },

            Color::Temperature {
                temperature,
                factor,
            } => Color::Temperature {
                temperature: temperature,
                factor: factor * rhs,
            },
        }
    }
}

impl MulAssign<f32> for Color {
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs;
    }
}

impl Lerp for Color {
    /// Note that this isn't a proper lerp in spectral space.  However,
    /// for our purposes that should be fine: all we care about is that
    /// the interpolation is smooth and "reasonable".
    ///
    /// If at some point it turns out this causes artifacts, then we
    /// also have bigger problems: texture filtering in the shading
    /// pipeline will have the same issues, which will be even harder
    /// to address.  However, I strongly suspect this will not be an issue.
    /// (Famous last words!)
    fn lerp(self, other: Self, alpha: f32) -> Self {
        let inv_alpha = 1.0 - alpha;
        match (self, other) {
            (Color::XYZ(x1, y1, z1), Color::XYZ(x2, y2, z2)) => Color::XYZ(
                (x1 * inv_alpha) + (x2 * alpha),
                (y1 * inv_alpha) + (y2 * alpha),
                (z1 * inv_alpha) + (z2 * alpha),
            ),

            (
                Color::Blackbody {
                    temperature: tmp1,
                    factor: fac1,
                },
                Color::Blackbody {
                    temperature: tmp2,
                    factor: fac2,
                },
            ) => Color::Blackbody {
                temperature: (tmp1 * inv_alpha) + (tmp2 * alpha),
                factor: (fac1 * inv_alpha) + (fac2 * alpha),
            },

            (
                Color::Temperature {
                    temperature: tmp1,
                    factor: fac1,
                },
                Color::Temperature {
                    temperature: tmp2,
                    factor: fac2,
                },
            ) => Color::Temperature {
                temperature: (tmp1 * inv_alpha) + (tmp2 * alpha),
                factor: (fac1 * inv_alpha) + (fac2 * alpha),
            },

            _ => panic!("Cannot lerp colors with different representations."),
        }
    }
}

fn plancks_law(temperature: f32, wavelength: f32) -> f32 {
    const C: f32 = 299_792_458.0; // Speed of light
    const H: f32 = 6.626_070_15e-34; // Planck constant
    const KB: f32 = 1.380_648_52e-23; // Boltzmann constant

    // At 400 kelvin and below, the spectrum is black anyway,
    // but the equations become numerically unstable somewhere
    // around 100 kelvin.  So just return zero energy below 200.
    // (Technically there is still a tiny amount of energy that
    // we're losing this way, but it's incredibly tiny, with tons
    // of zeros after the decimal point--way less energy than would
    // ever, ever, ever even have the slightest chance of showing
    // impacting a render.)
    if temperature < 200.0 {
        return 0.0;
    }

    // Convert the wavelength from nanometers to meters for
    // the equations below.
    let wavelength = wavelength * 1.0e-9;

    // // As written at https://en.wikipedia.org/wiki/Planck's_law, here for
    // // reference and clarity:
    // let a = (2.0 * H * C * C) / (wavelength * wavelength * wavelength * wavelength * wavelength);
    // let b = 1.0 / (((H * C) / (wavelength * KB * temperature)).exp() - 1.0);
    // let energy = a * b;

    // Optimized version of the commented code above:
    const TMP1: f32 = (2.0f64 * H as f64 * C as f64 * C as f64) as f32;
    const TMP2: f32 = (H as f64 * C as f64 / KB as f64) as f32;
    let wl5 = {
        let wl2 = wavelength * wavelength;
        wl2 * wl2 * wavelength
    };
    let tmp3 = wl5 * (fast_exp(TMP2 / (wavelength * temperature)) - 1.0);
    let energy = TMP1 / tmp3;

    // Convert energy to appropriate units and return.
    (energy * 1.0e-6).max(0.0)
}

/// Same as above, except normalized to keep roughly equal spectral
/// energy across temperatures.  This makes it easier to use for
/// choosing colors without making brightness explode.
fn plancks_law_normalized(temperature: f32, wavelength: f32) -> f32 {
    let t2 = temperature * temperature;
    plancks_law(temperature, wavelength) * 4.0e7 / (t2 * t2)
}

//----------------------------------------------------------------

#[derive(Copy, Clone, Debug)]
pub struct SpectralSample {
    pub e: Vec4,
    hero_wavelength: f32,
}

impl SpectralSample {
    pub fn new(wavelength: f32) -> SpectralSample {
        debug_assert!(wavelength >= WL_MIN && wavelength <= WL_MAX);
        SpectralSample {
            e: Vec4::splat(0.0),
            hero_wavelength: wavelength,
        }
    }

    #[allow(dead_code)]
    pub fn from_value(value: f32, wavelength: f32) -> SpectralSample {
        debug_assert!(wavelength >= WL_MIN && wavelength <= WL_MAX);
        SpectralSample {
            e: Vec4::splat(value),
            hero_wavelength: wavelength,
        }
    }

    pub fn from_parts(e: Vec4, wavelength: f32) -> SpectralSample {
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

    pub fn from_wavelength(wavelength: f32, intensity: f32) -> XYZ {
        XYZ {
            x: x_1931(wavelength) * intensity,
            y: y_1931(wavelength) * intensity,
            z: z_1931(wavelength) * intensity,
        }
    }

    pub fn from_spectral_sample(ss: &SpectralSample) -> XYZ {
        let xyz0 = XYZ::from_wavelength(ss.wl_n(0), ss.e.x());
        let xyz1 = XYZ::from_wavelength(ss.wl_n(1), ss.e.y());
        let xyz2 = XYZ::from_wavelength(ss.wl_n(2), ss.e.z());
        let xyz3 = XYZ::from_wavelength(ss.wl_n(3), ss.e.w());
        (xyz0 + xyz1 + xyz2 + xyz3) * 0.75
    }

    pub fn to_tuple(&self) -> (f32, f32, f32) {
        (self.x, self.y, self.z)
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
fn xyz_to_spectrum_4(xyz: (f32, f32, f32), wavelengths: Vec4) -> Vec4 {
    spectrum_xyz_to_p_4(wavelengths, xyz) * Vec4::splat(1.0 / EQUAL_ENERGY_REFLECTANCE)
    // aces_to_spectrum_p4(wavelengths, xyz_to_aces_ap0_e(xyz))
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
