extern crate rustc_serialize;
extern crate docopt;

mod math;
mod lerp;
mod float4;
mod ray;
mod bbox;
mod data_tree;
mod image;

use std::path::Path;

use docopt::Docopt;

use image::Image;
use data_tree::DataTree;

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

    // Write output image
    let mut img = Image::new(512, 512);
    img.set(256, 256, (255, 255, 255));
    let _ = img.write_binary_ppm(Path::new(&args.arg_imgpath));

    let test_string = r##"
        Thing $yar { # A comment
            Obj [Things and stuff\]]
        }

        Thing $yar { # A comment
            Obj [23]
            Obj [42]
            Obj ["The meaning of life!"]
        }
    "##;
    let tree = DataTree::from_str(test_string);

    println!("{:#?}", tree);
}
