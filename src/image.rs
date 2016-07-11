#![allow(dead_code)]

use std::io;
use std::io::Write;
use std::path::Path;
use std::fs::File;
use std::marker::PhantomData;
use std::sync::Mutex;
use std::cell::{RefCell, UnsafeCell};
use std::mem;
use std::cmp;

use lodepng;

use color::{XYZ, xyz_to_rec709e};

#[derive(Debug)]
pub struct Image {
    data: UnsafeCell<Vec<XYZ>>,
    res: (usize, usize),
    checked_out_blocks: Mutex<RefCell<Vec<((u32, u32), (u32, u32))>>>, // (min, max)
}

unsafe impl Sync for Image {}

impl Image {
    pub fn new(width: usize, height: usize) -> Image {
        Image {
            data: UnsafeCell::new(vec![XYZ::new(0.0, 0.0, 0.0); width * height]),
            res: (width, height),
            checked_out_blocks: Mutex::new(RefCell::new(Vec::new())),
        }
    }

    pub fn width(&self) -> usize {
        self.res.0
    }

    pub fn height(&self) -> usize {
        self.res.1
    }

    pub fn get(&mut self, x: usize, y: usize) -> XYZ {
        assert!(x < self.res.0);
        assert!(y < self.res.1);

        let data: &Vec<XYZ> = unsafe { mem::transmute(self.data.get()) };
        data[self.res.0 * y + x]
    }

    pub fn set(&mut self, x: usize, y: usize, value: XYZ) {
        assert!(x < self.res.0);
        assert!(y < self.res.1);

        let data: &mut Vec<XYZ> = unsafe { mem::transmute(self.data.get()) };
        data[self.res.0 * y + x] = value;
    }

    pub fn get_bucket<'a>(&'a self, min: (u32, u32), max: (u32, u32)) -> Bucket<'a> {
        let tmp = self.checked_out_blocks.lock().unwrap();
        let mut bucket_list = tmp.borrow_mut();

        // Make sure this won't overlap with any already checked out buckets
        for bucket in bucket_list.iter() {
            // Calculate the intersection between the buckets
            let inter_min = (cmp::max(min.0, (bucket.0).0), cmp::max(min.1, (bucket.0).1));
            let inter_max = (cmp::min(max.0, (bucket.1).0), cmp::min(max.1, (bucket.1).1));

            // If it's not degenerate and not zero-sized, there's overlap, so
            // panic.
            if inter_min.0 < inter_max.0 && inter_min.1 < inter_max.1 {
                panic!("Attempted to check out a bucket with pixels that are already checked out.");
            }
        }

        // Clip bucket to image
        let max = (cmp::min(max.0, self.res.0 as u32), cmp::min(max.1, self.res.1 as u32));

        // Push bucket onto list
        bucket_list.push((min, max));

        Bucket {
            min: min,
            max: max,
            img: unsafe { mem::transmute(self as *const Image) },
            _phantom: PhantomData,
        }
    }

    pub fn write_ascii_ppm(&mut self, path: &Path) -> io::Result<()> {
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

    pub fn write_binary_ppm(&mut self, path: &Path) -> io::Result<()> {
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

    pub fn write_png(&mut self, path: &Path) -> io::Result<()> {
        let mut image = Vec::new();

        // Convert pixels
        for y in 0..self.res.1 {
            for x in 0..self.res.0 {
                let (r, g, b) = quantize_tri_255(xyz_to_srgbe(self.get(x, y).to_tuple()));
                let d = lodepng::RGB::new(r, g, b);
                image.push(d);
            }
        }

        // Write file
        if let Err(_) = lodepng::encode_file(path,
                                             &image,
                                             self.res.0,
                                             self.res.1,
                                             lodepng::ColorType::LCT_RGB,
                                             8) {
            panic!("Couldn't write PNG file.");
        }

        // Done
        Ok(())
    }
}

#[derive(Debug)]
pub struct Bucket<'a> {
    min: (u32, u32),
    max: (u32, u32),
    img: *mut Image,
    _phantom: PhantomData<&'a Image>,
}

impl<'a> Bucket<'a> {
    pub fn get(&mut self, x: u32, y: u32) -> XYZ {
        assert!(x >= self.min.0 && x < self.max.0);
        assert!(y >= self.min.1 && y < self.max.1);

        let img: &mut Image = unsafe { mem::transmute(self.img) };
        let data: &Vec<XYZ> = unsafe { mem::transmute(img.data.get()) };

        data[img.res.0 * y as usize + x as usize]
    }

    pub fn set(&mut self, x: u32, y: u32, value: XYZ) {
        assert!(x >= self.min.0 && x < self.max.0);
        assert!(y >= self.min.1 && y < self.max.1);

        let img: &mut Image = unsafe { mem::transmute(self.img) };
        let data: &mut Vec<XYZ> = unsafe { mem::transmute(img.data.get()) };

        data[img.res.0 * y as usize + x as usize] = value;
    }
}

impl<'a> Drop for Bucket<'a> {
    fn drop(&mut self) {
        let img: &mut Image = unsafe { mem::transmute(self.img) };
        let tmp = img.checked_out_blocks.lock().unwrap();
        let mut bucket_list = tmp.borrow_mut();

        // Find matching bucket and remove it
        let i = bucket_list.iter().position(|bucket| {
            (bucket.0).0 == self.min.0 && (bucket.0).1 == self.min.1 &&
            (bucket.1).0 == self.max.0 && (bucket.1).1 == self.max.1
        });
        bucket_list.swap_remove(i.unwrap());
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
