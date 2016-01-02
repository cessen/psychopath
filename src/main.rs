extern crate rustc_serialize;
extern crate docopt;

mod math;
mod algorithm;
mod lerp;
mod float4;
mod ray;
mod bbox;
mod data_tree;
mod image;
mod triangle;
mod bvh;
mod halton;

use std::mem;
use std::path::Path;

use docopt::Docopt;

use image::Image;
use math::{Point, Vector, fast_logit};
use ray::Ray;
use bbox::BBox;

// ----------------------------------------------------------------

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

const USAGE: &'static str = r#"
Psychopath <VERSION>

Usage:
  psychopath [options] <imgpath>
  psychopath (-h | --help)
  psychopath --version

Options:
  -i <input_file>       Input .psy file
  -s <n>, --spp <n>     Number of samples per pixel [default: 16].
  -h, --help            Show this screen.
  --version             Show version.
"#;

#[derive(Debug, RustcDecodable)]
struct Args {
    arg_imgpath: String,
    flag_input_file: Option<String>,
    flag_spp: Option<u32>,
    flag_version: bool,
}


// ----------------------------------------------------------------

fn hash_u32(n: u32, seed: u32) -> u32 {
    let mut hash = n;

    for _ in 0..3 {
        hash = hash.wrapping_mul(1936502639);
        hash ^= hash.wrapping_shr(16);
        hash = hash.wrapping_add(seed);
    }

    return hash;
}

fn main() {
    // Parse command line arguments.
    let args: Args = Docopt::new(USAGE.replace("<VERSION>", VERSION))
                         .and_then(|d| d.decode())
                         .unwrap_or_else(|e| e.exit());

    // Print version and exit if requested.
    if args.flag_version {
        println!("Psychopath {}", VERSION);
        return;
    }

    let samples_per_pixel = args.flag_spp.unwrap_or_else(|| 16);
    println!("Sample count: {}", samples_per_pixel);

    println!("Ray size: {} bytes", mem::size_of::<Ray>());

    // Generate a scene of triangles
    let mut triangles = {
        let mut triangles = Vec::new();
        let xres = 32;
        let yres = 32;
        let xinc = 512.0 / (xres as f32);
        let yinc = 512.0 / (yres as f32);
        for x in 0..xres {
            for y in 0..yres {
                let i = y * xres + x;
                let cx = halton::sample(0, i) * 512.0;
                let cy = halton::sample(1, i) * 512.0;
                let cz = halton::sample(2, i) * 512.0;
                triangles.push((Point::new(cx, cy, cz + 1.0),
                                Point::new(cx + xinc, cy, cz + 1.1),
                                Point::new(cx, cy + yinc, cz + 1.2)));
                triangles.push((Point::new(cx + xinc, cy + yinc, cz + 1.0),
                                Point::new(cx, cy + yinc, cz + 1.1),
                                Point::new(cx + xinc, cy, cz + 1.2)));
            }
        }
        triangles
    };
    let scene = bvh::BVH::from_objects(&mut triangles[..], 3, |tri| {
        let minimum = tri.0.min(tri.1.min(tri.2));
        let maximum = tri.0.max(tri.1.max(tri.2));
        BBox {
            min: minimum,
            max: maximum,
        }
    });
    println!("Scene built.");

    let mut rays = Vec::new();
    let mut isects = Vec::new();

    // Write output image of ray-traced triangle
    let mut img = Image::new(512, 512);
    for y in 0..img.height() {
        for x in 0..img.width() {
            let offset = hash_u32(((x as u32) << 16) ^ (y as u32), 0);

            // Generate rays
            rays.clear();
            isects.clear();
            for si in 0..samples_per_pixel {
                let mut ray = Ray::new(Point::new(0.5 + x as f32 +
                                                  fast_logit(halton::sample(0,
                                                                            offset + si as u32),
                                                             1.5),
                                                  0.5 + y as f32 +
                                                  fast_logit(halton::sample(3,
                                                                            offset + si as u32),
                                                             1.5),
                                                  0.0),
                                       Vector::new(0.0, 0.0, 1.0));
                ray.id = si as u32;
                rays.push(ray);
                isects.push((false, 0.0, 0.0));
            }

            // Test rays against scene
            scene.traverse(&mut rays, |tri, rs| {
                for r in rs {
                    if let Some((t, tri_u, tri_v)) = triangle::intersect_ray(r, *tri) {
                        if t < r.max_t {
                            isects[r.id as usize] = (true, tri_u, tri_v);
                            r.max_t = t;
                        }
                    }
                }
            });

            // Calculate color based on ray hits
            let mut r = 0.0;
            let mut g = 0.0;
            let mut b = 0.0;
            for &(hit, u, v) in isects.iter() {
                if hit {
                    r += u;
                    g += v;
                    b += (1.0 - u - v).max(0.0);
                } else {
                    r += 0.1;
                    g += 0.1;
                    b += 0.1;
                }
            }
            r *= 255.0 / samples_per_pixel as f32;
            g *= 255.0 / samples_per_pixel as f32;
            b *= 255.0 / samples_per_pixel as f32;

            // Set pixel color
            img.set(x, y, (r as u8, g as u8, b as u8));
        }
    }
    let _ = img.write_binary_ppm(Path::new(&args.arg_imgpath));
}
