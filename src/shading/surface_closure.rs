#![allow(dead_code)]

use std::f32::consts::PI as PI_32;

use glam::Vec4;

use crate::{
    color::{Color, SpectralSample},
    lerp::{lerp, Lerp},
    math::{dot, zup_to_vec, Normal, Vector},
    sampling::cosine_sample_hemisphere,
};

const INV_PI: f32 = 1.0 / PI_32;
const H_PI: f32 = PI_32 / 2.0;

/// A surface closure, specifying a BSDF for a point on a surface.
#[derive(Debug, Copy, Clone)]
pub enum SurfaceClosure {
    // Normal surface closures.
    Lambert(Color),
    GGX {
        color: Color,
        roughness: f32,
        fresnel: f32, // [0.0, 1.0] determines how much fresnel reflection comes into play
    },

    // Special closures that need special handling by the renderer.
    Emit(Color),
}

use self::SurfaceClosure::*;

/// Note when implementing new BSDFs: both the the color filter and pdf returned from
/// `sample()` and `evaluate()` should be identical for the same parameters and outgoing
/// light direction.
impl SurfaceClosure {
    /// Returns whether the closure has a delta distribution or not.
    pub fn is_delta(&self) -> bool {
        match *self {
            Lambert(_) => false,
            GGX { roughness, .. } => roughness == 0.0,
            Emit(_) => false,
        }
    }

    /// Given an incoming ray and sample values, generates an outgoing ray and
    /// color filter.
    ///
    /// inc:        Incoming light direction.
    /// nor:        The shading surface normal at the surface point.
    /// nor_g:      The geometric surface normal at the surface point.
    /// uv:         The sampling values.
    /// wavelength: Hero wavelength to generate the color filter for.
    ///
    /// Returns a tuple with the generated outgoing light direction, color filter, and pdf.
    pub fn sample(
        &self,
        inc: Vector,
        nor: Normal,
        nor_g: Normal,
        uv: (f32, f32),
        wavelength: f32,
    ) -> (Vector, SpectralSample, f32) {
        match *self {
            Lambert(color) => lambert_closure::sample(color, inc, nor, nor_g, uv, wavelength),

            GGX {
                color,
                roughness,
                fresnel,
            } => ggx_closure::sample(color, roughness, fresnel, inc, nor, nor_g, uv, wavelength),

            Emit(color) => emit_closure::sample(color, inc, nor, nor_g, uv, wavelength),
        }
    }

    /// Evaluates the closure for the given incoming and outgoing rays.
    ///
    /// inc:        The incoming light direction.
    /// out:        The outgoing light direction.
    /// nor:        The shading surface normal at the surface point.
    /// nor_g:      The geometric surface normal at the surface point.
    /// wavelength: Hero wavelength to generate the color filter for.
    ///
    /// Returns the resulting filter color and pdf of if this had been generated
    /// by `sample()`.
    pub fn evaluate(
        &self,
        inc: Vector,
        out: Vector,
        nor: Normal,
        nor_g: Normal,
        wavelength: f32,
    ) -> (SpectralSample, f32) {
        match *self {
            Lambert(color) => lambert_closure::evaluate(color, inc, out, nor, nor_g, wavelength),

            GGX {
                color,
                roughness,
                fresnel,
            } => ggx_closure::evaluate(color, roughness, fresnel, inc, out, nor, nor_g, wavelength),

            Emit(color) => emit_closure::evaluate(color, inc, out, nor, nor_g, wavelength),
        }
    }

    /// Returns an estimate of the sum total energy that evaluate() would return
    /// when integrated over a spherical light source with a center at relative
    /// position 'to_light_center' and squared radius 'light_radius_squared'.
    /// This is used for importance sampling, so does not need to be exact,
    /// but it does need to be non-zero anywhere that an exact solution would
    /// be non-zero.
    pub fn estimate_eval_over_sphere_light(
        &self,
        inc: Vector,
        to_light_center: Vector,
        light_radius_squared: f32,
        nor: Normal,
        nor_g: Normal,
    ) -> f32 {
        match *self {
            Lambert(color) => lambert_closure::estimate_eval_over_sphere_light(
                color,
                inc,
                to_light_center,
                light_radius_squared,
                nor,
                nor_g,
            ),
            GGX {
                color,
                roughness,
                fresnel,
            } => ggx_closure::estimate_eval_over_sphere_light(
                color,
                roughness,
                fresnel,
                inc,
                to_light_center,
                light_radius_squared,
                nor,
                nor_g,
            ),
            Emit(color) => emit_closure::estimate_eval_over_sphere_light(
                color,
                inc,
                to_light_center,
                light_radius_squared,
                nor,
                nor_g,
            ),
        }
    }

