use math::{Vector, Normal, dot, zup_to_vec};
use color::{XYZ, SpectralSample, Color};
use sampling::cosine_sample_hemisphere;
use std::f32::consts::PI as PI_32;
const INV_PI: f32 = 1.0 / PI_32;
const H_PI: f32 = PI_32 / 2.0;

#[derive(Debug, Copy, Clone)]
pub enum SurfaceClosureUnion {
    EmitClosure(EmitClosure),
    LambertClosure(LambertClosure),
}

impl SurfaceClosureUnion {
    pub fn as_surface_closure(&self) -> &SurfaceClosure {
        match self {
            &SurfaceClosureUnion::EmitClosure(ref closure) => closure as &SurfaceClosure,
            &SurfaceClosureUnion::LambertClosure(ref closure) => closure as &SurfaceClosure,
        }
    }
}

/// Trait for surface closures.
pub trait SurfaceClosure {
    /// Returns whether the closure has a delta distribution or not.
    fn is_delta(&self) -> bool;

    /// Given an incoming ray and sample values, generates an outgoing ray and
    /// color filter.
    ///
    /// inc: Incoming light direction.
    /// nor: The surface normal at the surface point.
    /// uv:  The sampling values.
    /// wavelength: The wavelength of light to sample at.
    ///
    /// Returns a tuple with the generated outgoing light direction, color filter, and pdf.
    fn sample(&self,
              inc: Vector,
              nor: Normal,
              uv: (f32, f32),
              wavelength: f32)
              -> (Vector, SpectralSample, f32);

    /// Evaluates the closure for the given incoming and outgoing rays.
    ///
    /// inc: The incoming light direction.
    /// out: The outgoing light direction.
    /// nor: The surface normal of the reflecting/transmitting surface point.
    /// wavelength: The wavelength of light to evaluate for.
    ///
    /// Returns the resulting filter color.
    fn evaluate(&self, inc: Vector, out: Vector, nor: Normal, wavelength: f32) -> SpectralSample;

    /// Returns the pdf for the given 'in' direction producing the given 'out'
    /// direction with the given differential geometry.
    ///
    /// inc: The incoming light direction.
    /// out: The outgoing light direction.
    /// nor: The surface normal of the reflecting/transmitting surface point.
    fn sample_pdf(&self, inc: Vector, out: Vector, nor: Normal) -> f32;

    /// Returns an estimate of the sum total energy that evaluate() would return
    /// when 'out' is evaluated over a circular solid angle.
    /// This is used for importance sampling, so does not need to be exact,
    /// but it does need to be non-zero anywhere that an exact solution would
    /// be non-zero.
    fn estimate_eval_over_solid_angle(&self,
                                      inc: Vector,
                                      out: Vector,
                                      nor: Normal,
                                      cos_theta: f32)
                                      -> f32;
}


/// Utility function that calculates the fresnel reflection factor of a given
/// incoming ray against a surface with the given ior outside/inside ratio.
///
/// ior_ratio: The ratio of the outside material ior (probably 1.0 for air)
///            over the inside ior.
/// c: The cosine of the angle between the incoming light and the
///    surface's normal.  Probably calculated e.g. with a normalized
///    dot product.
#[allow(dead_code)]
fn dielectric_fresnel(ior_ratio: f32, c: f32) -> f32 {
    let g = (ior_ratio - 1.0 + (c * c)).sqrt();

    let f1 = g - c;
    let f2 = g + c;
    let f3 = (f1 * f1) / (f2 * f2);

    let f4 = (c * f2) - 1.0;
    let f5 = (c * f1) + 1.0;
    let f6 = 1.0 + ((f4 * f4) / (f5 * f5));

    return 0.5 * f3 * f6;
}


/// Schlick's approximation of the fresnel reflection factor.
///
/// Same interface as dielectric_fresnel(), above.
#[allow(dead_code)]
fn schlick_fresnel(ior_ratio: f32, c: f32) -> f32 {
    let f1 = (1.0 - ior_ratio) / (1.0 + ior_ratio);
    let f2 = f1 * f1;
    let c1 = 1.0 - c;
    let c2 = c1 * c1;
    return f2 + ((1.0 - f2) * c1 * c2 * c2);
}


/// Utility function that calculates the fresnel reflection factor of a given
/// incoming ray against a surface with the given normal-reflectance factor.
///
/// frensel_fac: The ratio of light reflected back if the ray were to
///              hit the surface head-on (perpendicular to the surface).
/// c The cosine of the angle between the incoming light and the
///   surface's normal.  Probably calculated e.g. with a normalized
///   dot product.
#[allow(dead_code)]
fn dielectric_fresnel_from_fac(fresnel_fac: f32, c: f32) -> f32 {
    let tmp1 = fresnel_fac.sqrt() - 1.0;

    // Protect against divide by zero.
    if tmp1.abs() < 0.000001 {
        return 1.0;
    }

    // Find the ior ratio
    let tmp2 = (-2.0 / tmp1) - 1.0;
    let ior_ratio = tmp2 * tmp2;

    // Calculate fresnel factor
    return dielectric_fresnel(ior_ratio, c);
}


