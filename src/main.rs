extern crate crossbeam;
extern crate docopt;
extern crate lodepng;
extern crate num_cpus;
extern crate openexr;
extern crate rustc_serialize;
extern crate scoped_threadpool;
extern crate time;

#[macro_use]
extern crate nom;

#[cfg(feature = "simd_perf")]
extern crate simd;

mod algorithm;
mod assembly;
mod bbox;
mod boundable;
mod bvh;
mod camera;
mod color;
mod float4;
mod halton;
mod hash;
mod hilbert;
mod image;
mod lerp;
mod light_accel;
mod light;
mod math;
mod objects_split;
mod parse;
mod ray;
mod renderer;
mod sampling;
mod scene;
mod shading;
mod surface;
mod timer;
mod tracer;
mod transform_stack;
mod triangle;

use std::fs::File;
use std::io;
use std::io::Read;
use std::mem;
use std::path::Path;

use docopt::Docopt;

use parse::{parse_scene, DataTree};
use ray::{Ray, AccelRay};
use renderer::LightPath;
use timer::Timer;


// ----------------------------------------------------------------

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

const USAGE: &'static str = r#"
Psychopath <VERSION>

Usage:
  psychopath [options] -i <file>
  psychopath --dev
  psychopath (-h | --help)
  psychopath --version

Options:
  -i <file>, --input <file>     Input .psy file.
  -s <n>, --spp <n>             Number of samples per pixel.
  -t <n>, --threads <n>         Number of threads to render with.  Defaults
                                to the number of logical cores on the system.
  --dev                         Show useful dev/debug info.
  -h, --help                    Show this screen.
  --version                     Show version.
"#;

#[derive(Debug, RustcDecodable)]
struct Args {
    flag_input: Option<String>,
    flag_spp: Option<usize>,
    flag_threads: Option<usize>,
    flag_dev: bool,
    flag_version: bool,
}


// ----------------------------------------------------------------

fn main() {
    let mut t = Timer::new();

    // Parse command line arguments.
    let args: Args = Docopt::new(USAGE.replace("<VERSION>", VERSION))
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());

    // Print version and exit if requested.
    if args.flag_version {
        println!("Psychopath {}", VERSION);
        return;
    }

    // Print some misc useful dev info.
    if args.flag_dev {
        println!("Ray size:       {} bytes", mem::size_of::<Ray>());
        println!("AccelRay size:  {} bytes", mem::size_of::<AccelRay>());
        println!("LightPath size: {} bytes", mem::size_of::<LightPath>());
        return;
    }

    // Parse data tree of scene file
    println!("Parsing scene file...");
    t.tick();
    let mut s = String::new();
    let dt = if let Some(fp) = args.flag_input {
        let mut f = io::BufReader::new(File::open(fp).unwrap());
        let _ = f.read_to_string(&mut s);

        DataTree::from_str(&s).unwrap()
    } else {
        panic!()
    };
    println!("\tParsed scene file in {:.3}s", t.tick());


    // Iterate through scenes and render them
    if let DataTree::Internal { ref children, .. } = dt {
        for child in children {
            t.tick();
            if child.type_name() == "Scene" {
                println!("Building scene...");
                let mut r = parse_scene(child).unwrap();

                if let Some(spp) = args.flag_spp {
                    println!("\tOverriding scene spp: {}", spp);
                    r.spp = spp;
                }

                let thread_count = if let Some(threads) = args.flag_threads {
                    threads as u32
                } else {
                    num_cpus::get() as u32
                };

                println!("\tBuilt scene in {:.3}s", t.tick());

                println!("Rendering scene with {} threads...", thread_count);
                let mut image = r.render(thread_count);
                println!("\tRendered scene in {:.3}s", t.tick());

                println!("Writing image to disk...");
                if r.output_file.ends_with(".png") {
                    let _ = image.write_png(Path::new(&r.output_file));
                } else if r.output_file.ends_with(".exr") {
                    image.write_exr(Path::new(&r.output_file));
                } else {
                    panic!("Unknown output file extension.");
                }
                println!("\tWrote image in {:.3}s", t.tick());
            }
        }
    }
}
