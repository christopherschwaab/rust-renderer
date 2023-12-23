use std::{
    ptr,
    mem::{self, MaybeUninit},
    ffi,
    hint::black_box,
};
use jordan_tinyrenderer::{Coord, obj};
use windows::{
    core::{self, PCWSTR},
    Win32::{
        Graphics::Gdi::{self, GetDC, BITMAPINFO, BI_RGB, BITMAPINFOHEADER, StretchDIBits},
        System::{LibraryLoader::GetModuleHandleW, Performance::{QueryPerformanceCounter, QueryPerformanceFrequency}, Threading::Sleep},
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
                DefWindowProcW,
                SW_SHOW,
                PM_REMOVE,
                WM_QUIT,
                WM_DESTROY,
                WM_CLOSE,
                WM_PAINT,
                WM_ACTIVATEAPP,
                DestroyWindow,
                PostQuitMessage,
                WM_SIZE,
            }, Input::KeyboardAndMouse,
        },
        Foundation::{HWND, LRESULT, WPARAM, LPARAM}
    },
};

mod platform_lib;
use platform_lib::*;

const INITIAL_WIDTH: i32 = 1024;
const INITIAL_HEIGHT: i32 = 800;

extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM
) -> LRESULT {
    let ctx: Option<&mut AppWindowContext> = unsafe {
        let ctx = WindowsAndMessaging::GetWindowLongPtrW(hwnd, WindowsAndMessaging::GWLP_USERDATA) as *mut AppWindowContext;
        if ctx.is_null() { None } else { Some(&mut *ctx) }
    };

    match msg {
        WM_DESTROY => {
            // TODO(chris): see what happens if this fails but probably we don't
            // really care assuming it involves the process dieing
            let _ = unsafe { DestroyWindow(hwnd) };
            LRESULT(0)
        },
        WM_CLOSE => {
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        },
        WM_PAINT => {
            // let mut paint: MaybeUninit<Gdi::PAINTSTRUCT> = MaybeUninit::uninit();
            // let (hdc, paint) = unsafe {
            //     let hdc = Gdi::BeginPaint(hwnd, paint.as_mut_ptr());
            //     (hdc, paint.assume_init())
            // };

            // let ctx = ctx.expect("WM_PAINT called before window context was initialized");
            // unsafe {
            //     StretchDIBits(
            //         hdc,
            //         0, 0, INITIAL_WIDTH, INITIAL_HEIGHT,
            //         0, 0, ctx.width, ctx.height,
            //         Some(ctx.pixels.as_ptr() as *const ffi::c_void),
            //         &ctx.bitmap_info,
            //         Gdi::DIB_RGB_COLORS,
            //         Gdi::SRCCOPY);

            //     Gdi::EndPaint(hwnd, &paint)
            // };
            LRESULT(0)
        },
        WM_KEYUP => {
            if let Some(ctx) = ctx {
                let vkcode = KeyboardAndMouse::VIRTUAL_KEY(wparam.0 as u16);
                match vkcode {
                    KeyboardAndMouse::VK_RIGHT => {
                        const DELTA: f32 = 3.1415 * -(1.0 / 20.0);
                        ctx.rotation += DELTA;
                    },
                    KeyboardAndMouse::VK_LEFT => {
                        const DELTA: f32 = 3.1415 * (1.0 / 20.0);
                        ctx.rotation += DELTA;
                    },
                    KeyboardAndMouse::VK_P => {
                        ctx.draw_parameters.draw_perspective = !ctx.draw_parameters.draw_perspective;
                    },
                    KeyboardAndMouse::VK_Z => {
                        ctx.draw_parameters.draw_depth_buffer = !ctx.draw_parameters.draw_depth_buffer;
                    },
                    KeyboardAndMouse::VK_T => {
                        ctx.draw_parameters.depth_test = !ctx.draw_parameters.depth_test;
                    },
                    _ => {},
                }
            };
            LRESULT(0)
        },
        WM_ACTIVATEAPP => {
            LRESULT(0)
        },
        WM_SIZE => {
            if let Some(ctx) = ctx {
                const LO_MASK: usize = (1 << 16) - 1;
                let width = (lparam.0 as usize & LO_MASK) as u16;
                let height = ((lparam.0 as usize >> 16) & LO_MASK) as u16;
                ctx.width = width as i32;
                ctx.height = height as i32;
                // TODO(chris): do we need to repaint here immediately?
                ctx.pixels = vec![0; ctx.width as usize * ctx.height as usize];
                ctx.bitmap_info.bmiHeader.biWidth = ctx.width;
                ctx.bitmap_info.bmiHeader.biHeight = -ctx.height;
            }
            LRESULT(0)
        },
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
    draw_parameters: jordan_tinyrenderer::DrawParameters,
    rotation: f32,
}

fn query_performance_counter() -> i64 {
    let mut counter = 0i64;
    // NOTE(chris): this can never fail on windows xp or later
    let _ = unsafe { QueryPerformanceCounter(&mut counter) };
    counter
}

fn query_performance_frequency() -> i64 {
    let mut perf_count_frequency = 0i64;
    // NOTE(chris): this can never fail on windows xp or later
    // counts/second
    let _ = unsafe { QueryPerformanceFrequency(&mut perf_count_frequency) };
    perf_count_frequency
}

pub fn platform_main() -> PlatformResult {
    let title = to_pcwstr("tinyrenderer-window");
    let wc_name = to_pcwstr("tinyrenderer-window-class");

    let main_module = unsafe { GetModuleHandleW(PCWSTR(ptr::null())) }?;
    let wc = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW | CS_OWNDC,
        hInstance: main_module.into(),
        lpszClassName: wc_name,
        lpfnWndProc: Some(window_proc),
        cbWndExtra: mem::size_of::<*mut AppWindowContext>() as i32,
        ..Default::default()
    };
    if unsafe { RegisterClassW(&wc) } == 0 {
        let e = log_win32_error("couldn't create window, failed to register window class.");
        return Err(e);
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
            None
        )
    };

    const FB_SIZE: usize = INITIAL_WIDTH as usize * INITIAL_HEIGHT as usize;
    let mut app_window_ctx = AppWindowContext {
        rotation: 3.1415,
        pixels: vec![0; FB_SIZE],
        width: INITIAL_WIDTH,
        height: INITIAL_HEIGHT,
        running: true,
        draw_parameters: jordan_tinyrenderer::DrawParameters {
            depth_test: true,
            draw_depth_buffer: false,
            draw_perspective: false,
        },
        bitmap_info: BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: INITIAL_WIDTH,
                biHeight: -INITIAL_HEIGHT,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        }
    };
    unsafe { WindowsAndMessaging::SetWindowLongPtrW(main_window, WindowsAndMessaging::GWLP_USERDATA, (&mut app_window_ctx as *mut AppWindowContext) as isize)};

    let hdc = unsafe { GetDC(main_window) };
    let mut msg = Default::default();

    if !unsafe { WindowsAndMessaging::ShowWindow(main_window, SW_SHOW) }.as_bool() {
        return Err(core::Error::from_win32());
    }

    const FOCAL_LENGTH: f32 = 1.0;
    let observer_position: Coord<f32, 3> = Coord([0.0, 0.0, 1.3]);

    const AFRICAN_HEAD_OBJ: &str = include_str!("../../../obj/african_head.obj");
    let (_, african_head_mesh) = obj::parse_obj::<(_, nom::error::ErrorKind)>(AFRICAN_HEAD_OBJ).expect("unexpectedly failed to parse african_head.obj");

    let perf_count_frequency = query_performance_frequency();

    const MS_PER_SECOND: u32 = 1000;
    let target_fps = 30;
    let target_seconds_per_frame = 1.0 / target_fps as f32;

    unsafe {
        let mut last_counter = query_performance_counter();
        while app_window_ctx.running {
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

            const SCALE: f32 = 1.3;
            const VIEWSCREEN_WIDTH: f32 = 2.5 * SCALE;
            const VIEWSCREEN_HEIGHT: f32 = 2.0 * SCALE;

            jordan_tinyrenderer::update_fb(
                app_window_ctx.rotation,
                &app_window_ctx.draw_parameters,
                &african_head_mesh,
                &mut vec![f32::MIN; FB_SIZE],
                &mut app_window_ctx.pixels,
                app_window_ctx.width as u32,
                app_window_ctx.height as u32,
                VIEWSCREEN_WIDTH,
                VIEWSCREEN_HEIGHT,
                observer_position.z(),
                FOCAL_LENGTH);

            let elapsed_seconds = (query_performance_counter() - last_counter) as f32 / perf_count_frequency as f32;
            if elapsed_seconds < target_seconds_per_frame {
                Sleep((MS_PER_SECOND as f32 * (target_seconds_per_frame - elapsed_seconds)) as u32);
            } else {
                // TODO(chris): missed frame
            }

            StretchDIBits(
                hdc,
                0, 0, INITIAL_WIDTH, INITIAL_HEIGHT,
                0, 0, app_window_ctx.width, app_window_ctx.height,
                Some(app_window_ctx.pixels.as_ptr() as *const ffi::c_void),
                &app_window_ctx.bitmap_info,
                Gdi::DIB_RGB_COLORS,
                Gdi::SRCCOPY);

            last_counter = query_performance_counter();
        }
    }

    Ok(())
}
