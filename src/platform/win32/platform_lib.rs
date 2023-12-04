use std::{
    alloc,
    ptr,
    mem::{MaybeUninit, self},
    ffi,
    marker::PhantomData,
};
use log::error;
use windows::{
    core::{self, PCWSTR, IntoParam},
    Win32::{
        Graphics::Gdi::{self, HDC, StretchDIBits},
        UI::WindowsAndMessaging::GetClientRect,
        Foundation::{HWND, RECT}
    }
};

pub fn to_wstring<S: AsRef<str>>(s: S) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    std::ffi::OsStr::new(s.as_ref())
        .encode_wide()
        .chain(Some(0).into_iter())
        .collect()
}

pub fn to_pcwstr<S: AsRef<str>>(s: S) -> PCWSTR {
    PCWSTR(to_wstring(s).as_ptr())
}

pub struct WindowDimension {
   pub  width: i32,
   pub  height: i32,
}

impl WindowDimension {
    fn from_rect(rect: RECT) -> Self {
        Self {
            width: rect.right - rect.left,
            height: rect.bottom - rect.top,
        }
    }

    pub fn from_window(hwnd: HWND) -> Self {
        let window_rect = unsafe {
            let mut window_rect: MaybeUninit<RECT> = MaybeUninit::uninit();
            if !GetClientRect(hwnd, window_rect.as_mut_ptr()).as_bool() {
                // TODO something
            }
            window_rect.assume_init()
        };
        Self::from_rect(window_rect)
    }
}

pub struct MutableBitmapMemoryPixelIter<'a> {
    buf: ptr::NonNull<Pixel>,
    end: *mut Pixel,
    _r: PhantomData<&'a mut Pixel>,
}

impl<'a> Iterator for MutableBitmapMemoryPixelIter<'a> {
    type Item = &'a mut Pixel;

    fn next(&mut self) -> Option<Self::Item> {
        if self.buf.as_ptr() == self.end {
            None
        } else {
            let p = self.buf.as_ptr();
            let pixel = unsafe { &mut *p };
            self.buf = unsafe { ptr::NonNull::new_unchecked(p.wrapping_offset(1)) };
            Some(pixel)
        }
    }
}

//pub fn perspective_projection(p: Coord3, focal_length: f32) -> Coord2 {
//    // (1 - t)*A + t*B, t in [0, 1]
//}

pub struct MutableBitmapMemory {
    pub width: i32,
    pub height: i32,
    pub data: *mut u8,
}

#[repr(transparent)]
pub struct Pixel(u32);

impl Pixel {
    pub fn packed_rgb(rgb: u32) -> Self {
        Self(rgb)
    }

    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self((r as u32) << 16 | (g as u32) << 8 | b as u32)
    }
}

impl Drop for MutableBitmapMemory {
    fn drop(&mut self) {
        if !self.as_mut_ptr().is_null() {
            unsafe { alloc::dealloc(self.data, Self::layout(self.width, self.height)) }
        }
    }
}

pub struct MutableBitmapMemorySize{
    width: i32,
    height: i32,
}

impl MutableBitmapMemory {
    pub fn with_size(width: i32, height: i32) -> Self {
        let data = unsafe { alloc::alloc(Self::layout(width, height)) };

        Self {
            width,
            height,
            data
        }
    }

    fn layout(width: i32, height: i32) -> alloc::Layout {
        alloc::Layout::from_size_align(Self::mem_size(width, height), mem::align_of::<Pixel>()).expect("invalid layout")
    }

    fn mem_size(width: i32, height: i32) -> usize {
        width as usize * height as usize * mem::size_of::<Pixel>()
    }

    pub fn len(&self) -> usize {
        self.width as usize * self.height as usize
    }

    pub fn as_mut_ptr(&mut self) -> *mut Pixel {
        self.data as *mut Pixel
    }

    pub fn iter_mut(&mut self) -> MutableBitmapMemoryPixelIter {
        MutableBitmapMemoryPixelIter {
            buf: unsafe { ptr::NonNull::new_unchecked(self.data.cast()) },
            end: self.data.wrapping_offset(Self::mem_size(self.width, self.height) as isize).cast(),
            _r: PhantomData,
        }
    }

    //pub fn blit8(fb: &[u8], width: i32, height: i32) {
    //    let y_scale = self.height / height;
    //    //let max_col = height * scale
    //    for y in 0..height {
    //        let col_ix = y * width;
    //        for x in 0..width {
    //            let pixel = fb[(col_ix + x) as usize];
    //            let pixel = if pixel == 0 { Pixel::rgb(0, 0, 0) } else { Pixel::rgb(255, 255, 255) };
    //            unsafe {
    //                *Self::get_pixel_ptr(x, y, width, height) = pixel;
    //            }
    //        }
    //    }
    //}
}


pub struct AppContext {
    pub running: bool,
    bitmap_info: Gdi::BITMAPINFO,
    pub bitmap_memory: MutableBitmapMemory,
}

impl AppContext {
    pub fn resize_dib_section(&mut self, width: i32, height: i32) {
        const BITS_PER_BYTE: u16 = 8;

        self.bitmap_info = Gdi::BITMAPINFO {
            bmiHeader: Gdi::BITMAPINFOHEADER {
                biSize: mem::size_of::<Gdi::BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: height,
                biPlanes: 1,
                biBitCount: mem::size_of::<Pixel>() as u16 * BITS_PER_BYTE,
                biCompression: Gdi::BI_RGB as u32,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [Default::default()],
        };

        self.bitmap_memory = MutableBitmapMemory::with_size(width, height);
    }

    pub fn update_window<'a>(&mut self, device_context: impl IntoParam<'a, HDC>, window_width: i32, window_height: i32) {
        unsafe {
            let _ = StretchDIBits(
                device_context,
                0, 0, window_width, window_height,
                0, 0, self.bitmap_memory.width, self.bitmap_memory.height,
                self.bitmap_memory.as_mut_ptr() as *mut ffi::c_void,
                &self.bitmap_info,
                Gdi::DIB_RGB_COLORS,
                Gdi::SRCCOPY);
        }
    }
}

pub static mut APP_CONTEXT: AppContext = AppContext {
    running: true,
    bitmap_info: Gdi::BITMAPINFO {
        bmiHeader: Gdi::BITMAPINFOHEADER {
            biSize: mem::size_of::<Gdi::BITMAPINFOHEADER>() as u32,
            biWidth: 0,
            biHeight: 0,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: Gdi::BI_RGB as u32,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [Gdi::RGBQUAD {
            rgbBlue: 0,
            rgbGreen: 0,
            rgbRed: 0,
            rgbReserved: 0,
        }],
    },
    bitmap_memory: MutableBitmapMemory {
        width: 0,
        height: 0,
        data: ptr::null_mut(),
    },
};

pub fn log_win32_error(msg: &str) -> core::Error {
    let e = core::Error::from_win32();
    error!("{}: {}", msg, e);
    e
}