    /// Returns the post-compression size of this closure.
    pub fn compressed_size(&self) -> usize {
        1 + match *self {
            Lambert(color) => color.compressed_size(),
            GGX { color, .. } => {
                2 // Roughness
                + 2 // Fresnel
                + color.compressed_size() // Color
            }
            Emit(color) => color.compressed_size(),
        }
    }

    /// Writes the compressed form of this closure to `out_data`.
    ///
    /// `out_data` must be at least `compressed_size()` bytes long, otherwise
    /// this method will panic.
    ///
    /// Returns the number of bytes written.
    pub fn write_compressed(&self, out_data: &mut [u8]) -> usize {
        match *self {
            Lambert(color) => {
                out_data[0] = 0; // Discriminant
                color.write_compressed(&mut out_data[1..]);
            }
            GGX {
                color,
                roughness,
                fresnel,
            } => {
                out_data[0] = 1; // Discriminant

                // Roughness and fresnel (we write these first because they are
                // constant-size, whereas the color is variable-size, so this
                // makes things a little easier).
                let rgh =
                    ((roughness.max(0.0).min(1.0) * std::u16::MAX as f32) as u16).to_le_bytes();
                let frs = ((fresnel.max(0.0).min(1.0) * std::u16::MAX as f32) as u16).to_le_bytes();
                out_data[1] = rgh[0];
                out_data[2] = rgh[1];
                out_data[3] = frs[0];
                out_data[4] = frs[1];

                // Color
                color.write_compressed(&mut out_data[5..]); // Color
            }
            Emit(color) => {
                out_data[0] = 2; // Discriminant
                color.write_compressed(&mut out_data[1..]);
            }
        }
        self.compressed_size()
    }

    /// Constructs a SurfaceClosure from compressed closure data, and also
    /// returns the number of bytes consumed from `in_data`.
    pub fn from_compressed(in_data: &[u8]) -> (SurfaceClosure, usize) {
        match in_data[0] {
            0 => {
                // Lambert
                let (col, size) = Color::from_compressed(&in_data[1..]);
                (SurfaceClosure::Lambert(col), 1 + size)
            }

            1 => {
                // GGX
                let mut rgh = [0u8; 2];
                let mut frs = [0u8; 2];
                rgh[0] = in_data[1];
                rgh[1] = in_data[2];
                frs[0] = in_data[3];
                frs[1] = in_data[4];
                let rgh = u16::from_le_bytes(rgh) as f32 * (1.0 / std::u16::MAX as f32);
                let frs = u16::from_le_bytes(frs) as f32 * (1.0 / std::u16::MAX as f32);
                let (col, size) = Color::from_compressed(&in_data[5..]);
                (
                    SurfaceClosure::GGX {
                        color: col,
                        roughness: rgh,
                        fresnel: frs,
                    },
                    5 + size,
                )
            }

            2 => {
                // Emit
                let (col, size) = Color::from_compressed(&in_data[1..]);
                (SurfaceClosure::Emit(col), 1 + size)
            }

            _ => unreachable!(),
        }
    }
}

impl Lerp for SurfaceClosure {
    fn lerp(self, other: SurfaceClosure, alpha: f32) -> SurfaceClosure {
        match (self, other) {
            (Lambert(col1), Lambert(col2)) => Lambert(lerp(col1, col2, alpha)),
            (
                GGX {
                    color: col1,
                    roughness: rgh1,
                    fresnel: frs1,
                },
                GGX {
                    color: col2,
                    roughness: rgh2,
                    fresnel: frs2,
                },
            ) => GGX {
                color: lerp(col1, col2, alpha),
                roughness: lerp(rgh1, rgh2, alpha),
                fresnel: lerp(frs1, frs2, alpha),
            },
            (Emit(col1), Emit(col2)) => Emit(lerp(col1, col2, alpha)),

            _ => panic!("Cannot lerp between different surface closure types."),
        }
    }
}

