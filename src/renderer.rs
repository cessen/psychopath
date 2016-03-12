#![allow(dead_code)]

use std::path::Path;

use camera::Camera;
use halton;
use math::fast_logit;
use image::Image;
use surface;
use surface::Surface;
use scene::Scene;

#[derive(Debug)]
pub struct Renderer {
    pub output_file: String,
    pub resolution: (usize, usize),
    pub spp: usize,
    pub scene: Scene,
}

impl Renderer {
    pub fn render(&self) {
        let mut rays = Vec::new();
        let mut isects = Vec::new();
        let mut img = Image::new(self.resolution.0, self.resolution.1);

        // Render image of ray-traced triangle
        let cmpx = 1.0 / self.resolution.0 as f32;
        let cmpy = 1.0 / self.resolution.1 as f32;

        for y in 0..img.height() {
            for x in 0..img.width() {
                let offset = hash_u32(((x as u32) << 16) ^ (y as u32), 0);

                // Generate rays
                rays.clear();
                isects.clear();
                for si in 0..self.spp {
                    let mut ray = {
                        let filter_x = fast_logit(halton::sample(3, offset + si as u32), 1.5);
                        let filter_y = fast_logit(halton::sample(4, offset + si as u32), 1.5);
                        self.scene.camera.generate_ray((x as f32 + filter_x) * cmpx - 0.5,
                                                       (y as f32 + filter_y) * cmpy - 0.5,
                                                       halton::sample(0, offset + si as u32),
                                                       halton::sample(1, offset + si as u32),
                                                       halton::sample(2, offset + si as u32))
                    };
                    ray.id = si as u32;
                    rays.push(ray);
                    isects.push(surface::SurfaceIntersection::Miss);
                }

                // Test rays against scene
                self.scene.root.intersect_rays(&mut rays, &mut isects);

                // Calculate color based on ray hits
                let mut r = 0.0;
                let mut g = 0.0;
                let mut b = 0.0;
                for isect in isects.iter() {
                    if let &surface::SurfaceIntersection::Hit{
                            t: _,
                            pos: _,
                            nor: _,
                            space: _,
                            uv,
                        } = isect {
                        r += uv.0;
                        g += uv.1;
                        b += (1.0 - uv.0 - uv.1).max(0.0);
                    } else {
                        r += 0.02;
                        g += 0.02;
                        b += 0.02;
                    }
                }
                r = 255.0 * srgb_gamma(r / self.spp as f32);
                g = 255.0 * srgb_gamma(g / self.spp as f32);
                b = 255.0 * srgb_gamma(b / self.spp as f32);

                // Set pixel color
                img.set(x, y, (r as u8, g as u8, b as u8));
            }
        }

        // Write rendered image to disk
        let _ = img.write_binary_ppm(Path::new(&self.output_file));
    }
}


fn hash_u32(n: u32, seed: u32) -> u32 {
    let mut hash = n;

    for _ in 0..3 {
        hash = hash.wrapping_mul(1936502639);
        hash ^= hash.wrapping_shr(16);
        hash = hash.wrapping_add(seed);
    }

    return hash;
}


fn srgb_gamma(n: f32) -> f32 {
    if n < 0.0031308 {
        n * 12.92
    } else {
        (1.055 * n.powf(1.0 / 2.4)) - 0.055
    }
}

fn srgb_inv_gamma(n: f32) -> f32 {
    if n < 0.04045 {
        n / 12.92
    } else {
        ((n + 0.055) / 1.055).powf(2.4)
    }
}
