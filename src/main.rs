#[path = "platform/win32/platform_main.rs"]
#[cfg(target_os = "windows")]
mod platform;

#[path = "platform/linux/platform_main.rs"]
#[cfg(target_os = "linux")]
mod platform;

fn main() -> platform::PlatformResult {
    platform::platform_main()
}
