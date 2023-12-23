use std::{ptr, fmt::format, io, ffi::{c_int, c_ulong, c_uint}};
use x11_dl::xlib::{self, Xlib, _XDisplay};

pub static mut XLIB: *mut Xlib = ptr::null_mut();

macro_rules! def_xcall {
    ( $call: ident (
        $( $param_name:ident : $param_type:ty ),*
    ) -> enum $error_ty_name:ident : $return_ty:ty {
        $( $error_name:ident = $error_val:expr ),*
    }) => {
        #[repr(transparent)]
        #[derive(Debug, PartialEq, Eq, Copy, Clone)]
        enum $error_ty_name : $return_ty {
            $( $error_name = $error_val, )*
        }
        #[allow(non_snake_case)]
        pub fn $call( $( $param_name: $param_type  ),* ) -> $error_ty_name { unsafe { ((*XLIB).$call)( $( $param_name ),* ) } }
    };

    ( $call: ident ( $( $param_name:ident : $param_type:ty ),* ) -> $return_ty:ty ) => {
        #[allow(non_snake_case)]
        pub fn $call( $( $param_name: $param_type  ),* ) -> $return_ty { unsafe { ((*XLIB).$call)( $( $param_name ),* ) } }
    };
}

// macro_rules! def_xcall {
//     ( $call: ident ( $( $param_name:ident : $param_type:ty ),* ) -> $return_ty:ty ) => {
//         #[allow(non_snake_case)]
//         pub fn $call( $( $param_name: $param_type  ),* ) -> $return_ty { unsafe { ((*XLIB).$call)( $( $param_name ),* ) } }
//     }
// }

def_xcall!( XOpenDisplay(display : *const i8) -> *mut _XDisplay );
def_xcall!( XCreateSimpleWindow(
    display: *mut _XDisplay,
    parent: c_ulong,
    x: c_int,
    y: c_int,
    width: c_uint,
    height: c_uint,
    border_width: c_uint,
    border: c_ulong,
    background: c_ulong
) -> enum XCreateSimpleWindowError : c_ulong {
    BadAlloc = xlib::BadAlloc,
    BadMatch = xlib::BadMatch,
    BadValue = xlib::BadValue,
    BadWindow = xlib::BadWindow
} );
def_xcall!( XRootWindow(display: *mut _XDisplay, screen_number: c_int ) -> c_ulong );
def_xcall!( XBlackPixel(display: *mut _XDisplay, screen_number: c_int ) -> c_ulong );
def_xcall!( XWhitePixel(display: *mut _XDisplay, screen_number: c_int ) -> c_ulong );

pub type PlatformResult = std::io::Result<()>;

pub fn platform_main() -> PlatformResult {
    let xlib = Xlib::open().map_err(|e| io::Error::new(io::ErrorKind::Other, format!("failed to open the xlib library: {:?}", e)))?;
    unsafe { *XLIB = xlib };
    let display = XOpenDisplay(ptr::null());
    if display.is_null() {
          return Err(io::Error::new(io::ErrorKind::Other, format!("failed to get the default display")));
    }

    BadAlloc // , BadMatch, BadValue, and BadWindow
    let window = XCreateSimpleWindow(
        display,
        XRootWindow(display, 0),
        0, 0,
        400, 300,
        1,
        XBlackPixel(display, 0),
        XWhitePixel(display, 0));

    Ok(())
}