/// Lambert closure code.
mod lambert_closure {
    use super::*;

    pub fn sample(
        color: Color,
        inc: Vector,
        nor: Normal,
        nor_g: Normal,
        uv: (f32, f32),
        wavelength: f32,
    ) -> (Vector, SpectralSample, f32) {
        let (nn, flipped_nor_g) = if dot(nor_g.into_vector(), inc) <= 0.0 {
            (nor.normalized().into_vector(), nor_g.into_vector())
        } else {
            (-nor.normalized().into_vector(), -nor_g.into_vector())
        };

        // Generate a random ray direction in the hemisphere
        // of the shading surface normal.
        let dir = cosine_sample_hemisphere(uv.0, uv.1);
        let pdf = dir.z() * INV_PI;
        let out = zup_to_vec(dir, nn);

        // Make sure it's not on the wrong side of the geometric normal.
        if dot(flipped_nor_g, out) >= 0.0 {
            (out, color.to_spectral_sample(wavelength) * pdf, pdf)
        } else {
            (out, SpectralSample::new(0.0), 0.0)
        }
    }

    pub fn evaluate(
        color: Color,
        inc: Vector,
        out: Vector,
        nor: Normal,
        nor_g: Normal,
        wavelength: f32,
    ) -> (SpectralSample, f32) {
        let (nn, flipped_nor_g) = if dot(nor_g.into_vector(), inc) <= 0.0 {
            (nor.normalized().into_vector(), nor_g.into_vector())
        } else {
            (-nor.normalized().into_vector(), -nor_g.into_vector())
        };

        if dot(flipped_nor_g, out) >= 0.0 {
            let fac = dot(nn, out.normalized()).max(0.0) * INV_PI;
            (color.to_spectral_sample(wavelength) * fac, fac)
        } else {
            (SpectralSample::new(0.0), 0.0)
        }
    }

    pub fn estimate_eval_over_sphere_light(
        _color: Color,
        inc: Vector,
        to_light_center: Vector,
        light_radius_squared: f32,
        nor: Normal,
        nor_g: Normal,
    ) -> f32 {
        let _ = nor_g; // Not using this, silence warning

        // Analytically calculates lambert shading from a uniform light source
        // subtending a circular solid angle.
        // Only works for solid angle subtending equal to or less than a hemisphere.
        //
        // Formula taken from "Area Light Sources for Real-Time Graphics"
        // by John M. Snyder
        fn sphere_lambert(nlcos: f32, rcos: f32) -> f32 {
            assert!(nlcos >= -1.0 && nlcos <= 1.0);
            assert!(rcos >= 0.0 && rcos <= 1.0);

            let nlsin: f32 = (1.0 - (nlcos * nlcos)).sqrt();
            let rsin2: f32 = 1.0 - (rcos * rcos);
            let rsin: f32 = rsin2.sqrt();
            let ysin: f32 = rcos / nlsin;
            let ycos2: f32 = 1.0 - (ysin * ysin);
            let ycos: f32 = ycos2.sqrt();

            let g: f32 = (-2.0 * nlsin * rcos * ycos) + H_PI - ysin.asin() + (ysin * ycos);
            let h: f32 = nlcos * ((ycos * (rsin2 - ycos2).sqrt()) + (rsin2 * (ycos / rsin).asin()));

            let nl: f32 = nlcos.acos();
            let r: f32 = rcos.acos();

            if nl < (H_PI - r) {
                nlcos * rsin2
            } else if nl < H_PI {
                (nlcos * rsin2) + g - h
            } else if nl < (H_PI + r) {
                (g + h) * INV_PI
            } else {
                0.0
            }
        }

        let dist2 = to_light_center.length2();
        if dist2 <= light_radius_squared {
            return (light_radius_squared / dist2).min(4.0);
        } else {
            let sin_theta_max2 = (light_radius_squared / dist2).min(1.0);
            let cos_theta_max = (1.0 - sin_theta_max2).sqrt();

            let v = to_light_center.normalized();
            let nn = if dot(nor_g.into_vector(), inc) <= 0.0 {
                nor.normalized()
            } else {
                -nor.normalized()
            }
            .into_vector();

            let cos_nv = dot(nn, v).max(-1.0).min(1.0);

            // Alt implementation from the SPI paper.
            // Worse sampling, but here for reference.
            // {
            //     let nl_ang = cos_nv.acos();
            //     let rad_ang = cos_theta_max.acos();
            //     let min_ang = (nl_ang - rad_ang).max(0.0);
            //     let lamb = min_ang.cos().max(0.0);

            //     return lamb / dist2;
            // }

            return sphere_lambert(cos_nv, cos_theta_max);
        }
    }
}

