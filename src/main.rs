extern crate rustc_serialize;
extern crate docopt;
#[macro_use]
extern crate nom;

mod math;
mod algorithm;
mod lerp;
mod float4;
mod ray;
mod bbox;
mod camera;
mod parse;
mod renderer;
mod tracer;
mod image;
mod boundable;
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

use ray::Ray;
use parse::{parse_scene, DataTree};

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

    // Parse data tree of scene file
    let mut s = String::new();
    let dt = if let Some(fp) = args.flag_input {
        let mut f = io::BufReader::new(File::open(fp).unwrap());
        let _ = f.read_to_string(&mut s);

        DataTree::from_str(&s).unwrap()
    } else {
        panic!()
    };


    // Generate a scene of triangles
    // let mesh = TriangleMesh::from_triangles(2, {
    // let mut triangles = Vec::new();
    // let xres = 32;
    // let yres = 32;
    // let xinc = 512.0 / (xres as f32);
    // let yinc = 512.0 / (yres as f32);
    // for x in 0..xres {
    // for y in 0..yres {
    // let i = y * xres + x;
    // let cx = halton::sample(0, i) * 512.0;
    // let cy = halton::sample(1, i) * 512.0;
    // let cz = halton::sample(2, i) * 512.0;
    // let cx = x as f32 * xinc;
    // let cy = y as f32 * yinc;
    // let cz = 1.0;
    // triangles.push((Point::new(cx, cy, cz + 1.0),
    // Point::new(cx + xinc, cy, cz + 1.1),
    // Point::new(cx, cy + yinc, cz + 1.2)));
    // triangles.push((Point::new(cx + 25.0, cy, cz + 1.0),
    // Point::new(cx + 25.0 + xinc, cy, cz + 1.1),
    // Point::new(cx + 25.0, cy + yinc, cz + 1.2)));
    // }
    // }
    // triangles
    // });
    //
    // let cam = Camera::new(vec![Matrix4x4::from_location(Point::new(256.0, 256.0, -1024.0))],
    // vec![0.785],
    // vec![20.0],
    // vec![1026.0]);
    //
    // let mut assembly_b = AssemblyBuilder::new();
    // assembly_b.add_object("yar", Object::Surface(Box::new(mesh)));
    // assembly_b.add_object_instance("yar",
    // Some(&[Matrix4x4::from_location(Point::new(25.0, 0.0, 0.0))]));
    // let assembly = assembly_b.build();
    //
    // let scene = Scene {
    // name: None,
    // background_color: (0.0, 0.0, 0.0),
    // camera: cam,
    // root: assembly,
    // };
    //
    // let r = Renderer {
    // output_file: args.arg_imgpath.clone(),
    // resolution: (512, 512),
    // spp: samples_per_pixel as usize,
    // scene: scene,
    // };
    //
    println!("Scene built.");

    let samples_per_pixel = args.flag_spp.unwrap_or_else(|| 16);
    println!("Sample count: {}", samples_per_pixel);

    println!("Ray size: {} bytes", mem::size_of::<Ray>());

    // Iterate through scenes and render them
    if let DataTree::Internal{ref children, ..} = dt {
        for child in children {
            if child.type_name() == "Scene" {
                println!("Parsing scene...");
                let r = parse_scene(child).unwrap();
                println!("Rendering scene...");
                r.render();
            }
        }
    }
}
