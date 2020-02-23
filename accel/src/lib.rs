//! GPGPU framework for Rust

extern crate cuda_driver_sys as cuda;
extern crate cuda_runtime_sys as cudart;

pub mod device;
pub mod driver;
pub mod error;
pub mod mvec;
pub mod uvec;

pub use driver::kernel::{Block, Grid};
pub use mvec::MVec;
pub use uvec::UVec;

#[macro_export]
macro_rules! ffi_call {
    ($ffi:path, $($args:expr),*) => {
        $ffi($($args),*).check(stringify!($ffi))
    };
    ($ffi:path) => {
        $ffi().check(stringify!($ffi))
    };
}

#[macro_export]
macro_rules! ffi_call_unsafe {
    ($ffi:path, $($args:expr),*) => {
        unsafe { $ffi($($args),*).check(stringify!($ffi)) }
    };
    ($ffi:path) => {
        unsafe { $ffi().check(stringify!($ffi)) }
    };
}

#[macro_export]
macro_rules! ffi_new {
    ($ffi:path, $($args:expr),*) => {
        unsafe {
            let mut value = ::std::mem::MaybeUninit::uninit();
            $ffi(value.as_mut_ptr(), $($args),*).check(stringify!($ffi)).map(|_| value.assume_init())
        }
    };
    ($ffi:path) => {
        unsafe {
            let mut value = ::std::mem::MaybeUninit::uninit();
            $ffi(value.as_mut_ptr()).check(stringify!($ffi)).map(|_| value.assume_init())
        }
    };
}
