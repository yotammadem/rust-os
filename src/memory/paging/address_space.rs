use crate::memory::{AllocationResult, BitmapAllocator, PAGE_SIZE, PageSpan, PhysAddr};

use super::mapper::{PagingError, map_range};
use super::table::{
    EntryFlags, MappedPage, MappingRequest, PAGE_TABLE_ENTRIES, PageTableLevel,
    VirtualAddressLayout, KERNEL_ALLOC_BASE, KERNEL_ALLOC_LIMIT, KERNEL_DIRECT_MAP_BASE,
    KERNEL_VIRT_BASE, PAGE_TABLE_ADDR_MASK, PROCESS_PRIVATE_LIMIT,
};

const MAX_TABLE_PAGES: usize = 16;
const MAX_TRACKED_ENTRIES_PER_TABLE: usize = PAGE_TABLE_ENTRIES;
const MAX_OWNED_SPANS: usize = 64;
const MAX_KERNEL_REGIONS: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddressSpaceKind {
    Kernel,
    Process,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableOwner {
    AddressSpace,
    SharedKernel,
}

impl From<AddressSpaceKind> for TableOwner {
    fn from(value: AddressSpaceKind) -> Self {
        match value {
            AddressSpaceKind::Kernel => Self::SharedKernel,
            AddressSpaceKind::Process => Self::AddressSpace,
        }
    }
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
pub struct TablePage {
    pub phys_addr: PhysAddr,
    pub level: PageTableLevel,
    owner: TableOwner,
    entries: [u64; MAX_TRACKED_ENTRIES_PER_TABLE],
}

impl TablePage {
    const EMPTY: Self = Self {
        phys_addr: 0,
        level: PageTableLevel::Pml4,
        owner: TableOwner::AddressSpace,
        entries: [0; MAX_TRACKED_ENTRIES_PER_TABLE],
    };

    fn new(phys_addr: PhysAddr, level: PageTableLevel, owner: TableOwner) -> Self {
        let table = Self {
            phys_addr,
            level,
            owner,
            entries: [0; MAX_TRACKED_ENTRIES_PER_TABLE],
        };
        table.zero_physical_page();
        table
    }

    pub(crate) fn get(&self, index: usize) -> u64 {
        self.entries[index]
    }

    pub(crate) fn set(&mut self, index: usize, value: u64) -> Result<(), PagingError> {
        if index >= self.entries.len() {
            return Err(PagingError::CapacityExceeded);
        }
        self.entries[index] = value;
        self.write_physical_entry(index, value);
        Ok(())
    }

    pub(crate) fn clear(&mut self, index: usize) {
        if index < self.entries.len() {
            self.entries[index] = 0;
        }
        self.write_physical_entry(index, 0);
    }

    fn zero_physical_page(&self) {
        zero_page_bytes(self.phys_addr);
    }

    fn write_physical_entry(&self, index: usize, value: u64) {
        write_entry_bytes(self.phys_addr, index, value);
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
    root_entries: [Option<(usize, u64)>; MAX_KERNEL_REGIONS],
    root_entry_count: usize,
    shared_tables: [TablePage; MAX_TABLE_PAGES],
    shared_table_count: usize,
    pub managed_phys_limit: PhysAddr,
    pub transition_alias_start: u64,
    pub transition_alias_page_count: usize,
}

impl KernelMappingTemplate {
    pub const fn empty() -> Self {
        Self {
            root_entries: [None; MAX_KERNEL_REGIONS],
            root_entry_count: 0,
            shared_tables: [TablePage::EMPTY; MAX_TABLE_PAGES],
            shared_table_count: 0,
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
    tables: [TablePage; MAX_TABLE_PAGES],
    table_count: usize,
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
            tables: [TablePage::EMPTY; MAX_TABLE_PAGES],
            table_count: 0,
            owned_table_pages: [None; MAX_OWNED_SPANS],
            owned_table_page_count: 0,
            private_mapping_floor: 0,
            private_mapping_ceiling: PROCESS_PRIVATE_LIMIT,
            next_kernel_alloc_virt: KERNEL_ALLOC_BASE,
            kernel_regions: [None; MAX_KERNEL_REGIONS],
            kernel_region_count: 0,
        };
        space.install_table(root_span.start_phys_addr, PageTableLevel::Pml4, TableOwner::AddressSpace)?;
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
            EntryFlags::PRESENT | EntryFlags::WRITABLE | EntryFlags::GLOBAL | EntryFlags::NO_EXECUTE,
        )?;
        kernel.map_kernel_region(
            allocator,
            KERNEL_VIRT_BASE,
            kernel_phys_start,
            kernel_page_count,
            EntryFlags::PRESENT | EntryFlags::WRITABLE,
        )?;
        let alias_start = kernel_phys_start;
        kernel.map_kernel_region(
            allocator,
            alias_start,
            kernel_phys_start,
            kernel_page_count,
            EntryFlags::PRESENT | EntryFlags::WRITABLE,
        )?;

        let mut template = KernelMappingTemplate::empty();
        template.managed_phys_limit = kernel.managed_phys_limit;
        let root = kernel.root_table();
        for entry in 0..512 {
            let value = root.get(entry);
            if value != 0 && entry >= VirtualAddressLayout::pml4_index(KERNEL_VIRT_BASE) {
                if template.root_entry_count >= template.root_entries.len() {
                    return Err(PagingError::CapacityExceeded);
                }
                template.root_entries[template.root_entry_count] = Some((entry, value));
                template.root_entry_count += 1;
            }
        }
        for table in &kernel.tables[..kernel.table_count] {
            if table.owner == TableOwner::SharedKernel {
                template.shared_tables[template.shared_table_count] = *table;
                template.shared_table_count += 1;
            }
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
            tables: [TablePage::EMPTY; MAX_TABLE_PAGES],
            table_count: 0,
            owned_table_pages: [None; MAX_OWNED_SPANS],
            owned_table_page_count: 0,
            private_mapping_floor: 0,
            private_mapping_ceiling: PROCESS_PRIVATE_LIMIT,
            next_kernel_alloc_virt: KERNEL_ALLOC_BASE,
            kernel_regions: [None; MAX_KERNEL_REGIONS],
            kernel_region_count: 0,
        };
        space.install_table(root_span.start_phys_addr, PageTableLevel::Pml4, TableOwner::AddressSpace)?;
        space.record_owned_span(root_span)?;

        for table in &template.shared_tables[..template.shared_table_count] {
            space.install_existing_table(*table)?;
        }
        for (entry, value) in template.root_entries[..template.root_entry_count]
            .iter()
            .copied()
            .flatten()
        {
            space.root_table_mut().set(entry, value)?;
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
            TableOwner::SharedKernel,
        )?;
        self.kernel_regions[self.kernel_region_count] = Some(KernelRegion {
            start: virt_start,
            end: virt_start + (page_count * PAGE_SIZE) as u64,
            flags,
        });
        self.kernel_region_count += 1;
        self.next_kernel_alloc_virt = self.next_kernel_alloc_virt.max(
            virt_start + (page_count * PAGE_SIZE) as u64,
        );
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
            TableOwner::AddressSpace,
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
        self.table_count = 0;
        Ok(())
    }

    pub fn translate(&self, virt_addr: u64) -> Option<MappedPage> {
        let indexes = VirtualAddressLayout::indexes(virt_addr);
        let mut table_phys = self.root_table_phys_addr;
        for (depth, index) in indexes.into_iter().enumerate() {
            let table = self.find_table(table_phys)?;
            let entry = table.get(index);
            if entry == 0 {
                return None;
            }
            if depth == 3 {
                let phys =
                    (entry & PAGE_TABLE_ADDR_MASK) + VirtualAddressLayout::page_offset(virt_addr) as u64;
                return Some(MappedPage {
                    virt_addr,
                    phys_addr: phys,
                    flags: EntryFlags::from_bits(entry & (EntryFlags::NO_EXECUTE.bits() | 0xfff)),
                });
            }
            table_phys = entry & !0xfff;
        }
        None
    }

    pub fn root_table(&self) -> &TablePage {
        self.find_table(self.root_table_phys_addr).expect("root table present")
    }

    pub fn root_table_mut(&mut self) -> &mut TablePage {
        self.find_table_mut(self.root_table_phys_addr)
            .expect("root table present")
    }

    pub fn find_table(&self, phys_addr: PhysAddr) -> Option<&TablePage> {
        self.tables[..self.table_count]
            .iter()
            .find(|table| table.phys_addr == phys_addr)
    }

    pub fn find_table_mut(&mut self, phys_addr: PhysAddr) -> Option<&mut TablePage> {
        self.tables[..self.table_count]
            .iter_mut()
            .find(|table| table.phys_addr == phys_addr)
    }

    pub fn install_table(
        &mut self,
        phys_addr: PhysAddr,
        level: PageTableLevel,
        owner: TableOwner,
    ) -> Result<(), PagingError> {
        if self.table_count >= self.tables.len() {
            return Err(PagingError::CapacityExceeded);
        }
        self.tables[self.table_count] = TablePage::new(phys_addr, level, owner);
        self.table_count += 1;
        Ok(())
    }

    pub fn install_existing_table(&mut self, table: TablePage) -> Result<(), PagingError> {
        if self.table_count >= self.tables.len() {
            return Err(PagingError::CapacityExceeded);
        }
        self.tables[self.table_count] = table;
        self.table_count += 1;
        Ok(())
    }

    pub fn record_owned_span(&mut self, span: PageSpan) -> Result<(), PagingError> {
        if self.owned_table_page_count >= self.owned_table_pages.len() {
            return Err(PagingError::CapacityExceeded);
        }
        self.owned_table_pages[self.owned_table_page_count] = Some(span);
        self.owned_table_page_count += 1;
        Ok(())
    }

    pub fn clone_shared_kernel_tables(&self) -> impl Iterator<Item = TablePage> + '_ {
        self.tables[..self.table_count]
            .iter()
            .copied()
            .filter(|table| table.owner == TableOwner::SharedKernel)
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
}

fn allocate_table_span(
    allocator: &mut BitmapAllocator<'_>,
) -> Result<(PageSpan, PagingAllocationRecord), PagingError> {
    let AllocationResult::Allocated(span) = allocator.allocate_page() else {
        return Err(PagingError::OutOfMemory);
    };
    let mut record = PagingAllocationRecord::new();
    record.push(span)?;
    Ok((span, record))
}

#[cfg(target_os = "uefi")]
fn zero_page_bytes(phys_addr: PhysAddr) {
    unsafe {
        // Safety: boot-time paging pages are allocator-owned 4 KiB frames and are only
        // initialized through this helper before or while publishing page-table entries.
        core::ptr::write_bytes(phys_addr as *mut u8, 0, PAGE_SIZE);
    }
}

#[cfg(not(target_os = "uefi"))]
fn zero_page_bytes(_phys_addr: PhysAddr) {}

#[cfg(target_os = "uefi")]
fn write_entry_bytes(phys_addr: PhysAddr, index: usize, value: u64) {
    unsafe {
        // Safety: the caller bounds `index` to a page-table entry slot and only publishes
        // aligned u64 entries into allocator-owned paging frames.
        core::ptr::write_volatile((phys_addr as *mut u64).add(index), value);
    }
}

#[cfg(not(target_os = "uefi"))]
fn write_entry_bytes(_phys_addr: PhysAddr, _index: usize, _value: u64) {}
