#![allow(dead_code)]

use std::path::Path;
use std::sync::Mutex;
use std::cell::RefCell;
use scoped_threadpool::Pool;

use tracer::Tracer;
use halton;
use math::fast_logit;
use image::Image;
use surface;
use scene::Scene;

#[derive(Debug)]
pub struct Renderer {
    pub output_file: String,
    pub resolution: (usize, usize),
    pub spp: usize,
    pub scene: Scene,
}

impl Renderer {
    pub fn render(&self, thread_count: u32) {
        let mut tpool = Pool::new(thread_count);

        let img = Mutex::new(RefCell::new(Image::new(self.resolution.0, self.resolution.1)));


        // Pre-calculate some useful values related to the image plane
        let cmpx = 1.0 / self.resolution.0 as f32;
        let cmpy = 1.0 / self.resolution.1 as f32;
        let min_x = -1.0;
        let max_x = 1.0;
        let min_y = -(self.resolution.1 as f32 / self.resolution.0 as f32);
        let max_y = self.resolution.1 as f32 / self.resolution.0 as f32;
        let x_extent = max_x - min_x;
        let y_extent = max_y - min_y;


        // Render
        tpool.scoped(|scope| {
            let (img_width, img_height) = {
                let i = img.lock().unwrap();
                let w = i.borrow().width();
                let h = i.borrow().height();
                (w, h)
            };
            for y in 0..img_height {
                for x in 0..img_width {
                    let img = &img;
                    scope.execute(move || {
                        let mut rays = Vec::new();
                        let mut tracer = Tracer::from_assembly(&self.scene.root);

                        let offset = hash_u32(((x as u32) << 16) ^ (y as u32), 0);

                        // Generate rays
                        rays.clear();
                        for si in 0..self.spp {
                            let mut ray = {
                                let filter_x =
                                    fast_logit(halton::sample(3, offset + si as u32), 1.5) + 0.5;
                                let filter_y =
                                    fast_logit(halton::sample(4, offset + si as u32), 1.5) + 0.5;
                                let samp_x = (filter_x + x as f32) * cmpx;
                                let samp_y = (filter_y + y as f32) * cmpy;

                                self.scene.camera.generate_ray((samp_x - 0.5) * x_extent,
                                                               (0.5 - samp_y) * y_extent,
                                                               halton::sample(0,
                                                                              offset + si as u32),
                                                               halton::sample(1,
                                                                              offset + si as u32),
                                                               halton::sample(2,
                                                                              offset + si as u32))
                            };
                            ray.id = si as u32;
                            rays.push(ray);
                        }

                        // Test rays against scene
                        let isects = tracer.trace(&rays);

                        // Calculate color based on ray hits
                        let mut r = 0.0;
                        let mut g = 0.0;
                        let mut b = 0.0;
                        for isect in isects.iter() {
                            if let &surface::SurfaceIntersection::Hit { t: _,
                                                                        pos: _,
                                                                        nor: _,
                                                                        space: _,
                                                                        uv } = isect {
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
                        let img = img.lock().unwrap();
                        img.borrow_mut().set(x, y, (r as u8, g as u8, b as u8));
                    });
                }
            }
            // scope.defer(|| println!("Exiting scope"));
            // scope.spawn(|| println!("Running child thread in scope"))
        });


        // Write rendered image to disk
        {
            let img = &img.lock().unwrap();
            let _ = img.borrow().write_binary_ppm(Path::new(&self.output_file));
        }
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
