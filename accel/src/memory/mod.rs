//! Memory management
//!
//! Unified address
//! ---------------
//!
//! - All memories are mapped into a single 64bit memory space
//! - We can get where the pointed memory exists from its value.
//!
//! Memory Types
//! ------------
//!
//! |name                 | where exists | From Host | From Device | As slice | Description                                                            |
//! |:--------------------|:------------:|:---------:|:-----------:|:--------:|:-----------------------------------------------------------------------|
//! | (usual) Host memory | Host         | ✓         |  -          |  ✓       | allocated by usual manner, e.g. `vec![0; n]`                           |
//! | [RegisteredMemory]  | Host         | ✓         |  ✓          |  ✓       | A host memory registered into CUDA memory management system            |
//! | [PageLockedMemory]  | Host         | ✓         |  ✓          |  ✓       | OS memory paging is disabled for accelerating memory transfer          |
//! | [DeviceMemory]      | Device       | ✓         |  ✓          |  ✓       | allocated on device as a single span                                   |
//! | [Array]             | Device       | ✓         |  ✓          |  -       | properly aligned memory on device for using Texture and Surface memory |
//!
//! Traits
//! -------
//!
//! |traits       |`[T]`|[RegisteredMemory]|[PageLockedMemory]|[DeviceMemory]|[Array]| Description                                |
//! |:------------|:---:|:----------------:|:----------------:|:------------:|:-----:|:-------------------------------------------|
//! |[Memory]     | ✓   | ✓                | ✓                | ✓            | ✓     | Has Unified address and element size       |
//! |[Contexted]  | -   | ✓                | ✓                | ✓            | ✓     | with CUDA Context                          |
//! |[Continuous] | ✓   | ✓                | ✓                | ✓            | -     | Can be treated as a Rust slice             |
//! |[Allocatable]| -   | -                | ✓                | ✓            | ✓     | Newly allocatable with its shape and value |
//!
//! [RegisteredMemory]: ./struct.RegisteredMemory.html
//! [PageLockedMemory]: ./struct.PageLockedMemory.html
//! [DeviceMemory]: ./struct.DeviceMemory.html
//! [Array]: ./struct.Array.html
//!
//! [Memory]: ./trait.Memory.html
//! [Memset]: ./trait.Memset.html
//! [Contexted]: ../device/trait.Contexted.html
//! [Continuous]: ./trait.Continuous.html
//! [Allocatable]: ./trait.Allocatable.html

mod array;
mod device;
mod dimension;
mod info;
mod page_locked;
mod registered;
mod scalar;
mod slice;

pub use array::*;
pub use device::*;
pub use dimension::*;
pub use info::*;
pub use page_locked::*;
pub use registered::*;
pub use scalar::*;

use crate::*;
use cuda::*;
use futures::future::BoxFuture;
use num_traits::Zero;
use std::{ffi::c_void, mem::MaybeUninit};

/// Memory type
///
/// Because of [unified addressing], we can get the memory type after casted into slice:
///
/// - [DeviceMemory]
///
/// ```
/// # use accel::{*, memory::*};
/// # let device = Device::nth(0).unwrap();
/// # let ctx = device.create_context();
/// let mem = DeviceMemory::<i32>::zeros(&ctx, 12);
/// let sl = mem.as_slice();
/// assert_eq!(sl.memory_type(), MemoryType::Device);
/// ```
///
/// - [PageLockedMemory]
///
/// ```
/// # use accel::{*, memory::*};
/// # let device = Device::nth(0).unwrap();
/// # let ctx = device.create_context();
/// let mem = PageLockedMemory::<i32>::zeros(&ctx, 12);
/// let sl = mem.as_slice();
/// assert_eq!(sl.memory_type(), MemoryType::PageLocked);
/// ```
///
/// - [RegisteredMemory]
///   - Be sure that [RegisteredMemory] and [PageLockedMemory] are indistinguishable
///
/// ```
/// # use accel::{*, memory::*};
/// # let device = Device::nth(0).unwrap();
/// # let ctx = device.create_context();
/// let mut a = vec![0_i32; 12];
/// let mem = RegisteredMemory::<i32>::new(&ctx, &mut a);
/// let sl = mem.as_slice();
/// assert_eq!(sl.memory_type(), MemoryType::PageLocked);
/// ```
///
/// - [Array] cannot be casted into a slice
///
/// [unified addressing]: https://docs.nvidia.com/cuda/cuda-driver-api/group__CUDA__UNIFIED.html#group__CUDA__UNIFIED
/// [Array]: ./struct.Array.html
/// [DeviceMemory]: ./struct.DeviceMemory.html
/// [RegisteredMemory]: ./struct.RegisteredMemory.html
/// [PageLockedMemory]: ./struct.PageLockedMemory.html
///
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum MemoryType {
    /// Host memory **not** managed by CUDA memory system
    Host,
    /// Host memory managed by CUDA memory system, i.e.
    /// [RegisteredMemory](./struct.RegisteredMemory.html), and
    /// [PageLockedMemory](./struct.PageLockedMemory.html)
    PageLocked,
    /// Device memory
    Device,
    /// Array memory
    Array,
}

