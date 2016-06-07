extern crate rustc_serialize;
extern crate docopt;
extern crate scoped_threadpool;
extern crate crossbeam;
extern crate num_cpus;
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
mod color;

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
  psychopath [options] -i <file>
  psychopath (-h | --help)
  psychopath --version

Options:
  -i <file>, --input <file>     Input .psy file.
  -s <n>, --spp <n>             Number of samples per pixel.
  -t <n>, --threads <n>         Number of threads to render with.  Defaults
                                to the number of logical cores on the system.
  -h, --help                    Show this screen.
  --version                     Show version.
"#;

#[derive(Debug, RustcDecodable)]
struct Args {
    flag_input: Option<String>,
    flag_spp: Option<usize>,
    flag_threads: Option<usize>,
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

    println!("Ray size: {} bytes", mem::size_of::<Ray>());

    // Iterate through scenes and render them
    if let DataTree::Internal { ref children, .. } = dt {
        for child in children {
            if child.type_name() == "Scene" {
                println!("Parsing scene...");
                let mut r = parse_scene(child).unwrap();

                if let Some(spp) = args.flag_spp {
                    println!("Overriding scene spp: {}", spp);
                    r.spp = spp;
                }

                let thread_count = if let Some(threads) = args.flag_threads {
                    threads as u32
                } else {
                    num_cpus::get() as u32
                };

                println!("Rendering scene with {} threads...", thread_count);
                r.render(thread_count);
            }
        }
    }
}
