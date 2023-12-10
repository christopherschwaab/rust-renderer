use std::{
    ptr,
    mem::{self, MaybeUninit}, ffi, alloc,
};
use log::trace;
use windows::{
    core::{self, PCWSTR},
    Win32::{
        Graphics::Gdi::{self, GetDC, BITMAPINFO, BI_RGB, BITMAPINFOHEADER, StretchDIBits},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            WindowsAndMessaging::{
                self,
                WNDCLASSW,
                CS_OWNDC,
                CS_HREDRAW,
                CS_VREDRAW,
                RegisterClassW,
                CreateWindowExW,
                WINDOW_EX_STYLE,
                WS_OVERLAPPEDWINDOW,
                WS_VISIBLE,
                CW_USEDEFAULT,
                HMENU,
                PeekMessageW,
                TranslateMessage,
                DispatchMessageW,
                WM_KEYUP,
                WM_KEYDOWN,
                DefWindowProcW,
                SW_SHOW,
                PM_REMOVE,
                WM_QUIT,
                WM_DESTROY,
                WM_CLOSE,
                WM_PAINT,
                WM_ACTIVATEAPP,
            },
            Input::KeyboardAndMouse,
        },
        Foundation::{HWND, LRESULT, WPARAM, LPARAM}
    },
};

mod platform_lib;
use platform_lib::*;

const INITIAL_WIDTH: i32 = 1024;
const INITIAL_HEIGHT: i32 = 768;

extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM
) -> LRESULT {
    let ctx: Option<&mut AppWindowContext> = unsafe {
        let ctx = mem::transmute::<isize, *mut AppWindowContext>(WindowsAndMessaging::GetWindowLongPtrW(hwnd, WindowsAndMessaging::GWLP_USERDATA));
        if ctx.is_null() { None } else { Some(&mut *ctx) }
    };

    match msg {
        WM_DESTROY => {
            ctx.map(|ctx| ctx.running = false);
            LRESULT(0)
        },
        WM_CLOSE => {
            ctx.map(|ctx| ctx.running = false);
            LRESULT(0)
        },
        WM_PAINT => {
            let ctx = ctx.unwrap();
            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: FB_WIDTH,
                    biHeight: FB_HEIGHT,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB as u32,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut paint: MaybeUninit<Gdi::PAINTSTRUCT> = MaybeUninit::uninit();
            let (device_ctx, paint) = unsafe {
                let device_ctx = Gdi::BeginPaint(hwnd, paint.as_mut_ptr());
                (device_ctx, paint.assume_init())
            };
            unsafe { StretchDIBits(device_ctx, 0, 0, ctx.width, ctx.height, 0, 0, FB_WIDTH, FB_HEIGHT, ctx.pixels as *const ffi::c_void, &bmi, Gdi::DIB_RGB_COLORS, Gdi::SRCCOPY) };

            unsafe { Gdi::EndPaint(hwnd, &paint) };

            // let window_dimension = WindowDimension::from_window(hwnd);
            // unsafe {
            //     APP_CONTEXT.update_window(device_context, window_dimension.width, window_dimension.height);
            //     Gdi::EndPaint(hwnd, &paint);
            // };
            LRESULT(0)
        },
        WM_KEYDOWN |
        WM_KEYUP => {
            let vkcode = KeyboardAndMouse::VIRTUAL_KEY(wparam.0 as u16);
            let _is_down = (lparam.0 as u32 & (1 << 31)) == 0;
            // TODO(chris): decide on keymap and modify emu input state
            match vkcode {
                _ => {},
            };
            LRESULT(0)
        },
        WM_ACTIVATEAPP => {
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

pub type PlatformResult = core::Result<()>;

#[derive(Debug)]
struct AppWindowContext {
    pixels: *mut u32,
    width: i32,
    height: i32,
    running: bool,
}

const FB_WIDTH: i32 = 800;
const FB_HEIGHT: i32 = 600;

pub fn platform_main() -> PlatformResult {
    simple_logger::init_with_env().unwrap();

    unsafe {
        let title = "tinyrenderer-window";
        let wc_name = to_pcwstr("tinyrenderer-window-class");

        let hinst = GetModuleHandleW(<PCWSTR as Default>::default())?;
        let wc = WNDCLASSW {
            style: CS_OWNDC | CS_HREDRAW | CS_VREDRAW,
            hInstance: hinst,
            lpszClassName: wc_name,
            lpfnWndProc: Some(window_proc),
            cbWndExtra: mem::size_of::<AppWindowContext>() as i32,
            ..Default::default()
        };
        let window_class_atom = RegisterClassW(&wc);
        if window_class_atom == 0 {
            return Err(log_win32_error("failed to register window class"));
        }

        let main_window = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            wc_name,
            title,
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            INITIAL_WIDTH,
            INITIAL_HEIGHT,
            <HWND as Default>::default(),
            HMENU(0),
            hinst,
            ptr::null()
        );
        if main_window == Default::default() {
            return Err(log_win32_error("failed to create main window"));
        }

        trace!("got main window {:?}", main_window);
        let layout = alloc::Layout::from_size_align(FB_WIDTH as usize * FB_HEIGHT as usize * mem::size_of::<u32>(), mem::align_of::<u32>()).unwrap();
        let app_window_ctx = &mut AppWindowContext { running: true, width: INITIAL_WIDTH, height: INITIAL_HEIGHT, pixels: alloc::alloc(layout) as *mut u32 } as *mut AppWindowContext;
        WindowsAndMessaging::SetWindowLongPtrW(main_window, WindowsAndMessaging::GWLP_USERDATA, mem::transmute::<*mut AppWindowContext, isize>(app_window_ctx));

        let device_ctx = GetDC(main_window);

        // APP_CONTEXT.resize_dib_section(INITIAL_WIDTH, INITIAL_HEIGHT);
        let mut msg = Default::default();

        if !WindowsAndMessaging::ShowWindow(main_window, SW_SHOW).as_bool() {
           return Err(core::Error::from_win32());
        }

        //(*app_window_ctx).pixels = (*app_window_ctx).pixels.offset(10 * FB_WIDTH as isize);
        for _ in 0..(FB_WIDTH * FB_HEIGHT) {
            *(*app_window_ctx).pixels = 0x00ff0000;
            (*app_window_ctx).pixels = (*app_window_ctx).pixels.offset(1);
            // for _ in 0..FB_WIDTH {
            //     //*(*app_window_ctx).pixels = 0x00ff0000;
            //     //(*app_window_ctx).pixels = (*app_window_ctx).pixels.offset(1);
            // }
            // //(*app_window_ctx).pixels = (*app_window_ctx).pixels.offset(FB_WIDTH as isize);
        }

        while (*app_window_ctx).running {
            while (*app_window_ctx).running && PeekMessageW(&mut msg, HWND(0), 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    (*app_window_ctx).running = false;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: FB_WIDTH,
                    biHeight: FB_HEIGHT,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB as u32,
                    ..Default::default()
                },
                ..Default::default()
            };
            unsafe { StretchDIBits(device_ctx, 0, 0, (*app_window_ctx).width, (*app_window_ctx).height, 0, 0, FB_WIDTH, FB_HEIGHT, (*app_window_ctx).pixels as *const ffi::c_void, &bmi, Gdi::DIB_RGB_COLORS, Gdi::SRCCOPY) };

        }

        //while APP_CONTEXT.running {
        //    while PeekMessageW(&mut msg, HWND(0), 0, 0, PM_REMOVE).as_bool() {
        //        if msg.message == WM_QUIT {
        //            APP_CONTEXT.running = false;
        //        }
        //        TranslateMessage(&msg);
        //        DispatchMessageW(&msg);
        //    }

        //    let window_dimension = WindowDimension::from_window(main_window);
        //    let mut p = APP_CONTEXT.bitmap_memory.as_mut_ptr();

        //    //let t: Triangle<u32, 2> = [[10, 100], [50, 50], [100, 100]].into();
        //    let t: Triangle<u32, 2> = [[0, 0], [0, 10], [10, 0]].into();
        //    let bb@[Coord([min_x, min_y]), Coord([max_x, max_y])] = t.bounding_box();

        //    //println!("p={:?}, bb={:?}, width={:?}", p, bb, APP_CONTEXT.bitmap_memory.width);
        //    let p_base = p;
        //    p = p.offset((min_y as i32 * APP_CONTEXT.bitmap_memory.width) as isize);
        //    //for y in min_y..max_y  {
        //    //    println!("{:?}", p as usize - p_base as usize);
        //    //    for x in min_x..max_x {
        //    //        unsafe {
        //    //            *p = 0xff;
        //    //            p = p.offset(1);
        //    //        }
        //    //    }
        //    //    p = p.offset(APP_CONTEXT.bitmap_memory.width as isize);
        //    //}
        //    //panic!("meow");


        //    p = p.offset(1024 / 4 * 10);
        //    for x in 0..100 {
        //        unsafe {
        //            *p = Pixel::rgb(255, 0, 0);
        //            p = p.offset(1);
        //        }
        //    }

        //    // for y in 0..100 {
        //    //     for x in 0..30 {
        //    //         unsafe {
        //    //             *p = Pixel::rgb(255, 0, 0);
        //    //             p = p.offset(1);
        //    //         }
        //    //     }
        //    //     p = p.offset(APP_CONTEXT.bitmap_memory.width as isize / size_of::<Pixel>() as isize);
        //    // }

        //    //unsafe {
        //    //    for _ in 0..APP_CONTEXT.bitmap_memory.height {
        //    //        for _ in 0..APP_CONTEXT.bitmap_memory.width {
        //    //            //let color = if (x + y) % 2 == 0 {
        //    //            //    0x00ff_ffff
        //    //            //} else {
        //    //            //    0x00ff_0000
        //    //            //};
        //    //            *p = Pixel::packed_rgb(0xff0000);
        //    //            p = p.offset(1);
        //    //        }
        //    //    }
        //    //}
        //    println!("{}, {}, {}, {}", APP_CONTEXT.bitmap_memory.width, APP_CONTEXT.bitmap_memory.height, window_dimension.width, window_dimension.height);
        //    APP_CONTEXT.update_window(device_context, window_dimension.width, window_dimension.height);
        //}
    }

    Ok(())
}
