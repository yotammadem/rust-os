use crate::memory::{AllocationResult, BitmapAllocator, PAGE_SIZE, PageSpan, PhysAddr};

use super::mapper::{PagingError, map_range};
use super::table::{
    EntryFlags, MappedPage, MappingRequest, VirtualAddressLayout, KERNEL_ALLOC_BASE,
    KERNEL_ALLOC_LIMIT, KERNEL_BOOTSTRAP_ALIAS_PML4_INDEX, KERNEL_DIRECT_MAP_BASE,
    KERNEL_VIRT_BASE, PAGE_TABLE_ADDR_MASK, PAGE_TABLE_ENTRIES, PROCESS_PRIVATE_LIMIT,
};

const MAX_OWNED_SPANS: usize = 256;
const MAX_KERNEL_REGIONS: usize = 16;
const MAX_TEMPLATE_ROOT_ENTRIES: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddressSpaceKind {
    Kernel,
    Process,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PagingAllocationRecord {
    spans: [Option<PageSpan>; MAX_OWNED_SPANS],
    span_count: usize,
    published: bool,
}

impl PagingAllocationRecord {
    pub const fn new() -> Self {
        Self {
            spans: [None; MAX_OWNED_SPANS],
            span_count: 0,
            published: false,
        }
    }

    pub fn push(&mut self, span: PageSpan) -> Result<(), PagingError> {
        if self.span_count >= self.spans.len() {
            return Err(PagingError::CapacityExceeded);
        }
        self.spans[self.span_count] = Some(span);
        self.span_count += 1;
        Ok(())
    }

    pub fn publish(&mut self) {
        self.published = true;
    }

    pub fn rollback(&mut self, allocator: &mut BitmapAllocator<'_>) -> Result<(), PagingError> {
        if self.published {
            return Ok(());
        }
        while self.span_count > 0 {
            self.span_count -= 1;
            let span = self.spans[self.span_count].take().expect("span_count tracked");
            match allocator.free_pages(span) {
                AllocationResult::Released(_) => {}
                _ => return Err(PagingError::AllocatorStateCorrupted),
            }
        }
        Ok(())
    }

    pub fn spans(&self) -> impl Iterator<Item = PageSpan> + '_ {
        self.spans[..self.span_count]
            .iter()
            .copied()
            .flatten()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct KernelRegion {
    start: u64,
    end: u64,
    flags: EntryFlags,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KernelMappingTemplate {
    root_entries: [Option<(usize, u64)>; MAX_TEMPLATE_ROOT_ENTRIES],
    root_entry_count: usize,
    pub managed_phys_limit: PhysAddr,
    pub transition_alias_start: u64,
    pub transition_alias_page_count: usize,
}

impl KernelMappingTemplate {
    pub const fn empty() -> Self {
        Self {
            root_entries: [None; MAX_TEMPLATE_ROOT_ENTRIES],
            root_entry_count: 0,
            managed_phys_limit: 0,
            transition_alias_start: 0,
            transition_alias_page_count: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KernelVirtualAllocation {
    pub virt_start_addr: u64,
    pub backing_span: PageSpan,
    pub page_count: usize,
    pub flags: EntryFlags,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AddressSpace {
    pub root_table_phys_addr: PhysAddr,
    pub root_table_virt_addr: u64,
    pub kind: AddressSpaceKind,
    managed_phys_limit: PhysAddr,
    owned_table_pages: [Option<PageSpan>; MAX_OWNED_SPANS],
    owned_table_page_count: usize,
    pub private_mapping_floor: u64,
    pub private_mapping_ceiling: u64,
    pub next_kernel_alloc_virt: u64,
    kernel_regions: [Option<KernelRegion>; MAX_KERNEL_REGIONS],
    kernel_region_count: usize,
}

impl AddressSpace {
    pub fn new_kernel(allocator: &mut BitmapAllocator<'_>) -> Result<Self, PagingError> {
        let (root_span, mut record) = allocate_table_span(allocator)?;
        let mut space = Self {
            root_table_phys_addr: root_span.start_phys_addr,
            root_table_virt_addr: root_span.start_phys_addr,
            kind: AddressSpaceKind::Kernel,
            managed_phys_limit: 0,
            owned_table_pages: [None; MAX_OWNED_SPANS],
            owned_table_page_count: 0,
            private_mapping_floor: 0,
            private_mapping_ceiling: PROCESS_PRIVATE_LIMIT,
            next_kernel_alloc_virt: KERNEL_ALLOC_BASE,
            kernel_regions: [None; MAX_KERNEL_REGIONS],
            kernel_region_count: 0,
        };
        space.record_owned_span(root_span)?;
        record.publish();
        Ok(space)
    }

    pub fn create_kernel_template(
        allocator: &mut BitmapAllocator<'_>,
        kernel_phys_start: PhysAddr,
        kernel_page_count: usize,
    ) -> Result<(Self, KernelMappingTemplate), PagingError> {
        let mut kernel = Self::new_kernel(allocator)?;
        kernel.managed_phys_limit = (allocator.managed_page_count() * PAGE_SIZE) as u64;

        kernel.map_kernel_region(
            allocator,
            KERNEL_DIRECT_MAP_BASE,
            0,
            allocator.managed_page_count(),
            EntryFlags::WRITABLE | EntryFlags::GLOBAL | EntryFlags::NO_EXECUTE,
        )?;
        kernel.map_kernel_region(
            allocator,
            KERNEL_VIRT_BASE,
            kernel_phys_start,
            kernel_page_count,
            EntryFlags::WRITABLE,
        )?;

        let alias_start = kernel_phys_start;
        kernel.map_kernel_region(
            allocator,
            alias_start,
            kernel_phys_start,
            kernel_page_count,
            EntryFlags::WRITABLE,
        )?;

        let mut template = KernelMappingTemplate::empty();
        template.managed_phys_limit = kernel.managed_phys_limit;

        for entry in 0..PAGE_TABLE_ENTRIES {
            let value = kernel.read_table_entry(kernel.root_table_phys_addr, entry);
            if value == 0 || entry == KERNEL_BOOTSTRAP_ALIAS_PML4_INDEX {
                continue;
            }
            if entry < VirtualAddressLayout::pml4_index(KERNEL_VIRT_BASE) {
                continue;
            }
            if template.root_entry_count >= template.root_entries.len() {
                return Err(PagingError::CapacityExceeded);
            }
            template.root_entries[template.root_entry_count] = Some((entry, value));
            template.root_entry_count += 1;
        }

        template.transition_alias_start = alias_start;
        template.transition_alias_page_count = kernel_page_count;
        Ok((kernel, template))
    }

    pub fn from_kernel_template(
        allocator: &mut BitmapAllocator<'_>,
        template: &KernelMappingTemplate,
    ) -> Result<Self, PagingError> {
        let (root_span, mut record) = allocate_table_span(allocator)?;
        let mut space = Self {
            root_table_phys_addr: root_span.start_phys_addr,
            root_table_virt_addr: root_span.start_phys_addr,
            kind: AddressSpaceKind::Process,
            managed_phys_limit: template.managed_phys_limit,
            owned_table_pages: [None; MAX_OWNED_SPANS],
            owned_table_page_count: 0,
            private_mapping_floor: 0,
            private_mapping_ceiling: PROCESS_PRIVATE_LIMIT,
            next_kernel_alloc_virt: KERNEL_ALLOC_BASE,
            kernel_regions: [None; MAX_KERNEL_REGIONS],
            kernel_region_count: 0,
        };
        space.record_owned_span(root_span)?;

        for (entry, value) in template.root_entries[..template.root_entry_count]
            .iter()
            .copied()
            .flatten()
        {
            space.write_table_entry(space.root_table_phys_addr, entry, value)?;
        }

        record.publish();
        Ok(space)
    }

    pub fn map_kernel_region(
        &mut self,
        allocator: &mut BitmapAllocator<'_>,
        virt_start: u64,
        phys_start: PhysAddr,
        page_count: usize,
        flags: EntryFlags,
    ) -> Result<(), PagingError> {
        if self.kernel_region_count >= self.kernel_regions.len() {
            return Err(PagingError::CapacityExceeded);
        }
        map_range(
            self,
            allocator,
            MappingRequest {
                start_virt_addr: virt_start,
                target_phys_start: phys_start,
                page_count,
                flags,
                allow_overwrite: false,
            },
        )?;
        self.kernel_regions[self.kernel_region_count] = Some(KernelRegion {
            start: virt_start,
            end: virt_start + (page_count * PAGE_SIZE) as u64,
            flags,
        });
        self.kernel_region_count += 1;
        let region_end = virt_start + (page_count * PAGE_SIZE) as u64;
        if virt_start >= KERNEL_ALLOC_BASE && region_end <= KERNEL_ALLOC_LIMIT {
            self.next_kernel_alloc_virt = self.next_kernel_alloc_virt.max(region_end);
        }
        Ok(())
    }

    pub fn allocate_kernel_virtual(
        &mut self,
        allocator: &mut BitmapAllocator<'_>,
        page_count: usize,
        flags: EntryFlags,
    ) -> Result<KernelVirtualAllocation, PagingError> {
        let AllocationResult::Allocated(backing_span) = allocator.allocate_pages(page_count) else {
            return Err(PagingError::OutOfMemory);
        };

        let virt_start = self.next_kernel_alloc_virt;
        if virt_start + (page_count * PAGE_SIZE) as u64 > KERNEL_ALLOC_LIMIT {
            match allocator.free_pages(backing_span) {
                AllocationResult::Released(_) => {}
                _ => return Err(PagingError::AllocatorStateCorrupted),
            }
            return Err(PagingError::AddressOutOfRange);
        }

        match map_range(
            self,
            allocator,
            MappingRequest {
                start_virt_addr: virt_start,
                target_phys_start: backing_span.start_phys_addr,
                page_count,
                flags,
                allow_overwrite: false,
            },
        ) {
            Ok(()) => {
                self.next_kernel_alloc_virt += (page_count * PAGE_SIZE) as u64;
                Ok(KernelVirtualAllocation {
                    virt_start_addr: virt_start,
                    backing_span,
                    page_count,
                    flags,
                })
            }
            Err(err) => {
                match allocator.free_pages(backing_span) {
                    AllocationResult::Released(_) => Err(err),
                    _ => Err(PagingError::AllocatorStateCorrupted),
                }
            }
        }
    }

    pub fn destroy(&mut self, allocator: &mut BitmapAllocator<'_>) -> Result<(), PagingError> {
        while self.owned_table_page_count > 0 {
            self.owned_table_page_count -= 1;
            let span = self.owned_table_pages[self.owned_table_page_count]
                .take()
                .expect("owned span tracked");
            match allocator.free_pages(span) {
                AllocationResult::Released(_) => {}
                _ => return Err(PagingError::AllocatorStateCorrupted),
            }
        }
        Ok(())
    }

    pub fn translate(&self, virt_addr: u64) -> Option<MappedPage> {
        let indexes = VirtualAddressLayout::indexes(virt_addr);
        let mut table_phys = self.root_table_phys_addr;

        for (depth, index) in indexes.into_iter().enumerate() {
            let entry = self.read_table_entry(table_phys, index);
            if entry == 0 {
                return None;
            }
            if depth == 3 {
                return Some(MappedPage {
                    virt_addr,
                    phys_addr: (entry & PAGE_TABLE_ADDR_MASK)
                        + VirtualAddressLayout::page_offset(virt_addr) as u64,
                    flags: EntryFlags::from_bits(entry & (EntryFlags::NO_EXECUTE.bits() | 0xfff)),
                });
            }
            table_phys = entry & PAGE_TABLE_ADDR_MASK;
        }

        None
    }

    pub fn kernel_regions(&self) -> impl Iterator<Item = (u64, u64, EntryFlags)> + '_ {
        self.kernel_regions[..self.kernel_region_count]
            .iter()
            .copied()
            .flatten()
            .map(|region| (region.start, region.end, region.flags))
    }

    pub fn managed_phys_limit(&self) -> PhysAddr {
        self.managed_phys_limit
    }

    pub(crate) fn read_table_entry(&self, table_phys: PhysAddr, index: usize) -> u64 {
        read_table_entry_bytes(table_phys, index)
    }

    pub(crate) fn write_table_entry(
        &mut self,
        table_phys: PhysAddr,
        index: usize,
        value: u64,
    ) -> Result<(), PagingError> {
        if index >= PAGE_TABLE_ENTRIES {
            return Err(PagingError::CapacityExceeded);
        }
        write_table_entry_bytes(table_phys, index, value);
        Ok(())
    }

    pub(crate) fn clear_table_entry(&mut self, table_phys: PhysAddr, index: usize) {
        if index < PAGE_TABLE_ENTRIES {
            write_table_entry_bytes(table_phys, index, 0);
        }
    }

    pub(crate) fn allocate_table_page(
        &mut self,
        allocator: &mut BitmapAllocator<'_>,
        record: &mut PagingAllocationRecord,
    ) -> Result<PhysAddr, PagingError> {
        let AllocationResult::Allocated(span) = allocator.allocate_page() else {
            return Err(PagingError::OutOfMemory);
        };
        zero_page_bytes(span.start_phys_addr);
        record.push(span)?;
        Ok(span.start_phys_addr)
    }

    pub(crate) fn record_owned_span(&mut self, span: PageSpan) -> Result<(), PagingError> {
        if self.owned_table_page_count >= self.owned_table_pages.len() {
            return Err(PagingError::CapacityExceeded);
        }
        self.owned_table_pages[self.owned_table_page_count] = Some(span);
        self.owned_table_page_count += 1;
        Ok(())
    }
}

fn allocate_table_span(
    allocator: &mut BitmapAllocator<'_>,
) -> Result<(PageSpan, PagingAllocationRecord), PagingError> {
    let AllocationResult::Allocated(span) = allocator.allocate_page() else {
        return Err(PagingError::OutOfMemory);
    };
    zero_page_bytes(span.start_phys_addr);
    let mut record = PagingAllocationRecord::new();
    record.push(span)?;
    Ok((span, record))
}

#[cfg(target_os = "uefi")]
fn zero_page_bytes(phys_addr: PhysAddr) {
    unsafe {
        core::ptr::write_bytes(phys_addr as *mut u8, 0, PAGE_SIZE);
    }
}

#[cfg(target_os = "uefi")]
fn read_table_entry_bytes(phys_addr: PhysAddr, index: usize) -> u64 {
    unsafe { core::ptr::read_volatile((phys_addr as *const u64).add(index)) }
}

#[cfg(target_os = "uefi")]
fn write_table_entry_bytes(phys_addr: PhysAddr, index: usize, value: u64) {
    unsafe {
        core::ptr::write_volatile((phys_addr as *mut u64).add(index), value);
    }
}

#[cfg(not(target_os = "uefi"))]
extern crate std;

#[cfg(not(target_os = "uefi"))]
use std::{
    boxed::Box,
    cell::RefCell,
    collections::BTreeMap,
};

#[cfg(not(target_os = "uefi"))]
type HostTablePage = Box<[u64; PAGE_TABLE_ENTRIES]>;

#[cfg(not(target_os = "uefi"))]
std::thread_local! {
    static HOST_TABLE_REGISTRY: RefCell<BTreeMap<PhysAddr, HostTablePage>> =
        RefCell::new(BTreeMap::new());
}

#[cfg(not(target_os = "uefi"))]
fn zero_page_bytes(phys_addr: PhysAddr) {
    HOST_TABLE_REGISTRY.with(|registry| {
        registry
            .borrow_mut()
            .entry(phys_addr)
            .and_modify(|entries| entries.fill(0))
            .or_insert_with(|| Box::new([0; PAGE_TABLE_ENTRIES]));
    });
}

#[cfg(not(target_os = "uefi"))]
fn read_table_entry_bytes(phys_addr: PhysAddr, index: usize) -> u64 {
    HOST_TABLE_REGISTRY.with(|registry| {
        registry
            .borrow()
            .get(&phys_addr)
            .map(|entries| entries[index])
            .unwrap_or(0)
    })
}

#[cfg(not(target_os = "uefi"))]
fn write_table_entry_bytes(phys_addr: PhysAddr, index: usize, value: u64) {
    HOST_TABLE_REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();
        let entries = registry
            .entry(phys_addr)
            .or_insert_with(|| Box::new([0; PAGE_TABLE_ENTRIES]));
        entries[index] = value;
    });
}

#[cfg(not(target_os = "uefi"))]
pub fn reset_host_shadow_page_tables_for_tests() {
    HOST_TABLE_REGISTRY.with(|registry| registry.borrow_mut().clear());
}
