mod spectra_xyz;

use self::spectra_xyz::{spectrum_xyz_to_p, EQUAL_ENERGY_REFLECTANCE};

// Minimum and maximum wavelength of light we care about, in nanometers
const WL_MIN: f32 = 380.0;
const WL_MAX: f32 = 700.0;
const WL_RANGE: f32 = WL_MAX - WL_MIN;
const WL_RANGE_Q: f32 = WL_RANGE / 4.0;

pub trait Color {
    fn sample_spectrum(&self, wavelength: f32) -> f32;
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

    pub fn add_color<T: Color>(&mut self, color: &T) {
        self.e[0] += color.sample_spectrum(self.wl_n(0));
        self.e[1] += color.sample_spectrum(self.wl_n(1));
        self.e[2] += color.sample_spectrum(self.wl_n(2));
        self.e[3] += color.sample_spectrum(self.wl_n(3));
    }

    pub fn mul_color<T: Color>(&mut self, color: &T) {
        self.e[0] *= color.sample_spectrum(self.wl_n(0));
        self.e[1] *= color.sample_spectrum(self.wl_n(1));
        self.e[2] *= color.sample_spectrum(self.wl_n(2));
        self.e[3] *= color.sample_spectrum(self.wl_n(3));
    }

    fn wl_n(&self, n: usize) -> f32 {
        let mut wl = self.hero_wavelength + (WL_RANGE_Q * n as f32);
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
    fn new(x: f32, y: f32, z: f32) -> XYZ {
        XYZ { x: x, y: y, z: z }
    }

    fn from_wavelength(wavelength: f32, intensity: f32) -> XYZ {
        XYZ {
            x: x_1931(wavelength) * intensity,
            y: y_1931(wavelength) * intensity,
            z: z_1931(wavelength) * intensity,
        }
    }
}

impl Color for XYZ {
    fn sample_spectrum(&self, wavelength: f32) -> f32 {
        xyz_to_spectrum((self.x, self.y, self.z), wavelength)
    }
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
