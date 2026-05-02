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
    MappedPage, MappingRequest, PAGE_TABLE_ENTRIES, PageTableLevel, PagingAllocationRecord,
    PagingError, VirtualAddressLayout, map_range, unmap_range,
};
