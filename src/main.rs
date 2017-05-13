extern crate blue_noise_mask;
extern crate float4;
extern crate halton;
extern crate math3d;
extern crate mem_arena;
extern crate spectra_xyz;

extern crate crossbeam;
extern crate clap;
extern crate lodepng;
extern crate num_cpus;
extern crate openexr;
extern crate rustc_serialize;
extern crate scoped_threadpool;
extern crate time;

#[macro_use]
extern crate nom;

#[macro_use]
extern crate lazy_static;

mod accel;
mod algorithm;
mod bbox;
mod boundable;
mod camera;
mod color;
mod hash;
mod hilbert;
mod image;
mod lerp;
mod light;
mod math;
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

use std::fs::File;
use std::io;
use std::io::Read;
use std::mem;
use std::path::Path;
use std::str::FromStr;

use clap::{App, Arg};

use mem_arena::MemArena;

use parse::{parse_scene, DataTree};
use ray::{Ray, AccelRay};
use surface::SurfaceIntersection;
use renderer::LightPath;
use bbox::BBox;
use accel::BVHNode;
use timer::Timer;




const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn main() {
    let mut t = Timer::new();

    // Parse command line arguments.
    let args =
        App::new("Psychopath")
            .version(VERSION)
            .about("A slightly psychotic path tracer")
            .arg(Arg::with_name("input")
                .short("i")
                .long("input")
                .value_name("FILE")
                .help("Input .psy file")
                .takes_value(true)
                .required_unless("dev"))
            .arg(Arg::with_name("spp")
                .short("s")
                .long("spp")
                .value_name("N")
                .help("Number of samples per pixel")
                .takes_value(true)
                .validator(|s| {
                    usize::from_str(&s).and(Ok(())).or(Err("must be an integer".to_string()))
                }))
            .arg(Arg::with_name("max_bucket_samples")
                .short("b")
                .long("spb")
                .value_name("N")
                .help("Target number of samples per bucket (determines bucket size)")
                .takes_value(true)
                .validator(|s| {
                    usize::from_str(&s).and(Ok(())).or(Err("must be an integer".to_string()))
                }))
            .arg(Arg::with_name("threads")
                .short("t")
                .long("threads")
                .value_name("N")
                .help("Number of threads to render with.  Defaults to the number of logical \
                       cores on the system.")
                .takes_value(true)
                .validator(|s| {
                    usize::from_str(&s).and(Ok(())).or(Err("must be an integer".to_string()))
                }))
            .arg(Arg::with_name("stats")
                .long("stats")
                .help("Print additional statistics about rendering"))
            .arg(Arg::with_name("dev")
                .long("dev")
                .help("Show useful dev/debug info."))
            .get_matches();

    // Print some misc useful dev info.
    if args.is_present("dev") {
        println!("Ray size:       {} bytes", mem::size_of::<Ray>());
        println!("AccelRay size:  {} bytes", mem::size_of::<AccelRay>());
        println!("SurfaceIntersection size:  {} bytes",
                 mem::size_of::<SurfaceIntersection>());
        println!("LightPath size: {} bytes", mem::size_of::<LightPath>());
        println!("BBox size: {} bytes", mem::size_of::<BBox>());
        println!("BVHNode size: {} bytes", mem::size_of::<BVHNode>());
        return;
    }

    // Parse data tree of scene file
    println!("Parsing scene file...");
    t.tick();
    let mut psy_contents = String::new();
    let dt = {
        let fp = args.value_of("input").unwrap();
        let mut f = io::BufReader::new(File::open(fp).unwrap());
        let _ = f.read_to_string(&mut psy_contents);

        DataTree::from_str(&psy_contents).unwrap()
    };
    println!("\tParsed scene file in {:.3}s", t.tick());

    // Iterate through scenes and render them
    if let DataTree::Internal { ref children, .. } = dt {
        for child in children {
            t.tick();
            if child.type_name() == "Scene" {
                println!("Building scene...");

                let arena = MemArena::with_min_block_size((1 << 20) * 4);
                let mut r = parse_scene(&arena, child).unwrap_or_else(|e| {
                    e.print(&psy_contents);
                    panic!("Parse error.");
                });

                if let Some(spp) = args.value_of("spp") {
                    println!("\tOverriding scene spp: {}", spp);
                    r.spp = usize::from_str(&spp).unwrap();
                }

                let max_samples_per_bucket = if let Some(max_samples_per_bucket) =
                    args.value_of("max_bucket_samples") {
                    u32::from_str(&max_samples_per_bucket).unwrap()
                } else {
                    4096
                };

                let thread_count = if let Some(threads) = args.value_of("threads") {
                    u32::from_str(&threads).unwrap()
                } else {
                    num_cpus::get() as u32
                };

                println!("\tBuilt scene in {:.3}s", t.tick());

                println!("Rendering scene with {} threads...", thread_count);
                let (mut image, rstats) = r.render(max_samples_per_bucket, thread_count);
                // Print render stats
                {
                    let rtime = t.tick();
                    let ntime = rtime as f64 / rstats.total_time;
                    println!("\tRendered scene in {:.3}s", rtime);
                    println!("\t\tTrace:                  {:.3}s",
                             ntime * rstats.trace_time);
                    println!("\t\t\tTraversal:            {:.3}s",
                             ntime * rstats.accel_traversal_time);
                    println!("\t\tInitial ray generation: {:.3}s",
                             ntime * rstats.initial_ray_generation_time);
                    println!("\t\tRay generation:         {:.3}s",
                             ntime * rstats.ray_generation_time);
                    println!("\t\tSample writing:         {:.3}s",
                             ntime * rstats.sample_writing_time);
                }

                println!("Writing image to disk...");
                if r.output_file.ends_with(".png") {
                    let _ = image.write_png(Path::new(&r.output_file));
                } else if r.output_file.ends_with(".exr") {
                    image.write_exr(Path::new(&r.output_file));
                } else {
                    panic!("Unknown output file extension.");
                }
                println!("\tWrote image in {:.3}s", t.tick());

                // Print memory stats if stats are wanted.
                if args.is_present("stats") {
                    let arena_stats = arena.stats();
                    let mib_occupied = arena_stats.0 as f64 / 1048576.0;
                    let mib_allocated = arena_stats.1 as f64 / 1048576.0;

                    println!("MemArena stats:");

                    if mib_occupied >= 1.0 {
                        println!("\tOccupied:      {:.1} MiB", mib_occupied);
                    } else {
                        println!("\tOccupied:      {:.4} MiB", mib_occupied);
                    }

                    if mib_allocated >= 1.0 {
                        println!("\tUsed:          {:.1} MiB", mib_allocated);
                    } else {
                        println!("\tUsed:          {:.4} MiB", mib_allocated);
                    }

                    println!("\tTotal blocks:  {}", arena_stats.2);
                }
            }
        }
    }

    // End with blank line
    println!("");
}
