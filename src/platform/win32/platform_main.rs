use std::{
    ptr,
    mem::{self, MaybeUninit},
    ffi,
    hint::black_box,
    time::SystemTime,
};
use jordan_tinyrenderer::Coord;
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
                WM_ACTIVATEAPP, DestroyWindow, PostQuitMessage,
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
            unsafe { DestroyWindow(hwnd) };
            LRESULT(0)
        },
        WM_CLOSE => {
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        },
        WM_PAINT => {
            let mut paint: MaybeUninit<Gdi::PAINTSTRUCT> = MaybeUninit::uninit();
            let (hdc, paint) = unsafe {
                let hdc = Gdi::BeginPaint(hwnd, paint.as_mut_ptr());
                (hdc, paint.assume_init())
            };

            let ctx = ctx.unwrap();
            unsafe {
                StretchDIBits(
                    hdc,
                    0, 0, INITIAL_WIDTH, INITIAL_HEIGHT,
                    0, 0, ctx.width, ctx.height,
                    ctx.pixels.as_ptr() as *const ffi::c_void,
                    &ctx.bitmap_info,
                    Gdi::DIB_RGB_COLORS,
                    Gdi::SRCCOPY);

                Gdi::EndPaint(hwnd, &paint)
            };
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
    bitmap_info: BITMAPINFO,
    pixels: Vec<u32>,
    width: i32,
    height: i32,
    running: bool,
}

pub fn platform_main() -> PlatformResult {
    let title = "tinyrenderer-window";
    let wc_name = to_pcwstr("tinyrenderer-window-class");

    let main_module = unsafe { GetModuleHandleW(PCWSTR(ptr::null())) }?;
    let wc = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW | CS_OWNDC,
        hInstance: main_module,
        lpszClassName: wc_name,
        lpfnWndProc: Some(window_proc),
        cbWndExtra: mem::size_of::<*mut AppWindowContext>() as i32,
        ..Default::default()
    };
    if unsafe { RegisterClassW(&wc) } == 0 {
        return Err(core::Error::from_win32());
    }

    let main_window = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            wc_name,
            title,
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            INITIAL_WIDTH,
            INITIAL_HEIGHT,
            HWND(0),
            HMENU(0),
            main_module,
            ptr::null()
        )
    };

    let mut app_window_ctx = AppWindowContext {
        pixels: vec![0; INITIAL_WIDTH as usize * INITIAL_HEIGHT as usize],
        width: INITIAL_WIDTH,
        height: INITIAL_HEIGHT,
        running: true,
        bitmap_info: BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: INITIAL_WIDTH,
                biHeight: -INITIAL_HEIGHT,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB as u32,
                ..Default::default()
            },
            ..Default::default()
        }
    };
    unsafe { WindowsAndMessaging::SetWindowLongPtrW(main_window, WindowsAndMessaging::GWLP_USERDATA, mem::transmute::<*mut AppWindowContext, isize>(&mut app_window_ctx as *mut AppWindowContext)) };

    let hdc = unsafe { GetDC(main_window) };
    let mut msg = Default::default();

    if !unsafe { WindowsAndMessaging::ShowWindow(main_window, SW_SHOW) }.as_bool() {
        return Err(core::Error::from_win32());
    }

    const FOCAL_LENGTH: f32 = 5.0;
    let observer_position: Coord<f32, 3> = Coord([0.0, 0.0, 1.0]);

    let mut wrote_to_fb = false;
    unsafe {
        let mut x0 = 0;
        let mut last_update = SystemTime::UNIX_EPOCH;

        while app_window_ctx.running {
            // let now = SystemTime::now();
            // if match now.duration_since(last_update) {
            //     Ok(d) => d.as_millis() > 16,
            //     Err(_) => false,
            // } {
            //     for y in 10..100 {
            //         for x in x0..x0+10 {
            //             app_window_ctx.pixels[y * INITIAL_WIDTH as usize + x] = 0xff000000;
            //         }
            //         for x in x0+10..x0+100 {
            //             app_window_ctx.pixels[y * INITIAL_WIDTH as usize + x] = 0xff0000ff;
            //         }
            //     }
            //     last_update = now;
            //     x0 = (x0 + 1) % (INITIAL_WIDTH - 100) as usize;
            // }

            while PeekMessageW(&mut msg, HWND(0), 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    app_window_ctx.running = false;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
                // NOTE(chris): I have no idea if this is correct or required
                // but am too lazy to investigate how rust handles ffi and want
                // it to know roughly that the context is used by
                // dispatchmessage.
                black_box(&mut app_window_ctx);
            }

            if !wrote_to_fb {
                const VIEWSCREEN_WIDTH: f32 = 16.0;
                const VIEWSCREEN_HEIGHT: f32 = 9.0;

                println!("drawing to fb");
                jordan_tinyrenderer::update_fb(
                    &mut app_window_ctx.pixels,
                    INITIAL_WIDTH as u32,
                    INITIAL_HEIGHT as u32,
                    VIEWSCREEN_WIDTH as f32,
                    VIEWSCREEN_HEIGHT as f32,
                    &observer_position,
                    FOCAL_LENGTH);
                wrote_to_fb = true;
            }
            StretchDIBits(
                hdc,
                0, 0, INITIAL_WIDTH, INITIAL_HEIGHT,
                0, 0, app_window_ctx.width, app_window_ctx.height,
                app_window_ctx.pixels.as_ptr() as *const ffi::c_void,
                &app_window_ctx.bitmap_info,
                Gdi::DIB_RGB_COLORS,
                Gdi::SRCCOPY);
        }
    }

    Ok(())
}
