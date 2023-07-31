use std::{fmt, io::{self, prelude::*}, iter::Iterator, ops::Shl, slice::Iter};

pub trait ColorSpace {
    fn new() -> Self;
    const BPP: u8;
}

#[derive(Copy, Clone, Debug)]
#[repr(packed)]
pub struct Grayscale {
    pub i: u8
}

#[derive(Copy, Clone, Debug)]
#[repr(packed)]
pub struct Rgb {
    pub b: u8,
    pub g: u8,
    pub r: u8,
}

#[derive(Copy, Clone, Debug)]
#[repr(packed)]
pub struct Rgba {
    pub b: u8,
    pub g: u8,
    pub r: u8,
    pub a: u8,
}

impl ColorSpace for Grayscale {
    fn new() -> Self {
        Grayscale { i: 0 }
    }
    const BPP: u8 = 1;
}

impl ColorSpace for Rgb {
    fn new() -> Self {
        Rgb { b: 0, g: 0, r: 0 }
    }
    const BPP: u8 = 3;
}

impl ColorSpace for Rgba {
    fn new() -> Self {
        Rgba { b: 0, g: 0, r: 0, a: 0 }
    }
    const BPP: u8 = 4;
}

pub struct Image<T: ColorSpace> {
    pub data: Vec<T>,
    pub width: u16,
    pub height: u16,
}

#[derive(Copy, Clone, Debug)]
pub enum Error {
    InvalidDimensions,
    InvalidData,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::InvalidDimensions => write!(f, "Invalid dimensions"),
            Error::InvalidData => write!(f, "Invalid data"),
        }
    }
}

unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    std::slice::from_raw_parts((p as *const T) as *const u8, std::mem::size_of::<T>())
}

struct PixelsIter<'a, T: ColorSpace> {
    iter: Iter<'a, T>
}

impl<'a, T: ColorSpace> Iterator for PixelsIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

//impl<'a, T: ColorSpace> IntoIterator for Image<T> {
//    type Item = &'a T;
//    type IntoIter = Iterator<'a, T>;
//
//    fn into_iter(self) -> Self::IntoIter {
//        self.data.into_iter()
//    }
//}

//impl<'a, T: ColorSpace> Iterator for PixelsIter<'a, T> {
//    type Item = T;
//
//    fn next(&mut self) -> Option<Self::Item> {
//    }
//}

impl<T: ColorSpace + Copy> Image<T> {
    pub fn new(width: u16, height: u16) -> Self {
        Image {
            width,
            height,
            data: vec![T::new(); width as usize * height as usize],
        }
    }

    pub fn set(&mut self, x: u16, y: u16, color: T) -> Result<(), Error> {
        //if x >= self.width || y >= self.height {
        //    println!("Invalid dimensions: {}, {}", x, y);
        //    return Err(Error::InvalidDimensions);
        //}
        let ix = self.pixel_offset(x, y);
        self.data[ix] = color;
        Ok(())
    }

    fn pixel_offset(&self, x: u16, y: u16) -> usize {
        (y as usize * self.width as usize + x as usize).into()
    }

    fn data_vec(&self) -> Vec<u8> {
        self.data
            .iter()
            .flat_map(|p| unsafe { any_as_u8_slice(p) })
            .copied()
            .collect::<Vec<u8>>()
    }


    pub fn write<W: io::Write>(&self, writer: &mut io::BufWriter<W>, vflip: bool, rle: bool) -> io::Result<()> {
        let header = Header {
            idlength: 0,
            bitsperpixel: T::BPP.shl(3),
            width: self.width,
            height: self.height,
            colormaptype: 0,
            datatypecode: if T::BPP == Grayscale::BPP {
                if rle { 9 } else { 3 }
            } else {
                if rle { 10 } else { 2 }
            },
            imagedescriptor: if vflip { 0 } else { 0x20 },
            ..Default::default()
        };

        writer.write_all(unsafe { any_as_u8_slice(&header) })?;
        if !rle {
            writer.write_all(&self.data_vec().as_slice())?;
        } else {
            todo!();
        }

        writer.write_all(&DEVELOPER_AREA_REF)?;
        writer.write_all(&EXTENSION_AREA_REF)?;
        writer.write_all(FOOTER)?;
        writer.flush()?;

        Ok(())
    }
}

const DEVELOPER_AREA_REF: [u8; 4] = [0, 0, 0, 0];
const EXTENSION_AREA_REF: [u8; 4] = [0, 0, 0, 0];
const FOOTER: &[u8; 18] = b"TRUEVISION-XFILE.\0";

#[derive(Default)]
#[repr(packed)]
#[allow(dead_code)]
struct Header {
    idlength: u8,
    colormaptype: u8,
    datatypecode: u8,
    colormaporigin: u16,
    colormaplength: u16,
    colormapdepth: u8,
    x_origin: u16,
    y_origin: u16,
    width: u16,
    height: u16,
    bitsperpixel: u8,
    imagedescriptor: u8,
}
