#![allow(dead_code)]

use std::path::Path;
use std::cmp::min;
use std::iter::Iterator;
use std::sync::{Mutex, RwLock};
use std::cell::RefCell;
use scoped_threadpool::Pool;
use crossbeam::sync::MsQueue;

use algorithm::partition_pair;
use lerp::lerp_slice;
use ray::Ray;
use assembly::Object;
use tracer::Tracer;
use halton;
use math::{Matrix4x4, dot, fast_logit};
use image::Image;
use surface;
use scene::Scene;
use color::{Color, XYZ, map_0_1_to_wavelength};

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

        let image = Mutex::new(RefCell::new(Image::new(self.resolution.0, self.resolution.1)));
        let (img_width, img_height) = {
            let i = image.lock().unwrap();
            let w = i.borrow().width();
            let h = i.borrow().height();
            (w, h)
        };

        let all_jobs_queued = RwLock::new(false);

        // Pre-calculate some useful values related to the image plane
        let cmpx = 1.0 / self.resolution.0 as f32;
        let cmpy = 1.0 / self.resolution.1 as f32;
        let min_x = -1.0;
        let max_x = 1.0;
        let min_y = -(self.resolution.1 as f32 / self.resolution.0 as f32);
        let max_y = self.resolution.1 as f32 / self.resolution.0 as f32;
        let x_extent = max_x - min_x;
        let y_extent = max_y - min_y;

        // Set up job queue
        let job_queue = MsQueue::new();

        // Render
        tpool.scoped(|scope| {
            // Spawn worker tasks
            for _ in 0..thread_count {
                let jq = &job_queue;
                let ajq = &all_jobs_queued;
                let img = &image;
                scope.execute(move || {
                    let mut paths = Vec::new();
                    let mut rays = Vec::new();
                    let mut tracer = Tracer::from_assembly(&self.scene.root);

                    loop {
                        paths.clear();
                        rays.clear();

                        // Get bucket, or exit if no more jobs left
                        let bucket: BucketJob;
                        loop {
                            if let Some(b) = jq.try_pop() {
                                bucket = b;
                                break;
                            } else {
                                if *ajq.read().unwrap() == true {
                                    return;
                                }
                            }
                        }

                        // Generate rays
                        for y in bucket.y..(bucket.y + bucket.h) {
                            for x in bucket.x..(bucket.x + bucket.w) {
                                let offset = hash_u32(((x as u32) << 16) ^ (y as u32), 0);
                                for si in 0..self.spp {
                                    // Calculate image plane x and y coordinates
                                    let (img_x, img_y) = {
                                        let filter_x =
                                            fast_logit(halton::sample(4, offset + si as u32), 1.5) +
                                            0.5;
                                        let filter_y =
                                            fast_logit(halton::sample(5, offset + si as u32), 1.5) +
                                            0.5;
                                        let samp_x = (filter_x + x as f32) * cmpx;
                                        let samp_y = (filter_y + y as f32) * cmpy;
                                        ((samp_x - 0.5) * x_extent, (0.5 - samp_y) * y_extent)
                                    };

                                    // Create the light path and initial ray for this sample
                                    let (path, ray) =
                                        LightPath::new(&self.scene,
                                                       (x, y),
                                                       (img_x, img_y),
                                                       (halton::sample(0, offset + si as u32),
                                                        halton::sample(1, offset + si as u32)),
                                                       halton::sample(2, offset + si as u32),
                                                       map_0_1_to_wavelength(
                                                           halton::sample(3, offset + si as u32)
                                                       ),
                                                       offset + si as u32);
                                    paths.push(path);
                                    rays.push(ray);
                                }
                            }
                        }

                        // Trace the paths!
                        let mut pi = paths.len();
                        while pi > 0 {
                            // Test rays against scene
                            let isects = tracer.trace(&rays);

                            // Determine next rays to shoot based on result
                            pi =
                                partition_pair(&mut paths[..pi], &mut rays[..pi], |i, path, ray| {
                                    path.next(&self.scene, &isects[i], &mut *ray)
                                });
                        }

                        // Calculate color based on ray hits
                        let img = img.lock().unwrap();
                        let mut img = img.borrow_mut();
                        for path in paths.iter() {
                            let mut col =
                                img.get(path.pixel_co.0 as usize, path.pixel_co.1 as usize);
                            col += path.color / self.spp as f32;
                            img.set(path.pixel_co.0 as usize, path.pixel_co.1 as usize, col);
                        }
                    }
                });
            }

            // Populate job queue
            let bucket_w = 16;
            let bucket_h = 16;
            for by in 0..((img_height / bucket_h) + 1) {
                for bx in 0..((img_width / bucket_w) + 1) {
                    let x = bx * bucket_w;
                    let y = by * bucket_h;
                    let w = min(bucket_w, img_width - x);
                    let h = min(bucket_h, img_height - y);
                    if w > 0 && h > 0 {
                        job_queue.push(BucketJob {
                            x: x as u32,
                            y: y as u32,
                            w: w as u32,
                            h: h as u32,
                        });
                    }
                }
            }

            // Mark done queuing jobs
            *all_jobs_queued.write().unwrap() = true;
        });


        // Write rendered image to disk
        {
            let img = &image.lock().unwrap();
            let _ = img.borrow().write_binary_ppm(Path::new(&self.output_file));
        }
    }
}


