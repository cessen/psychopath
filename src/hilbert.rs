#![allow(dead_code)]

const N: u32 = 1 << 16;

// Utility function used by the functions below.
fn hil_rot(n: u32, rx: u32, ry: u32, x: &mut u32, y: &mut u32) {
    use std::mem;
    if ry == 0 {
        if rx == 1 {
            *x = (n - 1).wrapping_sub(*x);
            *y = (n - 1).wrapping_sub(*y);
        }
        mem::swap(x, y);
    }
}

/// Convert (x,y) to hilbert curve index.
///
/// x: The x coordinate.  Must be a positive integer no greater than 2^16-1.
/// y: The y coordinate.  Must be a positive integer no greater than 2^16-1.
///
/// Returns the hilbert curve index corresponding to the (x,y) coordinates given.
pub fn xy2d(x: u32, y: u32) -> u32 {
    assert!(x < N);
    assert!(y < N);

    let (mut x, mut y) = (x, y);
    let mut d = 0;
    let mut s = N >> 1;
    while s > 0 {
        let rx = if (x & s) > 0 { 1 } else { 0 };
        let ry = if (y & s) > 0 { 1 } else { 0 };
        d += s * s * ((3 * rx) ^ ry);
        hil_rot(s, rx, ry, &mut x, &mut y);

        s >>= 1
    }

    d
}

/// Convert hilbert curve index to (x,y).
///
/// d: The hilbert curve index.
///
/// Returns the (x, y) coords at the given index.
pub fn d2xy(d: u32) -> (u32, u32) {
    let (mut x, mut y) = (0, 0);
    let mut s = 1;
    let mut t = d;
    while s < N {
        let rx = 1 & (t >> 1);
        let ry = 1 & (t ^ rx);
        hil_rot(s, rx, ry, &mut x, &mut y);
        x += s * rx;
        y += s * ry;
        t >>= 2;

        s <<= 1;
    }

    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reversible() {
        let d = 54;
        let (x, y) = d2xy(d);
        let d2 = xy2d(x, y);

        assert_eq!(d, d2);
    }
}