/// Has unique head address and allocated size.
pub trait Memory {
    /// Scalar type of each element
    type Elem: PartialEq + std::fmt::Debug + Copy + Send + Sync + Default + Sized;

    /// Get head address of the memory as a const pointer
    fn head_addr(&self) -> *const Self::Elem;

    /// Get head address of the memory as a mutable pointer
    fn head_addr_mut(&mut self) -> *mut Self::Elem;

    /// Number of elements
    fn num_elem(&self) -> usize;

    /// Get memory type, See [MemoryType](./enum.MemoryType.html) for detail.
    fn memory_type(&self) -> MemoryType;

    /// Set all elements by `value`
    ///
    /// Examples
    /// ---------
    ///
    /// - Set `i32`
    ///
    /// ```
    /// # use accel::*;
    /// # let device = Device::nth(0).unwrap();
    /// # let ctx = device.create_context();
    /// let mut mem = DeviceMemory::<i32>::zeros(&ctx, 12);
    /// mem.set(1234);
    /// for &val in mem.as_slice() {
    ///   assert_eq!(val, 1234);
    /// }
    /// ```
    ///
    /// - Set `f32`
    ///   - Be sure that `f64` is not supported yet because CUDA does not support 64-bit memset.
    ///
    /// ```
    /// # use accel::*;
    /// # let device = Device::nth(0).unwrap();
    /// # let ctx = device.create_context();
    /// let mut mem = DeviceMemory::<f32>::zeros(&ctx, 12);
    /// mem.set(1.0);
    /// for &val in mem.as_slice() {
    ///   assert_eq!(val, 1.0);
    /// }
    /// ```
    ///
    /// - Set for host memory equals to `mem.iter_mut().for_each(|v| *v = value)`
    ///
    /// ```
    /// # use accel::*;
    /// # let device = Device::nth(0).unwrap();
    /// # let ctx = device.create_context();
    /// let mut mem = PageLockedMemory::<i32>::zeros(&ctx, 12);
    /// mem.set(1234);
    /// for &val in mem.as_slice() {
    ///   assert_eq!(val, 1234);
    /// }
    /// ```
    fn set(&mut self, value: Self::Elem);

    /// Sets memory to 0u8 for as many bytes as size_of::<T>() contains
    fn set_zero_u8(&mut self);
}

/// Copy data from one to another
pub trait Memcpy<Target: Memory<Elem = Self::Elem> + ?Sized>: Memory {
    /// Examples
    /// ---------
    ///
    /// - memcpy from page-locked host memory to device memory
    ///
    /// ```
    /// # use accel::*;
    /// # let device = Device::nth(0).unwrap();
    /// # let ctx = device.create_context();
    /// let mut dest = DeviceMemory::<i32>::zeros(&ctx, 12);
    /// let src = PageLockedMemory::<i32>::zeros(&ctx, 12);
    /// dest.copy_from(&src);
    /// ```
    ///
    /// - memcpy from device memory to page-locked host memory
    ///
    /// ```
    /// # use accel::*;
    /// # let device = Device::nth(0).unwrap();
    /// # let ctx = device.create_context();
    /// let mut dest = PageLockedMemory::<i32>::zeros(&ctx, 12);
    /// let src = DeviceMemory::<i32>::zeros(&ctx, 12);
    /// dest.copy_from(&src);
    /// ```
    ///
    /// - memcpy from device to device
    ///
    /// ```
    /// # use accel::*;
    /// # let device = Device::nth(0).unwrap();
    /// # let ctx = device.create_context();
    /// let mut dest = DeviceMemory::<i32>::zeros(&ctx, 12);
    /// let src = DeviceMemory::<i32>::zeros(&ctx, 12);
    /// dest.copy_from(&src);
    /// ```
    ///
    /// - memcpy from Rust slice to device memory
    ///
    /// ```
    /// # use accel::*;
    /// # use std::ops::DerefMut;
    /// # let device = Device::nth(0).unwrap();
    /// # let ctx = device.create_context();
    /// let mut dest = DeviceMemory::<i32>::zeros(&ctx, 12);
    /// let src = vec![0_i32; 12];
    /// dest.copy_from(src.as_slice()); // requires explicit cast to slice
    /// ```
    ///
    /// - memcpy from device memory to Rust slice
    ///
    /// ```
    /// # use accel::*;
    /// # let device = Device::nth(0).unwrap();
    /// # let ctx = device.create_context();
    /// let mut dest = vec![0_i32; 12];
    /// let src = DeviceMemory::<i32>::zeros(&ctx, 12);
    /// dest.copy_from(&src);
    /// ```
    ///
    /// - Cannot copy between different types
    ///
    /// ```compile_fail
    /// # use accel::*;
    /// # let device = Device::nth(0).unwrap();
    /// # let ctx = device.create_context();
    /// let mut dest = DeviceMemory::<i64>::zeros(&ctx, 12);
    /// let src = PageLockedMemory::<i32>::zeros(&ctx, 12);
    /// dest.copy_from(&src); // compile fail
    /// ```
    ///
    /// - Panics if sizes are different
    ///
    /// ```should_panic
    /// # use accel::*;
    /// # let device = Device::nth(0).unwrap();
    /// # let ctx = device.create_context();
    /// let mut dest = DeviceMemory::<i32>::zeros(&ctx, 24);
    /// let src = PageLockedMemory::<i32>::zeros(&ctx, 12);
    /// dest.copy_from(&src); // will panic
    /// ```
    ///
    /// Panics
    /// -------
    ///
    /// - if `self` and `src` are identical
    /// - if sizes of memory mismatch
    ///
    fn copy_from(&mut self, source: &Target);

