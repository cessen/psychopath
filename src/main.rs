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
  psychopath <imgpath>
  psychopath (-h | --help)
  psychopath --version

Options:
  -h --help     Show this screen.
  --version     Show version.
"#;

#[derive(Debug, RustcDecodable)]
struct Args {
    arg_imgpath: String,
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

    // Generate a scene of triangles
    let mut triangles = {
        let mut triangles = Vec::new();
        let xres = 512;
        let yres = 512;
        let xinc = 512.0 / (xres as f32);
        let yinc = 512.0 / (yres as f32);
        for x in 0..xres {
            for y in 0..yres {
                let cx = x as f32 * xinc;
                let cy = y as f32 * yinc;
                triangles.push((Point::new(cx, cy, 1.0),
                                Point::new(cx + xinc, cy, 1.1),
                                Point::new(cx, cy + yinc, 1.2)));
                triangles.push((Point::new(cx + xinc, cy + yinc, 1.0),
                                Point::new(cx, cy + yinc, 1.1),
                                Point::new(cx + xinc, cy, 1.2)));
            }
        }
        triangles
    };
    let scene = bvh::BVH::from_objects(&mut triangles[..], |tri| {
        let minimum = tri.0.min(tri.1.min(tri.2));
        let maximum = tri.0.max(tri.1.max(tri.2));
        BBox {
            min: minimum,
            max: maximum,
        }
    });
    println!("Scene built.");

    // Write output image of ray-traced triangle
    let mut img = Image::new(512, 512);
    for y in 0..img.height() {
        for x in 0..img.width() {
            let mut r = 0.0;
            let mut g = 0.0;
            let mut b = 0.0;
            let offset = hash_u32(((x as u32) << 16) ^ (y as u32), 0);
            const SAMPLES: usize = 16;
            for si in 0..SAMPLES {
                let mut ray = Ray::new(Point::new(x as f32 +
                                                  fast_logit(halton::sample(0,
                                                                            offset + si as u32),
                                                             1.5),
                                                  y as f32 +
                                                  fast_logit(halton::sample(3,
                                                                            offset + si as u32),
                                                             1.5),
                                                  0.0),
                                       Vector::new(0.0, 0.0, 1.0));
                if let Some((_, u, v)) = bvh::intersect_bvh(&scene, &mut ray) {
                    r += u;
                    g += v;
                    b += (1.0 - u - v).max(0.0);
                    // r += 1.0;
                    // g += 1.0;
                    // b += 1.0;
                } else {
                    r += 0.1;
                    g += 0.1;
                    b += 0.1;
                }
            }
            r *= 255.0 / SAMPLES as f32;
            g *= 255.0 / SAMPLES as f32;
            b *= 255.0 / SAMPLES as f32;

            img.set(x, y, (r as u8, g as u8, b as u8));
        }
    }
    let _ = img.write_binary_ppm(Path::new(&args.arg_imgpath));
}
