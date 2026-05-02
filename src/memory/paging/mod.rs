mod address_space;
mod mapper;
mod table;

pub use address_space::{
    AddressSpace, AddressSpaceKind, KernelMappingTemplate, KernelVirtualAllocation,
    PagingAllocationRecord,
};
pub use mapper::{map_range, unmap_range, PagingError};
pub use table::{
    EntryFlags, MappingRequest, MappedPage, PAGE_TABLE_ADDR_MASK, PageTableLevel,
    VirtualAddressLayout, KERNEL_ALLOC_BASE, KERNEL_ALLOC_LIMIT, KERNEL_DIRECT_MAP_BASE,
    KERNEL_DIRECT_MAP_LIMIT, KERNEL_VIRT_BASE, PAGE_TABLE_ENTRIES,
};
