mod address_space;
mod mapper;
mod table;

pub use address_space::{
    AddressSpace, AddressSpaceKind, KernelMappingTemplate, KernelVirtualAllocation,
    PagingAllocationRecord,
};
#[cfg(target_os = "uefi")]
pub use address_space::set_runtime_page_access_active;
#[cfg(not(target_os = "uefi"))]
pub use address_space::reset_host_shadow_page_tables_for_tests;
pub use mapper::{map_range, unmap_range, PagingError};
pub use table::{
    EntryFlags, MappingRequest, MappedPage, PAGE_TABLE_ADDR_MASK, PageTableLevel,
    VirtualAddressLayout, KERNEL_ALLOC_BASE, KERNEL_ALLOC_LIMIT, KERNEL_DIRECT_MAP_BASE,
    KERNEL_DIRECT_MAP_LIMIT, KERNEL_VIRT_BASE, PAGE_TABLE_ENTRIES,
};
