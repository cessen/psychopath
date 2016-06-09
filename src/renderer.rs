#![allow(dead_code)]

use std::path::Path;
use std::cmp::min;
use std::iter::Iterator;
use std::sync::{Mutex, RwLock};
use std::cell::RefCell;
use scoped_threadpool::Pool;
use crossbeam::sync::MsQueue;

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
                    let mut rays = Vec::new();
                    let mut pixel_mapping = Vec::new();
                    let mut tracer = Tracer::from_assembly(&self.scene.root);

                    loop {
                        rays.clear();
                        pixel_mapping.clear();

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
                                    let mut ray = {
                                        let filter_x =
                                            fast_logit(halton::sample(3, offset + si as u32), 1.5) +
                                            0.5;
                                        let filter_y =
                                            fast_logit(halton::sample(4, offset + si as u32), 1.5) +
                                            0.5;
                                        let samp_x = (filter_x + x as f32) * cmpx;
                                        let samp_y = (filter_y + y as f32) * cmpy;

                                        self.scene
                                            .camera
                                            .generate_ray((samp_x - 0.5) * x_extent,
                                                          (0.5 - samp_y) * y_extent,
                                                          halton::sample(0, offset + si as u32),
                                                          halton::sample(1, offset + si as u32),
                                                          halton::sample(2, offset + si as u32))
                                    };
                                    rays.push(ray);
                                    pixel_mapping.push((x, y))
                                }
                            }
                        }

                        // Test rays against scene
                        let isects = tracer.trace(&rays);

                        // Calculate color based on ray hits
                        let img = img.lock().unwrap();
                        let mut img = img.borrow_mut();
                        for (isect, co) in Iterator::zip(isects.iter(), pixel_mapping.iter()) {
                            let mut col = img.get(co.0 as usize, co.1 as usize);
                            if let &surface::SurfaceIntersection::Hit { t: _,
                                                                        pos: _,
                                                                        nor: _,
                                                                        local_space: _,
                                                                        uv } = isect {

                                col.0 += uv.0 / self.spp as f32;
                                col.1 += uv.1 / self.spp as f32;
                                col.2 += (1.0 - uv.0 - uv.1).max(0.0) / self.spp as f32;

                            } else {
                                col.0 += 0.02 / self.spp as f32;
                                col.1 += 0.02 / self.spp as f32;
                                col.2 += 0.02 / self.spp as f32;
                            }
                            img.set(co.0 as usize, co.1 as usize, col);
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
