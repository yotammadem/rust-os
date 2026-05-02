pub mod bitmap;
pub mod map;
pub mod paging;

pub use bitmap::{BOOTSTRAP_BITMAP_STORAGE_BYTES, BitmapAllocator, InitError};
pub use map::{
    AllocationResult, BootMemoryMapSnapshot, MAX_MEMORY_REGIONS, MemoryRegion, PAGE_SIZE,
    PageSpan, PhysAddr, RegionKind, UEFI_MEMORY_MAP_STORAGE_BYTES, align_down, align_up,
};
pub use paging::{
    AddressSpace, AddressSpaceKind, EntryFlags, KernelMappingTemplate, KernelVirtualAllocation,
    KERNEL_DIRECT_MAP_BASE, KERNEL_DIRECT_MAP_LIMIT, MappedPage, MappingRequest,
    KERNEL_VIRT_BASE, PAGE_TABLE_ADDR_MASK, PAGE_TABLE_ENTRIES, PageTableLevel,
    PagingAllocationRecord, PagingError, VirtualAddressLayout, map_range, unmap_range,
};
#[cfg(not(target_os = "uefi"))]
pub use paging::reset_host_shadow_page_tables_for_tests;
