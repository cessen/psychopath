//! This file generates the Sobol direction vectors used by this crate's
//! Sobol sequence.

use std::{env, fs::File, io::Write, path::Path};

/// How many components to generate.
const NUM_DIMENSIONS: usize = 128;

/// What file to generate the numbers from.
const DIRECTION_NUMBERS_TEXT: &str = include_str!("direction_numbers/new-joe-kuo-5.1024.txt");

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("vectors.inc");
    let mut f = File::create(&dest_path).unwrap();

    // Init direction vectors.
    let vectors = generate_direction_vectors(NUM_DIMENSIONS);

    // Write dimensions limit.
    f.write_all(format!("pub const MAX_DIMENSION: u32 = {};\n", NUM_DIMENSIONS).as_bytes())
        .unwrap();

    // Write the vectors.
    // We write them in a rather atypical way because of how the library
    // uses them.  First, we interleave the numbers of each set of four
    // dimensions, for SIMD evaluation.  Second, each number is written
    // with reversed bits, to avoid needing to reverse them before scrambling.
    f.write_all(
        format!(
            "pub const REV_VECTORS: &[[[u{0}; 4]; {0}]] = &[\n",
            SOBOL_BITS
        )
        .as_bytes(),
    )
    .unwrap();
    for d4 in vectors.chunks_exact(4) {
        f.write_all("  [\n".as_bytes()).unwrap();
        for ((a, b), (c, d)) in d4[0]
            .iter()
            .zip(d4[1].iter())
            .zip(d4[2].iter().zip(d4[3].iter()))
        {
            f.write_all(
                format!(
                    "    [0x{:08x}, 0x{:08x}, 0x{:08x}, 0x{:08x}],\n",
                    a.reverse_bits(),
                    b.reverse_bits(),
                    c.reverse_bits(),
                    d.reverse_bits()
                )
                .as_bytes(),
            )
            .unwrap();
        }
        f.write_all("  ],\n".as_bytes()).unwrap();
    }
    f.write_all("];\n".as_bytes()).unwrap();
}

//======================================================================

// The following is adapted from the code on this webpage:
//
// http://web.maths.unsw.edu.au/~fkuo/sobol/
//
// From these papers:
//
//     * S. Joe and F. Y. Kuo, Remark on Algorithm 659: Implementing Sobol's
//       quasirandom sequence generator, ACM Trans. Math. Softw. 29,
//       49-57 (2003)
//
//     * S. Joe and F. Y. Kuo, Constructing Sobol sequences with better
//       two-dimensional projections, SIAM J. Sci. Comput. 30, 2635-2654 (2008)
//
// The adapted code is under the following license:
//
//     Copyright (c) 2008, Frances Y. Kuo and Stephen Joe
//     All rights reserved.
//
//     Redistribution and use in source and binary forms, with or without
//     modification, are permitted provided that the following conditions are
//     met:
//
//       * Redistributions of source code must retain the above copyright
//         notice, this list of conditions and the following disclaimer.
//
//       * Redistributions in binary form must reproduce the above copyright
//         notice, this list of conditions and the following disclaimer in the
//         documentation and/or other materials provided with the
//         distribution.
//
//       * Neither the names of the copyright holders nor the names of the
//         University of New South Wales and the University of Waikato
//         and its contributors may be used to endorse or promote products
//         derived from this software without specific prior written
//         permission.
//
//     THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS ``AS IS'' AND ANY
//     EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
//     IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
//     PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDERS BE
//     LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
//     CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
//     SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR
//     BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
//     WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE
//     OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN
//     IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

type SobolInt = u32;
const SOBOL_BITS: usize = std::mem::size_of::<SobolInt>() * 8;

pub fn generate_direction_vectors(dimensions: usize) -> Vec<[SobolInt; SOBOL_BITS]> {
    let mut vectors = Vec::new();

    // Calculate first dimension, which is just the van der Corput sequence.
    let mut dim_0 = [0 as SobolInt; SOBOL_BITS];
    for i in 0..SOBOL_BITS {
        dim_0[i] = 1 << (SOBOL_BITS - 1 - i);
    }
    vectors.push(dim_0);

    // Do the rest of the dimensions.
    let mut lines = DIRECTION_NUMBERS_TEXT.lines();
    for _ in 1..dimensions {
        let mut v = [0 as SobolInt; SOBOL_BITS];

        // Get data from the next valid line from the direction numbers text
        // file.
        let (s, a, m) = loop {
            if let Ok((a, m)) = parse_direction_numbers(
                lines
                    .next()
                    .expect("Not enough direction numbers for the requested number of dimensions."),
            ) {
                break (m.len(), a, m);
            }
        };

        // Generate the direction numbers for this dimension.
        if SOBOL_BITS <= s as usize {
            for i in 0..SOBOL_BITS {
                v[i] = (m[i] << (SOBOL_BITS - 1 - i)) as SobolInt;
            }
        } else {
            for i in 0..(s as usize) {
                v[i] = (m[i] << (SOBOL_BITS - 1 - i)) as SobolInt;
            }

            for i in (s as usize)..SOBOL_BITS {
                v[i] = v[i - s as usize] ^ (v[i - s as usize] >> s);

                for k in 1..s {
                    v[i] ^= ((a >> (s - 1 - k)) & 1) as SobolInt * v[i - k as usize];
                }
            }
        }

        vectors.push(v);
    }

    vectors
}

/// Parses the direction numbers from a single line of the direction numbers
/// text file.  Returns the `a` and `m` parts.
fn parse_direction_numbers(text: &str) -> Result<(u32, Vec<u32>), Box<dyn std::error::Error>> {
    let mut numbers = text.split_whitespace();
    if numbers.clone().count() < 4 || text.starts_with("#") {
        return Err(Box::new(ParseError(())));
    }

    // Skip the first two numbers, which are just the dimension and the count
    // of direction numbers for this dimension.
    let _ = numbers.next().unwrap().parse::<u32>()?;
    let _ = numbers.next().unwrap().parse::<u32>()?;

    let a = numbers.next().unwrap().parse::<u32>()?;

    let mut m = Vec::new();
    for n in numbers {
        m.push(n.parse::<u32>()?);
    }

    Ok((a, m))
}

#[derive(Debug, Copy, Clone)]
struct ParseError(());
impl std::error::Error for ParseError {}
impl std::fmt::Display for ParseError {
    fn fmt(&self, _f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        Ok(())
    }
}
