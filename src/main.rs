extern crate rustc_serialize;
extern crate docopt;

mod math;
mod lerp;
mod float4;
mod ray;
mod bbox;
mod data_tree;
mod image;
mod triangle;

use std::path::Path;

use docopt::Docopt;

use image::Image;
use data_tree::DataTree;
use math::{Point, Vector};
use ray::Ray;

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

    // Write output image of ray-traced triangle
    let p1 = Point::new(10.0, 80.0, 1.0);
    let p2 = Point::new(420.0, 40.0, 1.0);
    let p3 = Point::new(235.0, 490.0, 1.0);
    let mut img = Image::new(512, 512);
    for y in 0..img.height() {
        for x in 0..img.width() {
            let ray = Ray::new(Point::new(x as f32, y as f32, 0.0),
                               Vector::new(0.0, 0.0, 1.0));
            if let Some((_, u, v)) = triangle::intersect_ray(&ray, (p1, p2, p3)) {
                let r = (u * 255.0) as u8;
                let g = (v * 255.0) as u8;
                let b = ((1.0 - u - v) * 255.0).max(0.0) as u8;
                img.set(x, y, (r, g, b));
            }
        }
    }
    let _ = img.write_binary_ppm(Path::new(&args.arg_imgpath));

    let test_string = r##"
        Thing $yar { # A comment
            Obj [Things and stuff\]]
        }

        Thing { # A comment
            Obj [23]
            Obj [42]
            Obj ["The meaning of life!"]
        }
    "##;
    let tree = DataTree::from_str(test_string);

    println!("{:#?}", tree);
}