    /// Copy data in async manner
    ///
    /// ```
    /// use accel::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///   let device = Device::nth(0).unwrap();
    ///   let ctx = device.create_context();
    ///   let mut dest = DeviceMemory::<f32>::zeros(&ctx, 12);
    ///   let src = PageLockedMemory::<f32>::zeros(&ctx, 12);
    ///   dest.copy_from_async(&src).await;
    /// }
    /// ```
    ///
    /// - Arrays are captured until await:
    ///
    /// ```
    /// # use accel::*;
    /// # #[tokio::main]
    /// # async fn main() {
    /// # let device = Device::nth(0).unwrap();
    /// # let ctx = device.create_context();
    /// # let mut dest = DeviceMemory::<f32>::zeros(&ctx, 12);
    /// # let src = PageLockedMemory::<f32>::zeros(&ctx, 12);
    /// let future = dest.copy_from_async(&src);
    /// println!("src[0] = {}", src[0]);  // Source is always accessible as usual &-reference
    /// future.await;
    /// # }
    /// ```
    ///
    /// ```compile_fail
    /// # use accel::*;
    /// # #[tokio::main]
    /// # async fn main() {
    /// # let device = Device::nth(0).unwrap();
    /// # let ctx = device.create_context();
    /// # let mut dest = DeviceMemory::<f32>::zeros(&ctx, 12);
    /// # let src = PageLockedMemory::<f32>::zeros(&ctx, 12);
    /// let future = dest.copy_from_async(&src);
    /// println!("dest[0] = {}", dest[0]);  // Destination is not accessible until .await
    /// future.await;
    /// # }
    /// ```
    fn copy_from_async<'a>(&'a mut self, src: &'a Target) -> BoxFuture<'a, ()>;
}

/// Allocatable memories with CUDA context
pub trait Allocatable: Contexted + Memory + Sized {
    /// Shape for initialization
    type Shape: Zero;

    /// Allocate a memory without initialization
    ///
    /// Safety
    /// ------
    /// - Cause undefined behavior when read before write
    ///
    /// Panic
    /// ------
    /// - if shape is zero
    unsafe fn uninitialized(ctx: &Context, shape: Self::Shape) -> Self;

    /// uniformly initialized
    ///
    /// Panic
    /// ------
    /// - if shape is zero
    fn from_elem(ctx: &Context, shape: Self::Shape, elem: Self::Elem) -> Self {
        let mut mem = unsafe { Self::uninitialized(ctx, shape) };
        mem.set(elem);
        mem
    }

    /// uniformly initialized by zero
    ///
    /// Panic
    /// ------
    /// - if shape is zero
    fn zeros(ctx: &Context, shape: Self::Shape) -> Self {
        let mut mem = unsafe { Self::uninitialized(ctx, shape) };
        mem.set_zero_u8();
        mem
    }
}

/// Memory which has continuous 1D index, i.e. can be treated as a Rust slice
pub trait Continuous: Memory {
    fn as_slice(&self) -> &[Self::Elem];
    fn as_mut_slice(&mut self) -> &mut [Self::Elem];
}
