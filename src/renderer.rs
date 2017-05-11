use std;
use std::cell::Cell;
use std::cmp;
use std::cmp::min;
use std::io::{self, Write};
use std::sync::{RwLock, Mutex};

use crossbeam::sync::MsQueue;
use scoped_threadpool::Pool;

use algorithm::partition_pair;
use accel::ACCEL_TRAV_TIME;
use color::{Color, XYZ, SpectralSample, map_0_1_to_wavelength};
use float4::Float4;
use hash::hash_u32;
use hilbert;
use image::Image;
use math::{fast_logit, upper_power_of_two};
use ray::Ray;
use sampling::halton;
use scene::Scene;
use surface;
use timer::Timer;
use tracer::Tracer;
use transform_stack::TransformStack;


#[derive(Debug)]
pub struct Renderer<'a> {
    pub output_file: String,
    pub resolution: (usize, usize),
    pub spp: usize,
    pub seed: u32,
    pub scene: Scene<'a>,
}

#[derive(Debug, Copy, Clone)]
pub struct RenderStats {
    pub trace_time: f64,
    pub accel_traversal_time: f64,
    pub initial_ray_generation_time: f64,
    pub ray_generation_time: f64,
    pub sample_writing_time: f64,
    pub total_time: f64,
}

impl RenderStats {
    fn new() -> RenderStats {
        RenderStats {
            trace_time: 0.0,
            accel_traversal_time: 0.0,
            initial_ray_generation_time: 0.0,
            ray_generation_time: 0.0,
            sample_writing_time: 0.0,
            total_time: 0.0,
        }
    }

    fn collect(&mut self, other: RenderStats) {
        self.trace_time += other.trace_time;
        self.accel_traversal_time += other.accel_traversal_time;
        self.initial_ray_generation_time += other.initial_ray_generation_time;
        self.ray_generation_time += other.ray_generation_time;
        self.sample_writing_time += other.sample_writing_time;
        self.total_time += other.total_time;
    }
}

impl<'a> Renderer<'a> {
    pub fn render(&self, max_samples_per_bucket: u32, thread_count: u32) -> (Image, RenderStats) {
        let mut tpool = Pool::new(thread_count);

        let image = Image::new(self.resolution.0, self.resolution.1);
        let (img_width, img_height) = (image.width(), image.height());

        let all_jobs_queued = RwLock::new(false);

        let collective_stats = RwLock::new(RenderStats::new());

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

        // For printing render progress
        let total_pixels = self.resolution.0 * self.resolution.1;
        let pixels_rendered = Mutex::new(Cell::new(0));
        let pixrenref = &pixels_rendered;

        // Render
        tpool.scoped(|scope| {
            // Spawn worker tasks
            for _ in 0..thread_count {
                let jq = &job_queue;
                let ajq = &all_jobs_queued;
                let img = &image;
                let cstats = &collective_stats;
                scope.execute(move || {
                    let mut stats = RenderStats::new();
                    let mut timer = Timer::new();
                    let mut total_timer = Timer::new();

                    let mut paths = Vec::new();
                    let mut rays = Vec::new();
                    let mut tracer = Tracer::from_assembly(&self.scene.root);
                    let mut xform_stack = TransformStack::new();

                    'render_loop: loop {
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
                                    break 'render_loop;
                                }
                            }
                        }

                        timer.tick();
                        // Generate light paths and initial rays
                        for y in bucket.y..(bucket.y + bucket.h) {
                            for x in bucket.x..(bucket.x + bucket.w) {
                                let offset = hash_u32(((x as u32) << 16) ^ (y as u32), self.seed);
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
                                                       map_0_1_to_wavelength(halton::sample(3,
                                                                                            offset +
                                                                                            si as
                                                                                            u32)),
                                                       offset + si as u32);
                                    paths.push(path);
                                    rays.push(ray);
                                }
                            }
                        }
                        stats.initial_ray_generation_time += timer.tick() as f64;

                        // Trace the paths!
                        let mut pi = paths.len();
                        while pi > 0 {
                            // Test rays against scene
                            let isects = tracer.trace(&rays);
                            stats.trace_time += timer.tick() as f64;

                            // Determine next rays to shoot based on result
                            pi =
                                partition_pair(&mut paths[..pi], &mut rays[..pi], |i, path, ray| {
                                    path.next(&mut xform_stack, &self.scene, &isects[i], &mut *ray)
                                });
                            stats.ray_generation_time += timer.tick() as f64;
                        }

                        // Calculate color based on ray hits and save to image
                        {
                            let min = (bucket.x, bucket.y);
                            let max = (bucket.x + bucket.w, bucket.y + bucket.h);
                            let mut img_bucket = img.get_bucket(min, max);
                            for path in paths.iter() {
                                let path_col = SpectralSample::from_parts(path.color,
                                                                          path.wavelength);
                                let mut col = img_bucket.get(path.pixel_co.0, path.pixel_co.1);
                                col += XYZ::from_spectral_sample(&path_col) / self.spp as f32;
                                img_bucket.set(path.pixel_co.0, path.pixel_co.1, col);
                            }
                            stats.sample_writing_time += timer.tick() as f64;
                        }

                        // Print render progress
                        {
                            let guard = pixrenref.lock().unwrap();
                            let mut pr = (*guard).get();
                            let percentage_old = pr as f64 / total_pixels as f64 * 100.0;

                            pr += bucket.w as usize * bucket.h as usize;
                            (*guard).set(pr);
                            let percentage_new = pr as f64 / total_pixels as f64 * 100.0;

                            let old_string = format!("{:.2}%", percentage_old);
                            let new_string = format!("{:.2}%", percentage_new);

                            if new_string != old_string {
                                print!("\r{}", new_string);
                                let _ = io::stdout().flush();
                            }
                        }
                    }