mod ggx_closure {
    use super::*;

    // Makes sure values are in a valid range
    pub fn validate(roughness: f32, fresnel: f32) {
        debug_assert!(fresnel >= 0.0 && fresnel <= 1.0);
        debug_assert!(roughness >= 0.0 && roughness <= 1.0);
    }

    pub fn sample(
        col: Color,
        roughness: f32,
        fresnel: f32,
        inc: Vector,
        nor: Normal,
        nor_g: Normal,
        uv: (f32, f32),
        wavelength: f32,
    ) -> (Vector, SpectralSample, f32) {
        // Get normalized surface normal
        let (nn, flipped_nor_g) = if dot(nor_g.into_vector(), inc) <= 0.0 {
            (nor.normalized().into_vector(), nor_g.into_vector())
        } else {
            (-nor.normalized().into_vector(), -nor_g.into_vector())
        };

        // Generate a random ray direction in the hemisphere
        // of the surface.
        let theta_cos = half_theta_sample(uv.0, roughness);
        let theta_sin = (1.0 - (theta_cos * theta_cos)).sqrt();
        let angle = uv.1 * PI_32 * 2.0;
        let mut half_dir = Vector::new(angle.cos() * theta_sin, angle.sin() * theta_sin, theta_cos);
        half_dir = zup_to_vec(half_dir, nn).normalized();

        let out = inc - (half_dir * 2.0 * dot(inc, half_dir));

        // Make sure it's not on the wrong side of the geometric normal.
        if dot(flipped_nor_g, out) >= 0.0 {
            let (filter, pdf) = evaluate(col, roughness, fresnel, inc, out, nor, nor_g, wavelength);
            (out, filter, pdf)
        } else {
            (out, SpectralSample::new(0.0), 0.0)
        }
    }

    pub fn evaluate(
        col: Color,
        roughness: f32,
        fresnel: f32,
        inc: Vector,
        out: Vector,
        nor: Normal,
        nor_g: Normal,
        wavelength: f32,
    ) -> (SpectralSample, f32) {
        // Calculate needed vectors, normalized
        let aa = -inc.normalized(); // Vector pointing to where "in" came from
        let bb = out.normalized(); // Out
        let hh = (aa + bb).normalized(); // Half-way between aa and bb

        // Surface normal
        let (nn, flipped_nor_g) = if dot(nor_g.into_vector(), inc) <= 0.0 {
            (nor.normalized().into_vector(), nor_g.into_vector())
        } else {
            (-nor.normalized().into_vector(), -nor_g.into_vector())
        };

        // Make sure everything's on the correct side of the surface
        if dot(nn, aa) < 0.0 || dot(nn, bb) < 0.0 || dot(flipped_nor_g, bb) < 0.0 {
            return (SpectralSample::new(0.0), 0.0);
        }

        // Calculate needed dot products
        let na = dot(nn, aa).clamp(-1.0, 1.0);
        let nb = dot(nn, bb).clamp(-1.0, 1.0);
        let ha = dot(hh, aa).clamp(-1.0, 1.0);
        let hb = dot(hh, bb).clamp(-1.0, 1.0);
        let nh = dot(nn, hh).clamp(-1.0, 1.0);

        // Calculate F - Fresnel
        let col_f = {
            let spectrum_sample = col.to_spectral_sample(wavelength);
            let rev_fresnel = 1.0 - fresnel;
            let c0 = lerp(
                schlick_fresnel_from_fac(spectrum_sample.e[0], hb),
                spectrum_sample.e[0],
                rev_fresnel,
            );
            let c1 = lerp(
                schlick_fresnel_from_fac(spectrum_sample.e[1], hb),
                spectrum_sample.e[1],
                rev_fresnel,
            );
            let c2 = lerp(
                schlick_fresnel_from_fac(spectrum_sample.e[2], hb),
                spectrum_sample.e[2],
                rev_fresnel,
            );
            let c3 = lerp(
                schlick_fresnel_from_fac(spectrum_sample.e[3], hb),
                spectrum_sample.e[3],
                rev_fresnel,
            );

            SpectralSample::from_parts(Vec4::new(c0, c1, c2, c3), wavelength)
        };

        // Calculate everything else
        if roughness == 0.0 {
            // If sharp mirror, just return col * fresnel factor
            return (col_f, 0.0);
        } else {
            // Calculate D - Distribution
            let dist = ggx_d(nh, roughness) / na;

            // Calculate G1 and G2- Geometric microfacet shadowing
            let g1 = ggx_g(ha, na, roughness);
            let g2 = ggx_g(hb, nb, roughness);

            // Final result
            (col_f * (dist * g1 * g2) * INV_PI, dist * INV_PI)
        }
    }