#[derive(Debug)]
pub struct LightPath {
    pixel_co: (u32, u32),
    lds_offset: u32,
    dim_offset: u32,
    round: u32,
    time: f32,
    wavelength: f32,
    light_attenuation: XYZ,
    color: XYZ,
}

impl LightPath {
    fn new(scene: &Scene,
           pixel_co: (u32, u32),
           image_plane_co: (f32, f32),
           lens_uv: (f32, f32),
           time: f32,
           wavelength: f32,
           lds_offset: u32)
           -> (LightPath, Ray) {
        (LightPath {
            pixel_co: pixel_co,
            lds_offset: lds_offset,
            dim_offset: 6,
            round: 0,
            time: time,
            wavelength: wavelength,
            light_attenuation: XYZ::new(1.0, 1.0, 1.0),
            color: XYZ::new(0.0, 0.0, 0.0),
        },

         scene.camera.generate_ray(image_plane_co.0,
                                   image_plane_co.1,
                                   time,
                                   lens_uv.0,
                                   lens_uv.1))
    }

    fn next_lds_samp(&mut self) -> f32 {
        let s = halton::sample(self.dim_offset, self.lds_offset);
        self.dim_offset += 1;
        s
    }

    fn next(&mut self, scene: &Scene, isect: &surface::SurfaceIntersection, ray: &mut Ray) -> bool {
        match self.round {
            // Result of camera rays, prepare light rays
            0 => {
                self.round += 1;
                if let &surface::SurfaceIntersection::Hit { t: _,
                                                            pos,
                                                            nor,
                                                            local_space: _,
                                                            uv: _ } = isect {
                    // Hit something!  Do lighting!
                    if scene.root.light_accel.len() > 0 {
                        // Get the light and the mapping to its local space
                        let (light, space) = {
                            let l1 = &scene.root.objects[scene.root.light_accel[0].data_index];
                            let light = if let &Object::Light(ref light) = l1 {
                                light
                            } else {
                                panic!()
                            };
                            let space = if let Some((start, end)) = scene.root.light_accel[0]
                                .transform_indices {
                                lerp_slice(&scene.root.xforms[start..end], self.time)
                            } else {
                                Matrix4x4::new()
                            };
                            (light, space)
                        };

                        let lu = self.next_lds_samp();
                        let lv = self.next_lds_samp();
                        // TODO: store incident light info and pdf, and use them properly
                        let (_, shadow_vec, _) =
                            light.sample(&space, pos, lu, lv, self.wavelength, self.time);

                        let rnor = if dot(nor.into_vector(), ray.dir) > 0.0 {
                            -nor.into_vector().normalized()
                        } else {
                            nor.into_vector().normalized()
                        };
                        let la = dot(rnor, shadow_vec.normalized()).max(0.0);
                        self.light_attenuation = XYZ::from_spectral_sample(&XYZ::new(la, la, la)
                            .to_spectral_sample(self.wavelength));
                        *ray = Ray::new(pos + rnor * 0.0001,
                                        shadow_vec - rnor * 0.0001,
                                        self.time,
                                        true);

                        return true;
                    } else {
                        return false;
                    }
                } else {
                    // Didn't hit anything, so background color
                    let xyz = XYZ::new(0.02, 0.02, 0.02);
                    self.color +=
                        XYZ::from_spectral_sample(&xyz.to_spectral_sample(self.wavelength));
                    return false;
                }

            }

            // Result of light rays
            1 => {
                self.round += 1;
                if let &surface::SurfaceIntersection::Miss = isect {
                    self.color += self.light_attenuation;
                }
                return false;
            }

            // TODO
            _ => unimplemented!(),
        }
    }
}


#[derive(Debug)]
struct BucketJob {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
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
