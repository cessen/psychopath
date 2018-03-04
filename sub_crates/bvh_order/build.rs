// Generate table for traversal order of quad BVHs.

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn main() {
    // Build the traversal table.
    let mut traversal_table = [
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ];
    for raydir in 0..8 {
        let ray = [raydir & 1, (raydir >> 1) & 1, (raydir >> 2) & 1];

        for s2 in 0..3 {
            for s1 in 0..3 {
                for s0 in 0..3 {
                    let mut perm = [0, 1, 2, 3];
                    if ray[s1] == 1 {
                        perm.swap(0, 1);
                    }
                    if ray[s2] == 1 {
                        perm.swap(2, 3);
                    }
                    if ray[s0] == 1 {
                        perm.swap(0, 2);
                        perm.swap(1, 3);
                    }
                    traversal_table[raydir]
                        .push(perm[0] + (perm[1] << 2) + (perm[2] << 4) + (perm[3] << 6));
                }
            }
        }

        for s1 in 0..3 {
            for s0 in 0..3 {
                let mut perm = [0, 1, 2];
                if ray[s1] == 1 {
                    perm.swap(0, 1);
                }
                if ray[s0] == 1 {
                    perm.swap(0, 1);
                    perm.swap(0, 2);
                }
                traversal_table[raydir].push(perm[0] + (perm[1] << 2) + (perm[2] << 4));
            }
        }

        for s1 in 0..3 {
            for s0 in 0..3 {
                let mut perm = [0, 1, 2];
                if ray[s1] == 1 {
                    perm.swap(1, 2);
                }
                if ray[s0] == 1 {
                    perm.swap(0, 2);
                    perm.swap(0, 1);
                }
                traversal_table[raydir].push(perm[0] + (perm[1] << 2) + (perm[2] << 4));
            }
        }

        for s0 in 0..3 {
            let mut perm = [0, 1];
            if ray[s0] == 1 {
                perm.swap(0, 1);
            }
            traversal_table[raydir].push(perm[0] + (perm[1] << 2));
        }
    }

    // Write traversal table to Rust file
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("table_inc.rs");
    let mut f = File::create(&dest_path).unwrap();

    f.write_all("pub static TRAVERSAL_TABLE: [[u8; 48]; 8] = [".as_bytes())
        .unwrap();

    for sub_table in traversal_table.iter() {
        f.write_all("\n    [".as_bytes()).unwrap();
        for (i, n) in sub_table.iter().enumerate() {
            if i == 27 || i == 36 || i == 45 {
                f.write_all("\n     ".as_bytes()).unwrap();
            }
            f.write_all(format!("{}", n).as_bytes()).unwrap();
            if i != 47 {
                f.write_all(", ".as_bytes()).unwrap();
            }
        }
        f.write_all("],".as_bytes()).unwrap();
    }

    f.write_all("\n];".as_bytes()).unwrap();
}
