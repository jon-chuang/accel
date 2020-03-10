use super::context::*;
use crate::{ffi_call_unsafe, ffi_new_unsafe};
use anyhow::{ensure, Result};
use cuda::*;
use std::mem::MaybeUninit;

pub use cuda::CUmemAttach_flags_enum as AttachFlag;

/// Each variants correspond to the following:
///
/// - Host memory
/// - Device memory
/// - Array memory
/// - Unified device or host memory
pub use cuda::CUmemorytype_enum as MemoryType;

/// Total and Free memory size of the device (in bytes)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemoryInfo {
    pub free: usize,
    pub total: usize,
}

impl MemoryInfo {
    pub fn get(ctx: &Context) -> Result<Self> {
        ensure!(ctx.is_current()?, "Given context must be current");
        let mut free = 0;
        let mut total = 0;
        ffi_call_unsafe!(
            cuMemGetInfo_v2,
            &mut free as *mut usize,
            &mut total as *mut usize
        )?;
        Ok(MemoryInfo { free, total })
    }
}

/// Memory allocated on the device.
pub struct DeviceMemory {
    ptr: CUdeviceptr,
    size: usize,
}

impl Drop for DeviceMemory {
    fn drop(&mut self) {
        ffi_call_unsafe!(cuMemFree_v2, self.ptr).expect("Failed to free device memory");
    }
}

impl DeviceMemory {
    /// Allocate a new device memory with `size` byte length by [cuMemAlloc].
    /// This memory is not managed by the unified memory system.
    ///
    /// [cuMemAlloc]: https://docs.nvidia.com/cuda/cuda-driver-api/group__CUDA__MEM.html#group__CUDA__MEM_1gb82d2a09844a58dd9e744dc31e8aa467
    pub fn non_managed(ctx: &Context, size: usize) -> Result<Self> {
        ensure!(ctx.is_current()?, "Given context must be current");
        let ptr = ffi_new_unsafe!(cuMemAlloc_v2, size)?;
        Ok(DeviceMemory { ptr, size })
    }

    /// Allocate a new device memory with `size` byte length by [cuMemAllocManaged].
    /// This memory is managed by the unified memory system.
    ///
    /// [cuMemAllocManaged]: https://docs.nvidia.com/cuda/cuda-driver-api/group__CUDA__MEM.html#group__CUDA__MEM_1gb82d2a09844a58dd9e744dc31e8aa467
    pub fn managed(ctx: &Context, size: usize, flag: AttachFlag) -> Result<Self> {
        ensure!(ctx.is_current()?, "Given context must be current");
        let ptr = ffi_new_unsafe!(cuMemAllocManaged, size, flag as u32)?;
        Ok(DeviceMemory { ptr, size })
    }

    /// Length of Device memory (in bytes)
    pub fn len(&self) -> usize {
        self.size
    }

    fn get_attr<Attr>(&self, attr: CUpointer_attribute) -> Result<Attr> {
        let ty = MaybeUninit::uninit();
        ffi_call_unsafe!(cuPointerGetAttribute, ty.as_ptr() as *mut _, attr, self.ptr)?;
        let ty = unsafe { ty.assume_init() };
        Ok(ty)
    }

    /// Unique ID of the memory
    pub fn buffer_id(&self) -> Result<u64> {
        self.get_attr(CUpointer_attribute::CU_POINTER_ATTRIBUTE_BUFFER_ID)
    }

    /// Check if the memory is managed by the unified memory system
    pub fn is_managed(&self) -> Result<bool> {
        self.get_attr(CUpointer_attribute::CU_POINTER_ATTRIBUTE_IS_MANAGED)
    }

    pub fn memory_type(&self) -> Result<MemoryType> {
        self.get_attr(CUpointer_attribute::CU_POINTER_ATTRIBUTE_MEMORY_TYPE)
    }

    /// Check if this pointer is in a valid address range that is mapped to a backing allocation.
    /// This will always returns true
    pub fn is_mapped(&self) -> Result<bool> {
        self.get_attr(CUpointer_attribute::CU_POINTER_ATTRIBUTE_MAPPED)
    }
}

#[cfg(test)]
mod tests {
    use super::super::device::*;
    use super::*;

    #[test]
    fn info() -> Result<()> {
        let device = Device::nth(0)?;
        let ctx = device.create_context_auto()?;
        let mem_info = MemoryInfo::get(&ctx)?;
        dbg!(&mem_info);
        assert!(mem_info.free > 0);
        assert!(mem_info.total > mem_info.free);
        Ok(())
    }

    #[test]
    fn new() -> Result<()> {
        let device = Device::nth(0)?;
        let ctx = device.create_context_auto()?;
        let mem = DeviceMemory::non_managed(&ctx, 12)?;
        assert_eq!(mem.len(), 12);
        Ok(())
    }

    #[test]
    fn pointer_attributes() -> Result<()> {
        let device = Device::nth(0)?;
        let ctx = device.create_context_auto()?;

        // non-managed
        let mem1 = DeviceMemory::non_managed(&ctx, 12)?;
        dbg!(mem1.buffer_id()?);
        assert_eq!(mem1.memory_type()?, MemoryType::CU_MEMORYTYPE_DEVICE);
        assert!(!mem1.is_managed()?);
        assert!(mem1.is_mapped()?);

        // managed
        let mem2 = DeviceMemory::managed(&ctx, 12, AttachFlag::CU_MEM_ATTACH_GLOBAL)?;
        assert_eq!(mem2.memory_type()?, MemoryType::CU_MEMORYTYPE_DEVICE);
        assert!(mem2.is_managed()?);
        assert!(mem2.is_mapped()?);

        // Buffer id of two different memory must be different
        assert_ne!(mem1.buffer_id()?, mem2.buffer_id()?);
        Ok(())
    }
}
