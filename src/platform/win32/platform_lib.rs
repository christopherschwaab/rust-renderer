use log::error;
use windows::core::{self, PCWSTR};

pub fn to_wstring<S: AsRef<str>>(s: S) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    std::ffi::OsStr::new(s.as_ref())
        .encode_wide()
        .chain(Some(0))
        .collect()
}

pub fn to_pcwstr<S: AsRef<str>>(s: S) -> PCWSTR {
    PCWSTR(to_wstring(s).as_ptr())
}

pub fn log_win32_error(msg: &str) -> core::Error {
    let e = core::Error::from_win32();
    error!("{}: {}", msg, e);
    e
}
