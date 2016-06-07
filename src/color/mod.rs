mod spectra_xyz;

use self::spectra_xyz::{spectrum_xyz_to_p, EQUAL_ENERGY_REFLECTANCE};

pub fn xyz_to_spectrum(xyz: (f32, f32, f32), wavelength: f32) -> f32 {
    spectrum_xyz_to_p(wavelength, xyz) * (1.0 / EQUAL_ENERGY_REFLECTANCE)
}