                    stats.total_time += total_timer.tick() as f64;
                    ACCEL_TRAV_TIME.with(|att| {
                        stats.accel_traversal_time = att.get();
                        att.set(0.0);
                    });

                    // Collect stats
                    cstats.write().unwrap().collect(stats);
                });
            }

            // Print initial 0.00% progress
            print!("0.00%");
            let _ = io::stdout().flush();

            // Determine bucket size based on the per-thread maximum number of samples to
            // calculate at a time.
            let (bucket_w, bucket_h) = {
                let target_pixels_per_bucket = max_samples_per_bucket as f64 / self.spp as f64;
                let target_bucket_dim = if target_pixels_per_bucket.sqrt() < 1.0 {
                    1usize
                } else {
                    target_pixels_per_bucket.sqrt() as usize
                };

                (target_bucket_dim, target_bucket_dim)
            };

            // Populate job queue
            let bucket_n = {
                let bucket_count_x = ((img_width / bucket_w) + 1) as u32;
                let bucket_count_y = ((img_height / bucket_h) + 1) as u32;
                let larger = cmp::max(bucket_count_x, bucket_count_y);
                let pow2 = upper_power_of_two(larger);
                pow2 * pow2
            };
            for hilbert_d in 0..bucket_n {
                let (bx, by) = hilbert::d2xy(hilbert_d);

                let x = bx as usize * bucket_w;
                let y = by as usize * bucket_h;
                let w = if img_width >= x {
                    min(bucket_w, img_width - x)
                } else {
                    bucket_w
                };
                let h = if img_height >= y {
                    min(bucket_h, img_height - y)
                } else {
                    bucket_h
                };
                if x < img_width && y < img_height && w > 0 && h > 0 {
                    job_queue.push(BucketJob {
                        x: x as u32,
                        y: y as u32,
                        w: w as u32,
                        h: h as u32,
                    });
                }
            }

            // Mark done queuing jobs
            *all_jobs_queued.write().unwrap() = true;
        });

        // Clear percentage progress print
        print!("\r                \r");

        // Return the rendered image and stats
        return (image, *collective_stats.read().unwrap());
    }
}


