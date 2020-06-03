use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

#[derive(Eq, PartialEq, Debug)]
#[allow(dead_code)]
pub enum KeyError {
    NotDefined,
    PrefixConflict,
    NotSupported
}

// TODO: runtime detection?
#[cfg(feature = "ncurses-ext")]
extern "C" {
    fn key_defined(definition: *const c_char) -> c_int;
    fn define_key(definition: *const c_char, code: c_int) -> c_int;
}

extern "C" {
    fn tigetstr(name: *const c_char) -> *const c_char;
}

pub fn get_terminfo_string(name: &CStr) -> Option<&'static CStr> {
    let out = unsafe { tigetstr(name.as_ptr()) };
    if out.is_null() || out as isize == -1 {
        None
    } else {
        Some(unsafe { CStr::from_ptr(out) })
    }
}

#[cfg(feature = "ncurses-ext")]
pub fn key_code_for(definition: &CStr) -> Result<c_int, KeyError> {
    let ret = unsafe { key_defined(definition.as_ptr()) };
    if ret == 0 {
        Err(KeyError::NotDefined)
    } else if ret == -1 {
        Err(KeyError::PrefixConflict)
    } else {
        Ok(ret)
    }
}

#[cfg(not(feature = "ncurses-ext"))]
pub fn key_code_for(definition: &CStr) -> Result<c_int, KeyError> { Err(KeyError::NotSupported) }


#[cfg(feature = "ncurses-ext")]
pub unsafe fn define_key_code(definition: &CStr, code: c_int) -> Result<(), ()> {
    let ret = define_key(definition.as_ptr(), code);
    if ret == ncurses::OK {
        Ok(())
    } else {
        Err(())
    }
}

#[cfg(not(feature = "ncurses-ext"))]
pub unsafe fn define_key_code(definition: &CStr, code: c_int) -> Result<(), ()> { Err(()) }