    pub fn estimate_eval_over_sphere_light(
        _col: Color,
        roughness: f32,
        _fresnel: f32,
        inc: Vector,
        to_light_center: Vector,
        light_radius_squared: f32,
        nor: Normal,
        nor_g: Normal,
    ) -> f32 {
        // TODO: all of the stuff in this function is horribly hacky.
        // Find a proper way to approximate the light contribution from a
        // solid angle.

        let _ = nor_g; // Not using this, silence warning

        let dist2 = to_light_center.length2();
        let sin_theta_max2 = (light_radius_squared / dist2).min(1.0);
        let cos_theta_max = (1.0 - sin_theta_max2).sqrt();

        assert!(cos_theta_max >= -1.0);
        assert!(cos_theta_max <= 1.0);

        // Surface normal
        let nn = if dot(nor.into_vector(), inc) < 0.0 {
            nor.normalized()
        } else {
            -nor.normalized() // If back-facing, flip normal
        }
        .into_vector();

        let aa = -inc.normalized(); // Vector pointing to where "in" came from
        let bb = to_light_center.normalized(); // Out

        // Brute-force method
        //let mut fac = 0.0;
        //const N: usize = 256;
        //for i in 0..N {
        //    let uu = Halton::sample(0, i);
        //    let vv = Halton::sample(1, i);
        //    let mut samp = uniform_sample_cone(uu, vv, cos_theta_max);
        //    samp = zup_to_vec(samp, bb).normalized();
        //    if dot(nn, samp) > 0.0 {
        //        let hh = (aa+samp).normalized();
        //        fac += ggx_d(dot(nn, hh), roughness);
        //    }
        //}
        //fac /= N * N;

        // Approximate method
        let theta = cos_theta_max.acos();
        let hh = (aa + bb).normalized();
        let nh = dot(nn, hh).clamp(-1.0, 1.0);
        let fac = ggx_d(nh, (1.0f32).min(roughness.sqrt() + (2.0 * theta / PI_32)));

        fac * (1.0f32).min(1.0 - cos_theta_max) * INV_PI
    }

    //----------------------------------------------------

    // Returns the cosine of the half-angle that should be sampled, given
    // a random variable in [0,1]
    fn half_theta_sample(u: f32, rough: f32) -> f32 {
        let rough2 = rough * rough;

        // Calculate top half of equation
        let top = 1.0 - u;

        // Calculate bottom half of equation
        let bottom = 1.0 + ((rough2 - 1.0) * u);

        (top / bottom).sqrt()
    }

    /// The GGX microfacet distribution function.
    ///
    /// nh: cosine of the angle between the surface normal and the microfacet normal.
    fn ggx_d(nh: f32, rough: f32) -> f32 {
        if nh <= 0.0 {
            return 0.0;
        }

        let rough2 = rough * rough;
        let tmp = 1.0 + ((rough2 - 1.0) * (nh * nh));
        rough2 / (PI_32 * tmp * tmp)
    }

    /// The GGX Smith shadow-masking function.
    ///
    /// vh: cosine of the angle between the view vector and the microfacet normal.
    /// vn: cosine of the angle between the view vector and surface normal.
    fn ggx_g(vh: f32, vn: f32, rough: f32) -> f32 {
        if (vh * vn) <= 0.0 {
            0.0
        } else {
            2.0 / (1.0 + (1.0 + rough * rough * (1.0 - vn * vn) / (vn * vn)).sqrt())
        }
    }
}