#[derive(Debug)]
pub struct LightPath {
    pixel_co: (u32, u32),
    lds_offset: u32,
    dim_offset: Cell<u32>,
    round: u32,
    time: f32,
    wavelength: f32,
    interaction: surface::SurfaceIntersection,
    light_attenuation: Float4,
    pending_color_addition: Float4,
    color: Float4,
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
             dim_offset: Cell::new(6),
             round: 0,
             time: time,
             wavelength: wavelength,
             interaction: surface::SurfaceIntersection::Miss,
             light_attenuation: Float4::splat(1.0),
             pending_color_addition: Float4::splat(0.0),
             color: Float4::splat(0.0),
         },

         scene.camera.generate_ray(image_plane_co.0,
                                   image_plane_co.1,
                                   time,
                                   lens_uv.0,
                                   lens_uv.1))
    }

    fn next_lds_samp(&self) -> f32 {
        let s = halton::sample(self.dim_offset.get(), self.lds_offset);
        let inc = self.dim_offset.get() + 1;
        self.dim_offset.set(inc);
        s
    }

    fn next(&mut self,
            xform_stack: &mut TransformStack,
            scene: &Scene,
            isect: &surface::SurfaceIntersection,
            ray: &mut Ray)
            -> bool {
        self.round += 1;

        // Result of shading ray, prepare light ray
        if self.round % 2 == 1 {
            if let &surface::SurfaceIntersection::Hit { intersection_data: ref idata,
                                                        ref closure } = isect {
                // Hit something!  Do the stuff
                self.interaction = *isect; // Store interaction for use in next phase

                // Prepare light ray
                let light_n = self.next_lds_samp();
                let light_uvw = (self.next_lds_samp(), self.next_lds_samp(), self.next_lds_samp());
                xform_stack.clear();
                if let Some((light_color, shadow_vec, light_pdf, light_sel_pdf, is_infinite)) =
                    scene.sample_lights(xform_stack,
                                        light_n,
                                        light_uvw,
                                        self.wavelength,
                                        self.time,
                                        isect) {
                    // Calculate and store the light that will be contributed
                    // to the film plane if the light is not in shadow.
                    self.pending_color_addition = {
                        let material = closure.as_surface_closure();
                        let la = material.evaluate(ray.dir, shadow_vec, idata.nor, self.wavelength);
                        light_color.e * la.e * self.light_attenuation / (light_pdf * light_sel_pdf)
                    };

                    // Calculate the shadow ray for testing if the light is
                    // in shadow or not.
                    // TODO: use proper ray offsets for avoiding self-shadowing
                    // rather than this hacky stupid stuff.
                    *ray = Ray::new(idata.pos + shadow_vec.normalized() * 0.001,
                                    shadow_vec,
                                    self.time,
                                    true);

                    // For distant lights
                    if is_infinite {
                        ray.max_t = std::f32::INFINITY;
                    }

                    return true;
                } else {
                    return false;
                }
            } else {
                // Didn't hit anything, so background color
                self.color += scene.world.background_color.to_spectral_sample(self.wavelength).e *
                              self.light_attenuation;
                return false;
            }
        }
        // Result of light ray, prepare shading ray
        else if self.round % 2 == 0 {
            // If the light was not in shadow, add it's light to the film
            // plane.
            if let &surface::SurfaceIntersection::Miss = isect {
                self.color += self.pending_color_addition;
            }

            // Calculate bounced lighting!
            if self.round < 6 {
                if let surface::SurfaceIntersection::Hit { intersection_data: ref idata,
                                                           ref closure } = self.interaction {
                    // Sample material
                    let (dir, filter, pdf) = {
                        let material = closure.as_surface_closure();
                        let u = self.next_lds_samp();
                        let v = self.next_lds_samp();
                        material.sample(idata.incoming, idata.nor, (u, v), self.wavelength)
                    };

                    // Account for the additional light attenuation from
                    // this bounce
                    self.light_attenuation *= filter.e / pdf;

                    // Calculate the ray for this bounce
                    *ray = Ray::new(idata.pos + dir.normalized() * 0.0001, dir, self.time, false);

                    return true;
                } else {
                    return false;
                }
            } else {
                return false;
            }
        } else {
            // TODO
            unimplemented!()
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
