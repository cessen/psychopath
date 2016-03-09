extern crate rustc_serialize;
extern crate docopt;

mod math;
mod algorithm;
mod lerp;
mod float4;
mod ray;
mod bbox;
mod camera;
mod parse;
mod renderer;
mod image;
mod triangle;
mod surface;
mod bvh;
mod scene;
mod assembly;
mod halton;

use std::mem;
use std::io;
use std::io::Read;
use std::fs::File;

use docopt::Docopt;

use math::{Point, Matrix4x4};
use ray::Ray;
use camera::Camera;
use renderer::Renderer;
use surface::triangle_mesh::TriangleMesh;
use parse::DataTree;

// ----------------------------------------------------------------

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

const USAGE: &'static str = r#"
Psychopath <VERSION>

Usage:
  psychopath [options] <imgpath>
  psychopath [options] -i <file>
  psychopath (-h | --help)
  psychopath --version

Options:
  -i <file>, --input <file>     Input .psy file
  -s <n>, --spp <n>             Number of samples per pixel [default: 16].
  -h, --help                    Show this screen.
  --version                     Show version.
"#;

#[derive(Debug, RustcDecodable)]
struct Args {
    arg_imgpath: String,
    flag_input: Option<String>,
    flag_spp: Option<u32>,
    flag_version: bool,
}


// ----------------------------------------------------------------

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

    // =======================
    // Print tree from psy file if passed as an argument.
    // TODO: remove this, because it's for debugging
    if let Some(fp) = args.flag_input {
        let mut f = io::BufReader::new(File::open(fp).unwrap());
        let mut s = String::new();
        let _ = f.read_to_string(&mut s);
        let dt = DataTree::from_str(&s);
        println!("{:#?}", dt);
        return;
    }
    // =======================

    let samples_per_pixel = args.flag_spp.unwrap_or_else(|| 16);
    println!("Sample count: {}", samples_per_pixel);

    println!("Ray size: {} bytes", mem::size_of::<Ray>());

    // Generate a scene of triangles
    let mesh = TriangleMesh::from_triangles(2, {
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
                // let cx = x as f32 * xinc;
                // let cy = y as f32 * yinc;
                // let cz = 1.0;
                triangles.push((Point::new(cx, cy, cz + 1.0),
                                Point::new(cx + xinc, cy, cz + 1.1),
                                Point::new(cx, cy + yinc, cz + 1.2)));
                triangles.push((Point::new(cx + 25.0, cy, cz + 1.0),
                                Point::new(cx + 25.0 + xinc, cy, cz + 1.1),
                                Point::new(cx + 25.0, cy + yinc, cz + 1.2)));
            }
        }
        triangles
    });
    println!("Scene built.");

    let cam = Camera::new(vec![Matrix4x4::from_location(Point::new(256.0, 256.0, -1024.0))],
                          vec![0.785],
                          vec![20.0],
                          vec![1026.0]);

    let r = Renderer {
        output_file: args.arg_imgpath.clone(),
        resolution: (512, 512),
        spp: samples_per_pixel as usize,
        camera: cam,
        scene: mesh,
    };

    r.render();
}