/// Emit closure code.
///
/// NOTE: this needs to be handled specially by the integrator!  It does not
/// behave like a standard closure!
mod emit_closure {
    use super::*;

    pub fn sample(
        color: Color,
        inc: Vector,
        nor: Normal,
        nor_g: Normal,
        uv: (f32, f32),
        wavelength: f32,
    ) -> (Vector, SpectralSample, f32) {
        let _ = (inc, nor, nor_g, uv); // Not using these, silence warning

        (
            Vector::new(0.0, 0.0, 0.0),
            color.to_spectral_sample(wavelength),
            1.0,
        )
    }

    pub fn evaluate(
        color: Color,
        inc: Vector,
        out: Vector,
        nor: Normal,
        nor_g: Normal,
        wavelength: f32,
    ) -> (SpectralSample, f32) {
        let _ = (inc, out, nor, nor_g); // Not using these, silence warning

        (color.to_spectral_sample(wavelength), 1.0)
    }

    pub fn estimate_eval_over_sphere_light(
        _color: Color,
        _inc: Vector,
        _to_light_center: Vector,
        _light_radius_squared: f32,
        _nor: Normal,
        _nor_g: Normal,
    ) -> f32 {
        // TODO: what to do here?
        unimplemented!()
    }
}

//=============================================================================

/// Utility function that calculates the fresnel reflection factor of a given
/// incoming ray against a surface with the given normal-reflectance factor.
///
/// `frensel_fac`: The ratio of light reflected back if the ray were to
///                hit the surface head-on (perpendicular to the surface).
/// `c`: The cosine of the angle between the incoming light and the
///      surface's normal.  Probably calculated e.g. with a normalized
///      dot product.
#[allow(dead_code)]
fn dielectric_fresnel_from_fac(fresnel_fac: f32, c: f32) -> f32 {
    let tmp1 = fresnel_fac.sqrt() - 1.0;

    // Protect against divide by zero.
    if tmp1.abs() < 0.000_001 {
        return 1.0;
    }

    // Find the ior ratio
    let tmp2 = (-2.0 / tmp1) - 1.0;
    let ior_ratio = tmp2 * tmp2;

    // Calculate fresnel factor
    dielectric_fresnel(ior_ratio, c)
}

/// Schlick's approximation version of `dielectric_fresnel_from_fac()` above.
#[allow(dead_code)]
fn schlick_fresnel_from_fac(frensel_fac: f32, c: f32) -> f32 {
    let c1 = 1.0 - c;
    let c2 = c1 * c1;
    frensel_fac + ((1.0 - frensel_fac) * c1 * c2 * c2)
}

/// Utility function that calculates the fresnel reflection factor of a given
/// incoming ray against a surface with the given ior outside/inside ratio.
///
/// `ior_ratio`: The ratio of the outside material ior (probably 1.0 for air)
///              over the inside ior.
/// `c`: The cosine of the angle between the incoming light and the
///      surface's normal.  Probably calculated e.g. with a normalized
///      dot product.
#[allow(dead_code)]
fn dielectric_fresnel(ior_ratio: f32, c: f32) -> f32 {
    let g = (ior_ratio - 1.0 + (c * c)).sqrt();

    let f1 = g - c;
    let f2 = g + c;
    let f3 = (f1 * f1) / (f2 * f2);

    let f4 = (c * f2) - 1.0;
    let f5 = (c * f1) + 1.0;
    let f6 = 1.0 + ((f4 * f4) / (f5 * f5));

    0.5 * f3 * f6
}

/// Schlick's approximation of the fresnel reflection factor.
///
/// Same interface as `dielectric_fresnel()`, above.
#[allow(dead_code)]
fn schlick_fresnel(ior_ratio: f32, c: f32) -> f32 {
    let f1 = (1.0 - ior_ratio) / (1.0 + ior_ratio);
    let f2 = f1 * f1;
    let c1 = 1.0 - c;
    let c2 = c1 * c1;

    f2 + ((1.0 - f2) * c1 * c2 * c2)
}
