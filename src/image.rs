#![allow(dead_code)]

use std::io;
use std::io::Write;
use std::path::Path;
use std::fs::File;

#[derive(Debug, Clone)]
pub struct Image {
    data: Vec<(u8, u8, u8)>,
    res: (usize, usize),
}

impl Image {
    pub fn new(width: usize, height: usize) -> Image {
        Image {
            data: vec![(0,0,0); width * height],
            res: (width, height),
        }
    }

    pub fn width(&self) -> usize {
        self.res.0
    }

    pub fn height(&self) -> usize {
        self.res.1
    }

    pub fn get(&self, x: usize, y: usize) -> (u8, u8, u8) {
        assert!(x < self.res.0);
        assert!(y < self.res.1);

        self.data[self.res.0 * y + x]
    }

    pub fn set(&mut self, x: usize, y: usize, value: (u8, u8, u8)) {
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
                let d = [r, g, b];
                try!(f.write_all(&d));
            }
        }

        // Done
        Ok(())
    }
}
