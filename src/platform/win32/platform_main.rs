use std::{
    ptr,
    mem::MaybeUninit,
};
use log::trace;
use windows::{
    core::{self, PCWSTR},
    Win32::{
        Graphics::Gdi::{self, GetDC},
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
    match msg {
        WM_DESTROY => {
            unsafe { APP_CONTEXT.running = false };
            LRESULT(0)
        },
        WM_CLOSE => {
            unsafe { APP_CONTEXT.running = false };
            LRESULT(0)
        },
        WM_PAINT => {
            let mut paint: MaybeUninit<Gdi::PAINTSTRUCT> = MaybeUninit::uninit();
            let (device_context, paint) = unsafe {
                let device_context = Gdi::BeginPaint(hwnd, paint.as_mut_ptr());
                (device_context, paint.assume_init())
            };

            let window_dimension = WindowDimension::from_window(hwnd);
            unsafe {
                APP_CONTEXT.update_window(device_context, window_dimension.width, window_dimension.height);
                Gdi::EndPaint(hwnd, &paint);
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

        let device_context = GetDC(main_window);

        APP_CONTEXT.resize_dib_section(INITIAL_WIDTH, INITIAL_HEIGHT);
        let mut msg = Default::default();

        if !WindowsAndMessaging::ShowWindow(main_window, SW_SHOW).as_bool() {
            return Err(core::Error::from_win32());
        }

        while APP_CONTEXT.running {
            while PeekMessageW(&mut msg, HWND(0), 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    APP_CONTEXT.running = false;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            let window_dimension = WindowDimension::from_window(main_window);
            let mut p = APP_CONTEXT.bitmap_memory.as_mut_ptr();
            unsafe {
                for _ in 0..APP_CONTEXT.bitmap_memory.height {
                    for _ in 0..APP_CONTEXT.bitmap_memory.width {
                        //let color = if (x + y) % 2 == 0 {
                        //    0x00ff_ffff
                        //} else {
                        //    0x00ff_0000
                        //};
                        *p = Pixel::packed_rgb(0xff0000);
                        p = p.offset(1);
                    }
                }
            }
            APP_CONTEXT.update_window(device_context, window_dimension.width, window_dimension.height);
        }
    }

    Ok(())
}
