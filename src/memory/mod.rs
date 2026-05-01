pub mod bitmap;
pub mod map;

pub use bitmap::{BitmapAllocator, InitError};
pub use map::{
    AllocationResult, BootMemoryMapSnapshot, MAX_MEMORY_REGIONS, MemoryRegion, PAGE_SIZE, PageSpan,
    RegionKind, UEFI_MEMORY_MAP_STORAGE_BYTES, align_down, align_up,
};
