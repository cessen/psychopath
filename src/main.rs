#![allow(clippy::float_cmp)]
#![allow(clippy::inline_always)]
#![allow(clippy::many_single_char_names)]
#![allow(clippy::needless_lifetimes)]
#![allow(clippy::needless_return)]
#![allow(clippy::or_fun_call)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::redundant_field_names)]
#![allow(clippy::enum_variant_names)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::excessive_precision)]
#![allow(clippy::transmute_ptr_to_ptr)]

extern crate lazy_static;

mod accel;
mod algorithm;
mod bbox;
mod bbox4;
mod boundable;
mod camera;
mod color;
mod fp_utils;
mod hash;
mod hilbert;
mod image;
mod lerp;
mod light;
mod math;
mod mis;
mod parse;
mod ray;
// mod renderer;
mod sampling;
mod scene;
mod shading;
mod surface;
mod timer;
// mod tracer;
mod transform_stack;

use std::{fs::File, io, io::Read, mem, path::Path, str::FromStr};

use clap::{App, Arg};
use nom::bytes::complete::take_until;

use kioku::Arena;

use data_tree::{
    reader::{DataTreeReader, ReaderError},
    Event,
};

use crate::{
    accel::BVH4Node,
    bbox::BBox,
    parse::{parse_scene, DataTree},
    // renderer::LightPath,
    surface::SurfaceIntersection,
    timer::Timer,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[allow(clippy::cognitive_complexity)]
fn main() {
    let mut t = Timer::new();

    // Parse command line arguments.
    let args = App::new("Psychopath")
        .version(VERSION)
        .about("A slightly psychotic path tracer")
        .arg(
            Arg::with_name("input")
                .short("i")
                .long("input")
                .value_name("FILE")
                .help("Input .psy file")
                .takes_value(true)
                .required_unless_one(&["dev", "use_stdin"]),
        )
        .arg(
            Arg::with_name("spp")
                .short("s")
                .long("spp")
                .value_name("N")
                .help("Number of samples per pixel")
                .takes_value(true)
                .validator(|s| {
                    usize::from_str(&s)
                        .and(Ok(()))
                        .or(Err("must be an integer".to_string()))
                }),
        )
        .arg(
            Arg::with_name("max_bucket_samples")
                .short("b")
                .long("spb")
                .value_name("N")
                .help("Target number of samples per bucket (determines bucket size)")
                .takes_value(true)
                .validator(|s| {
                    usize::from_str(&s)
                        .and(Ok(()))
                        .or(Err("must be an integer".to_string()))
                }),
        )
        .arg(
            Arg::with_name("crop")
                .long("crop")
                .value_name("X1 Y1 X2 Y2")
                .help(
                    "Only render the image between pixel coordinates (X1, Y1) \
                     and (X2, Y2).  Coordinates are zero-indexed and inclusive.",
                )
                .takes_value(true)
                .number_of_values(4)
                .validator(|s| {
                    usize::from_str(&s)
                        .and(Ok(()))
                        .or(Err("must be four integers".to_string()))
                }),
        )
        .arg(
            Arg::with_name("threads")
                .short("t")
                .long("threads")
                .value_name("N")
                .help(
                    "Number of threads to render with.  Defaults to the number of logical \
                     cores on the system.",
                )
                .takes_value(true)
                .validator(|s| {
                    usize::from_str(&s)
                        .and(Ok(()))
                        .or(Err("must be an integer".to_string()))
                }),
        )
        .arg(
            Arg::with_name("stats")
                .long("stats")
                .help("Print additional statistics about rendering"),
        )
        .arg(
            Arg::with_name("dev")
                .long("dev")
                .help("Show useful dev/debug info."),
        )
        .arg(
            Arg::with_name("serialized_output")
                .long("serialized_output")
                .help("Serialize and send render output to standard output.")
                .hidden(true),
        )
        .arg(
            Arg::with_name("use_stdin")
                .long("use_stdin")
                .help("Take scene file in from stdin instead of a file path.")
                .hidden(true),
        )
        .get_matches();

    // Print some misc useful dev info.
    if args.is_present("dev") {
        println!(
            "SurfaceIntersection size:  {} bytes",
            mem::size_of::<SurfaceIntersection>()
        );
        // println!("LightPath size: {} bytes", mem::size_of::<LightPath>());
        println!("BBox size: {} bytes", mem::size_of::<BBox>());
        // println!("BVHNode size: {} bytes", mem::size_of::<BVHNode>());
        println!("BVH4Node size: {} bytes", mem::size_of::<BVH4Node>());
        return;
    }

    let crop = args.values_of("crop").map(|mut vals| {
        let coords = (
            u32::from_str(vals.next().unwrap()).unwrap(),
            u32::from_str(vals.next().unwrap()).unwrap(),
            u32::from_str(vals.next().unwrap()).unwrap(),
            u32::from_str(vals.next().unwrap()).unwrap(),
        );
        if coords.0 > coords.2 {
            panic!("Argument '--crop': X1 must be less than or equal to X2");
        }
        if coords.1 > coords.3 {
            panic!("Argument '--crop': Y1 must be less than or equal to Y2");
        }
        coords
    });

    // Parse data tree of scene file
    if !args.is_present("serialized_output") {
        println!("Parsing scene file...",);
    }
    t.tick();
    // let psy_contents = if args.is_present("use_stdin") {
    //     // Read from stdin
    //     let mut input = Vec::new();
    //     let tmp = std::io::stdin();
    //     let mut stdin = tmp.lock();
    //     let mut buf = vec![0u8; 4096];
    //     loop {
    //         let count = stdin
    //             .read(&mut buf)
    //             .expect("Unexpected end of scene input.");
    //         let start = if input.len() < 11 {
    //             0
    //         } else {
    //             input.len() - 11
    //         };
    //         let end = input.len() + count;
    //         input.extend(&buf[..count]);

    //         let mut done = false;
    //         let mut trunc_len = 0;
    //         if let nom::IResult::Ok((remaining, _)) =
    //             take_until::<&str, &[u8], ()>("__PSY_EOF__")(&input[start..end])
    //         {
    //             done = true;
    //             trunc_len = input.len() - remaining.len();
    //         }
    //         if done {
    //             input.truncate(trunc_len);
    //             break;
    //         }
    //     }
    //     String::from_utf8(input).unwrap()
    // } else {
    //     // Read from file
    //     let mut input = String::new();
    //     let fp = args.value_of("input").unwrap();
    //     let mut f = io::BufReader::new(File::open(fp).unwrap());
    //     let _ = f.read_to_string(&mut input);
    //     input
    // };

    // let dt = DataTree::from_str(&psy_contents).unwrap();
    // if !args.is_present("serialized_output") {
    //     println!("\tParsed scene file in {:.3}s", t.tick());
    // }

    let mut psy_file = io::BufReader::new(File::open(fp).unwrap());
    let mut events = DataTreeReader::new(&mut psy_file);

    // Iterate through scenes and render them
    loop {
        t.tick();
        match events.next_event() {
            Ok(Event::ValidEnd) => {
                break;
            }

            Ok(_) => {
                println!("Error: invalid scene in psy file.");
                break;
            }

            Err(e) => {
                println!("Error: {:?}", e);
                break;
            }

            // Parse a scene and render it.
            Ok(Event::InnerOpen {
                type_name: "Scene",
                ident,
                ..
            }) => {
                if !args.is_present("serialized_output") {
                    println!("Building scene...");
                }

                let arena = Arena::new().with_block_size((1 << 20) * 4);
                let ident = ident.into::<String>();
                let mut scene = parse_scene(&arena, &mut events, &ident).unwrap_or_else(|e| {
                    e.print(&psy_contents);
                    panic!("Parse error.");
                });

                if let Some(spp) = args.value_of("spp") {
                    if !args.is_present("serialized_output") {
                        println!("\tOverriding scene spp: {}", spp);
                    }
                    // r.spp = usize::from_str(spp).unwrap();
                }

                let max_samples_per_bucket =
                    if let Some(max_samples_per_bucket) = args.value_of("max_bucket_samples") {
                        u32::from_str(max_samples_per_bucket).unwrap()
                    } else {
                        4096
                    };

                let thread_count = if let Some(threads) = args.value_of("threads") {
                    u32::from_str(threads).unwrap()
                } else {
                    num_cpus::get() as u32
                };

                if !args.is_present("serialized_output") {
                    println!("\tBuilt scene in {:.3}s", t.tick());
                }

                if !args.is_present("serialized_output") {
                    println!("Rendering scene with {} threads...", thread_count);
                }

                println!("{:#?}", scene);

                println!("Didn't really render, because all that code is disabled!  Done!");
                // let (mut image, rstats) = r.render(
                //     max_samples_per_bucket,
                //     crop,
                //     thread_count,
                //     args.is_present("serialized_output"),
                // );
                // // Print render stats
                // if !args.is_present("serialized_output") {
                //     let rtime = t.tick();
                //     let ntime = rtime as f64 / rstats.total_time;
                //     println!("\tRendered scene in {:.3}s", rtime);
                //     println!(
                //         "\t\tTrace:                  {:.3}s",
                //         ntime * rstats.trace_time
                //     );
                //     println!("\t\t\tRays traced:          {}", rstats.ray_count);
                //     println!(
                //         "\t\t\tRays/sec:             {}",
                //         (rstats.ray_count as f64 / (ntime * rstats.trace_time) as f64) as u64
                //     );
                //     println!("\t\t\tRay/node tests:       {}", rstats.accel_node_visits);
                //     println!(
                //         "\t\tInitial ray generation: {:.3}s",
                //         ntime * rstats.initial_ray_generation_time
                //     );
                //     println!(
                //         "\t\tRay generation:         {:.3}s",
                //         ntime * rstats.ray_generation_time
                //     );
                //     println!(
                //         "\t\tSample writing:         {:.3}s",
                //         ntime * rstats.sample_writing_time
                //     );
                // }

                // // Write to disk
                // if !args.is_present("serialized_output") {
                //     println!("Writing image to disk into '{}'...", r.output_file);
                //     if r.output_file.ends_with(".png") {
                //         image
                //             .write_png(Path::new(&r.output_file))
                //             .expect("Failed to write png...");
                //     } else if r.output_file.ends_with(".exr") {
                //         image.write_exr(Path::new(&r.output_file));
                //     } else {
                //         panic!("Unknown output file extension.");
                //     }
                //     println!("\tWrote image in {:.3}s", t.tick());
                // }

                // // Print memory stats if stats are wanted.
                // if args.is_present("stats") {
                //     let arena_stats = arena.stats();
                //     let mib_occupied = arena_stats.0 as f64 / 1_048_576.0;
                //     let mib_allocated = arena_stats.1 as f64 / 1_048_576.0;

                //     println!("MemArena stats:");

                //     if mib_occupied >= 1.0 {
                //         println!("\tOccupied:      {:.1} MiB", mib_occupied);
                //     } else {
                //         println!("\tOccupied:      {:.4} MiB", mib_occupied);
                //     }

                //     if mib_allocated >= 1.0 {
                //         println!("\tUsed:          {:.1} MiB", mib_allocated);
                //     } else {
                //         println!("\tUsed:          {:.4} MiB", mib_allocated);
                //     }

                //     println!("\tTotal blocks:  {}", arena_stats.2);
                // }
            }
        }
    }

    // End with blank line
    println!();
}
