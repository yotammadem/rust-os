pub const PAGE_SIZE: usize = 4096;
pub const MAX_MEMORY_REGIONS: usize = 128;
pub const UEFI_MEMORY_MAP_STORAGE_BYTES: usize = 16 * 1024;

pub type PhysAddr = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegionKind {
    Usable,
    Reserved,
    Kernel,
    Boot,
    AllocatorMetadata,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryRegion {
    pub start_phys_addr: PhysAddr,
    pub length_bytes: u64,
    pub start_page_index: usize,
    pub page_count: usize,
    pub kind: RegionKind,
}

impl MemoryRegion {
    pub const EMPTY: Self = Self {
        start_phys_addr: 0,
        length_bytes: 0,
        start_page_index: 0,
        page_count: 0,
        kind: RegionKind::Reserved,
    };

    pub const fn from_aligned_range(
        start_phys_addr: PhysAddr,
        end_phys_addr_exclusive: PhysAddr,
        kind: RegionKind,
    ) -> Self {
        let start_page_index = (start_phys_addr as usize) / PAGE_SIZE;
        let page_count = ((end_phys_addr_exclusive - start_phys_addr) as usize) / PAGE_SIZE;

        Self {
            start_phys_addr,
            length_bytes: end_phys_addr_exclusive - start_phys_addr,
            start_page_index,
            page_count,
            kind,
        }
    }

    pub fn normalized(
        start_phys_addr: PhysAddr,
        length_bytes: u64,
        kind: RegionKind,
    ) -> Option<Self> {
        let aligned_start = align_up(start_phys_addr, PAGE_SIZE as u64);
        let aligned_end = align_down(
            start_phys_addr.saturating_add(length_bytes),
            PAGE_SIZE as u64,
        );

        if aligned_end <= aligned_start {
            return None;
        }

        Some(Self::from_aligned_range(aligned_start, aligned_end, kind))
    }

    pub const fn end_phys_addr_exclusive(&self) -> PhysAddr {
        self.start_phys_addr + self.length_bytes
    }

    pub const fn end_page_index(&self) -> usize {
        self.start_page_index + self.page_count
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BootMemoryMapSnapshot<'a> {
    pub regions: &'a [MemoryRegion],
    pub descriptor_count: usize,
    pub descriptor_size: usize,
    pub page_size: usize,
    pub highest_usable_address: PhysAddr,
}

impl BootMemoryMapSnapshot<'_> {
    pub fn managed_page_count(&self) -> usize {
        (self.highest_usable_address as usize).div_ceil(PAGE_SIZE)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageSpan {
    pub start_page_index: usize,
    pub page_count: usize,
    pub start_phys_addr: PhysAddr,
    pub end_phys_addr_exclusive: PhysAddr,
}

impl PageSpan {
    pub fn new(start_page_index: usize, page_count: usize) -> Option<Self> {
        if page_count == 0 {
            return None;
        }

        let start_phys_addr = (start_page_index * PAGE_SIZE) as PhysAddr;
        let end_phys_addr_exclusive = ((start_page_index + page_count) * PAGE_SIZE) as PhysAddr;

        Some(Self {
            start_page_index,
            page_count,
            start_phys_addr,
            end_phys_addr_exclusive,
        })
    }

    pub const fn contains_page(&self, page_index: usize) -> bool {
        page_index >= self.start_page_index && page_index < self.start_page_index + self.page_count
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AllocationResult {
    Allocated(PageSpan),
    Released(PageSpan),
    OutOfMemory,
    InvalidRequest,
    InvalidFree,
}

pub const fn align_up(value: u64, alignment: u64) -> u64 {
    if alignment == 0 {
        return value;
    }

    let remainder = value % alignment;
    if remainder == 0 {
        value
    } else {
        value + (alignment - remainder)
    }
}

pub const fn align_down(value: u64, alignment: u64) -> u64 {
    if alignment == 0 {
        value
    } else {
        value - (value % alignment)
    }
}
