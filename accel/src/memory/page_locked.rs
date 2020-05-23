//! Device and Host memory handlers

use super::*;
use crate::{error::Result, *};
use cuda::*;
use std::ops::{Deref, DerefMut};

/// Host memory as page-locked.
///
/// Allocating excessive amounts of pinned memory may degrade system performance,
/// since it reduces the amount of memory available to the system for paging.
/// As a result, this function is best used sparingly to allocate staging areas for data exchange between host and device.
///
/// See also [cuMemAllocHost].
///
/// [cuMemAllocHost]: https://docs.nvidia.com/cuda/cuda-driver-api/group__CUDA__MEM.html#group__CUDA__MEM_1gdd8311286d2c2691605362c689bc64e0
#[derive(Contexted)]
pub struct PageLockedMemory<T> {
    ptr: *mut T,
    size: usize,
    context: Context,
}

impl<T> Drop for PageLockedMemory<T> {
    fn drop(&mut self) {
        if let Err(e) = unsafe { contexted_call!(self, cuMemFreeHost, self.ptr as *mut _) } {
            log::error!("Cannot free page-locked memory: {:?}", e);
        }
    }
}

impl<T> Deref for PageLockedMemory<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.ptr as _, self.size) }
    }
}

impl<T> DerefMut for PageLockedMemory<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.size) }
    }
}

impl<T: Scalar> Memory for PageLockedMemory<T> {
    type Elem = T;
    fn head_addr(&self) -> *const T {
        self.ptr as _
    }

    fn head_addr_mut(&mut self) -> *mut T {
        self.ptr as _
    }

    fn num_elem(&self) -> usize {
        self.size
    }

    fn memory_type(&self) -> MemoryType {
        MemoryType::PageLocked
    }
}

impl<T: Scalar> Memcpy<Self> for PageLockedMemory<T> {
    fn copy_from(&mut self, src: &Self) {
        assert_ne!(self.head_addr(), src.head_addr());
        assert_eq!(self.num_elem(), src.num_elem());
        self.copy_from_slice(src)
    }
}

impl<T: Scalar> Memcpy<RegisteredMemory<'_, T>> for PageLockedMemory<T> {
    fn copy_from(&mut self, src: &RegisteredMemory<'_, T>) {
        assert_ne!(self.head_addr(), src.head_addr());
        assert_eq!(self.num_elem(), src.num_elem());
        self.copy_from_slice(src)
    }
}

impl<T: Scalar> Memcpy<DeviceMemory<T>> for PageLockedMemory<T> {
    fn copy_from(&mut self, src: &DeviceMemory<T>) {
        assert_ne!(self.head_addr(), src.head_addr());
        assert_eq!(self.num_elem(), src.num_elem());
        unsafe {
            contexted_call!(
                self,
                cuMemcpyDtoH_v2,
                self.as_mut_ptr() as *mut _,
                src.as_ptr() as CUdeviceptr,
                self.num_elem() * T::size_of()
            )
        }
        .expect("memcpy from Device to Page-locked memory failed")
    }
}

impl<T: Scalar> Memset for PageLockedMemory<T> {
    fn set(&mut self, value: Self::Elem) {
        self.iter_mut().for_each(|v| *v = value);
    }
}

impl<T: Scalar> Continuous for PageLockedMemory<T> {
    fn as_slice(&self) -> &[T] {
        self
    }
    fn as_mut_slice(&mut self) -> &mut [T] {
        self
    }
}

impl<T: Scalar> Managed for PageLockedMemory<T> {}

impl<T: Scalar> Allocatable for PageLockedMemory<T> {
    type Shape = usize;
    unsafe fn uninitialized(context: &Context, size: usize) -> Self {
        assert!(size > 0, "Zero-sized malloc is forbidden");
        let ptr = contexted_new!(context, cuMemAllocHost_v2, size * std::mem::size_of::<T>())
            .expect("Cannot allocate page-locked memory");
        Self {
            ptr: ptr as *mut T,
            size,
            context: context.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_mut_slice() -> Result<()> {
        let device = Device::nth(0)?;
        let context = device.create_context();
        let mut mem = PageLockedMemory::<i32>::zeros(&context, 12);
        let sl = mem.as_mut_slice();

        sl[0] = 3; // test if accessible
        assert_eq!(sl.num_elem(), 12);
        Ok(())
    }

    #[should_panic(expected = "Zero-sized malloc is forbidden")]
    #[test]
    fn page_locked_new_zero() {
        let device = Device::nth(0).unwrap();
        let context = device.create_context();
        let _a = PageLockedMemory::<i32>::zeros(&context, 0);
    }
}
