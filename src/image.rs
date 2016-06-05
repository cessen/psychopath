#![allow(dead_code)]

use std::io;
use std::io::Write;
use std::path::Path;
use std::fs::File;

#[derive(Debug, Clone)]
pub struct Image {
    data: Vec<(f32, f32, f32)>,
    res: (usize, usize),
}

impl Image {
    pub fn new(width: usize, height: usize) -> Image {
        Image {
            data: vec![(0.0, 0.0, 0.0); width * height],
            res: (width, height),
        }
    }

    pub fn width(&self) -> usize {
        self.res.0
    }

    pub fn height(&self) -> usize {
        self.res.1
    }

    pub fn get(&self, x: usize, y: usize) -> (f32, f32, f32) {
        assert!(x < self.res.0);
        assert!(y < self.res.1);

        self.data[self.res.0 * y + x]
    }

    pub fn set(&mut self, x: usize, y: usize, value: (f32, f32, f32)) {
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
                let (r, g, b) = self.get(x, y);
                let r = quantize_255(srgb_gamma(r));
                let g = quantize_255(srgb_gamma(g));
                let b = quantize_255(srgb_gamma(b));
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
                let (r, g, b) = self.get(x, y);
                let r = quantize_255(srgb_gamma(r));
                let g = quantize_255(srgb_gamma(g));
                let b = quantize_255(srgb_gamma(b));
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

fn quantize_255(n: f32) -> u8 {
    let n = 1.0f32.min(0.0f32.max(n)) * 255.0;
    n as u8
}
