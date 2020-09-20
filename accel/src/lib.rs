//! GPGPU framework for Rust based on [CUDA Driver API]
//!
//! [CUDA Driver API]: https://docs.nvidia.com/cuda/cuda-driver-api/
//!
//! Setup
//! -----
//! Currently (0.3.0), accel works only on Linux system. Windows support will come in future release (0.3.x or 0.4~).
//!
//! 1. Install [CUDA](https://developer.nvidia.com/cuda-downloads) on your system
//! 2. Setup Rust environement using rustup (Requires 1.42 or later)
//! 3. Add `nvptx64-nvidia-cuda` target and install `ptx-linker`, or run
//!
//!     ```shell
//!     curl -sSL https://gitlab.com/termoshtt/accel/raw/master/setup_nvptx_toolchain.sh | bash
//!     ```
//!
//! Examples
//! --------
//! accel works with stable Rust
//!
//! ```toml
//! [dependencies]
//! accel = "=0.3.0-alpha.2"
//! ```
//!
//! Do **NOT** add `accel-core` to `[dependencies]`.
//! It will be linked automatically into the device code.
//!
//! ### Vector Add
//!
//! ```
//! use accel::*;
//!
//! #[kernel]
//! unsafe fn add(a: *const f32, b: *const f32, c: *mut f32, n: usize) {
//!     let i = accel_core::index();
//!     if (i as usize) < n {
//!         *c.offset(i) = *a.offset(i) + *b.offset(i);
//!     }
//! }
//!
//! fn main() -> error::Result<()> {
//!     let device = Device::nth(0)?;
//!     let ctx = device.create_context();
//!
//!     // Allocate memories on GPU
//!     let n = 32;
//!     let mut a = DeviceMemory::<f32>::zeros(&ctx, n);
//!     let mut b = DeviceMemory::<f32>::zeros(&ctx, n);
//!     let mut c = DeviceMemory::<f32>::zeros(&ctx, n);
//!
//!     // Accessible from CPU as usual Rust slice (though this will be slow)
//!     for i in 0..n {
//!         a[i] = i as f32;
//!         b[i] = 2.0 * i as f32;
//!     }
//!     println!("a = {:?}", a.as_slice());
//!     println!("b = {:?}", b.as_slice());
//!
//!     // Launch kernel synchronously
//!     add(&ctx,
//!         1 /* grid */,
//!         n /* block */,
//!         (a.as_ptr(), b.as_ptr(), c.as_mut_ptr(), n)
//!     ).expect("Kernel call failed");
//!
//!     println!("c = {:?}", c.as_slice());
//!     Ok(())
//! }
//! ```
//!
//! ### Assertion on GPU
//!
//! ```
//! use accel::*;
//!
//! #[kernel]
//! fn assert() {
//!     accel_core::assert_eq!(1 + 2, 4);  // will fail
//! }
//!
//! fn main() -> error::Result<()> {
//!     let device = Device::nth(0)?;
//!     let ctx = device.create_context();
//!     let result = assert(&ctx, 1 /* grid */, 4 /* block */, ());
//!     assert!(result.is_err()); // assertion failed
//!     Ok(())
//! }
//! ```
//!
//! ### Print from GPU
//!
//! ```
//! use accel::*;
//!
//! #[kernel]
//! pub fn print() {
//!     let i = accel_core::index();
//!     accel_core::println!("Hello from {}", i);
//! }
//!
//! fn main() -> error::Result<()> {
//!     let device = Device::nth(0)?;
//!     let ctx = device.create_context();
//!     print(&ctx, 1, 4, ())?;
//!     Ok(())
//! }
//! ```

extern crate cuda_driver_sys as cuda;

pub use accel_derive::{kernel, kernel_mod, kernel_func};

pub mod device;
pub mod error;
pub mod execution;
pub mod linker;
pub mod memory;
pub mod module;
pub mod profiler;
pub mod stream;

mod block;
mod grid;
mod instruction;

pub use block::Block;
pub use device::*;
pub use execution::*;
pub use grid::Grid;
pub use instruction::Instruction;
pub use linker::*;
pub use memory::*;
pub use module::*;
pub use profiler::*;
pub use stream::*;

#[cfg(test)]
mod tests {
    /// Test accel_derive::kernel can be used in accel crate itself
    #[super::kernel]
    fn f() {}
}
