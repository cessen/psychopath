#![allow(dead_code)]

use std::io;
use std::io::Write;
use std::path::Path;
use std::fs::File;

use color::{XYZ, xyz_to_rec709e};

#[derive(Debug, Clone)]
pub struct Image {
    data: Vec<XYZ>,
    res: (usize, usize),
}

impl Image {
    pub fn new(width: usize, height: usize) -> Image {
        Image {
            data: vec![XYZ::new(0.0, 0.0, 0.0); width * height],
            res: (width, height),
        }
    }

    pub fn width(&self) -> usize {
        self.res.0
    }

    pub fn height(&self) -> usize {
        self.res.1
    }

    pub fn get(&self, x: usize, y: usize) -> XYZ {
        assert!(x < self.res.0);
        assert!(y < self.res.1);

        self.data[self.res.0 * y + x]
    }

    pub fn set(&mut self, x: usize, y: usize, value: XYZ) {
        assert!(x < self.res.0);
        assert!(y < self.res.1);

        self.data[self.res.0 * y + x] = value;
    }

    pub fn write_ascii_ppm(&self, path: &Path) -> io::Result<()> {
        // Open file.
        let mut f = io::BufWriter::new(try!(File::create(path)));

        // Write header
        try!(write!(f, "P3\n{} {}\n255\n", self.res.0, self.res.1));

        // Write pixels
        for y in 0..self.res.1 {
            for x in 0..self.res.0 {
                let (r, g, b) = quantize_tri_255(xyz_to_srgbe(self.get(x, y).to_tuple()));
                try!(write!(f, "{} {} {} ", r, g, b));
            }
            try!(write!(f, "\n"));
        }

        // Done
        Ok(())
    }

    pub fn write_binary_ppm(&self, path: &Path) -> io::Result<()> {
        // Open file.
        let mut f = io::BufWriter::new(try!(File::create(path)));

        // Write header
        try!(write!(f, "P6\n{} {}\n255\n", self.res.0, self.res.1));

        // Write pixels
        for y in 0..self.res.1 {
            for x in 0..self.res.0 {
                let (r, g, b) = quantize_tri_255(xyz_to_srgbe(self.get(x, y).to_tuple()));
                let d = [r, g, b];
                try!(f.write_all(&d));
            }
        }

        // Done
        Ok(())
    }
}

fn srgb_gamma(n: f32) -> f32 {
    if n < 0.0031308 {
        n * 12.92
    } else {
        (1.055 * n.powf(1.0 / 2.4)) - 0.055
    }
}

fn srgb_inv_gamma(n: f32) -> f32 {
    if n < 0.04045 {
        n / 12.92
    } else {
        ((n + 0.055) / 1.055).powf(2.4)
    }
}

fn xyz_to_srgbe(xyz: (f32, f32, f32)) -> (f32, f32, f32) {
    let rgb = xyz_to_rec709e(xyz);
    (srgb_gamma(rgb.0), srgb_gamma(rgb.1), srgb_gamma(rgb.2))
}

fn quantize_tri_255(tri: (f32, f32, f32)) -> (u8, u8, u8) {
    fn quantize(n: f32) -> u8 {
        let n = 1.0f32.min(0.0f32.max(n)) * 255.0;
        n as u8
    }

    (quantize(tri.0), quantize(tri.1), quantize(tri.2))
}