/// Schlick's approximation version of dielectric_fresnel_from_fac() above.
#[allow(dead_code)]
fn schlick_fresnel_from_fac(frensel_fac: f32, c: f32) -> f32 {
    let c1 = 1.0 - c;
    let c2 = c1 * c1;
    return frensel_fac + ((1.0 - frensel_fac) * c1 * c2 * c2);
}


/// Emit closure.
///
/// NOTE: this needs to be handled specially by the integrator!  It does not
/// behave like a standard closure!
#[derive(Debug, Copy, Clone)]
pub struct EmitClosure {
    col: XYZ,
}

impl EmitClosure {
    pub fn emitted_color(&self, wavelength: f32) -> SpectralSample {
        self.col.to_spectral_sample(wavelength)
    }
}

impl SurfaceClosure for EmitClosure {
    fn is_delta(&self) -> bool {
        false
    }

    fn sample(&self,
              inc: Vector,
              nor: Normal,
              uv: (f32, f32),
              wavelength: f32)
              -> (Vector, SpectralSample, f32) {
        let _ = (inc, nor, uv); // Not using these, silence warning

        (Vector::new(0.0, 0.0, 0.0), SpectralSample::new(wavelength), 1.0)
    }

    fn evaluate(&self, inc: Vector, out: Vector, nor: Normal, wavelength: f32) -> SpectralSample {
        let _ = (inc, out, nor); // Not using these, silence warning

        SpectralSample::new(wavelength)
    }

    fn sample_pdf(&self, inc: Vector, out: Vector, nor: Normal) -> f32 {
        let _ = (inc, out, nor); // Not using these, silence warning

        1.0
    }

    fn estimate_eval_over_solid_angle(&self,
                                      inc: Vector,
                                      out: Vector,
                                      nor: Normal,
                                      cos_theta: f32)
                                      -> f32 {
        let _ = (inc, out, nor, cos_theta); // Not using these, silence warning

        // TODO: what to do here?
        unimplemented!()
    }
}


/// Lambertian surface closure
#[derive(Debug, Copy, Clone)]
pub struct LambertClosure {
    col: XYZ,
}

impl LambertClosure {
    pub fn new(col: XYZ) -> LambertClosure {
        LambertClosure { col: col }
    }
}

impl SurfaceClosure for LambertClosure {
    fn is_delta(&self) -> bool {
        false
    }

    fn sample(&self,
              inc: Vector,
              nor: Normal,
              uv: (f32, f32),
              wavelength: f32)
              -> (Vector, SpectralSample, f32) {
        let nn = if dot(nor.into_vector(), inc) <= 0.0 {
                nor.normalized()
            } else {
                -nor.normalized()
            }
            .into_vector();

        // Generate a random ray direction in the hemisphere
        // of the surface.
        let dir = cosine_sample_hemisphere(uv.0, uv.1);
        let pdf = dir.z() * INV_PI;
        let out = zup_to_vec(dir, nn);
        let filter = self.evaluate(inc, out, nor, wavelength);

        (out, filter, pdf)
    }

    fn evaluate(&self, inc: Vector, out: Vector, nor: Normal, wavelength: f32) -> SpectralSample {
        let v = out.normalized();
        let nn = if dot(nor.into_vector(), inc) <= 0.0 {
                nor.normalized()
            } else {
                -nor.normalized()
            }
            .into_vector();
        let fac = dot(nn, v).max(0.0) * INV_PI;

        self.col.to_spectral_sample(wavelength) * fac
    }

    fn sample_pdf(&self, inc: Vector, out: Vector, nor: Normal) -> f32 {
        let v = out.normalized();
        let nn = if dot(nor.into_vector(), inc) <= 0.0 {
                nor.normalized()
            } else {
                -nor.normalized()
            }
            .into_vector();

        dot(nn, v).max(0.0) * INV_PI
    }

    fn estimate_eval_over_solid_angle(&self,
                                      inc: Vector,
                                      out: Vector,
                                      nor: Normal,
                                      cos_theta: f32)
                                      -> f32 {
        assert!(cos_theta >= -1.0 && cos_theta <= 1.0);

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

        if cos_theta < 0.0 {
            return 1.0;
        } else {
            let v = out.normalized();
            let nn = if dot(nor.into_vector(), inc) <= 0.0 {
                    nor.normalized()
                } else {
                    -nor.normalized()
                }
                .into_vector();

            let cos_nv = dot(nn, v);

            return sphere_lambert(cos_nv, cos_theta);
        }
    }
}
