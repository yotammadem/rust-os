use core::slice;

use crate::memory::map::{
    AllocationResult, BootMemoryMapSnapshot, MemoryRegion, PAGE_SIZE, PageSpan, RegionKind,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InitError {
    EmptyMemoryMap,
    InvalidRegionLayout,
    MetadataPlacementFailed,
    StorageTooSmall,
}

pub struct BitmapAllocator<'a> {
    used_bits: &'a mut [u8],
    allocatable_bits: &'a mut [u8],
    managed_page_count: usize,
    free_page_count: usize,
    metadata_span: PageSpan,
    last_search_page: usize,
}

impl<'a> BitmapAllocator<'a> {
    pub fn from_regions(
        regions: &[MemoryRegion],
        storage: &'a mut [u8],
    ) -> Result<Self, InitError> {
        let managed_page_count = managed_page_count(regions);
        if managed_page_count == 0 {
            return Err(InitError::EmptyMemoryMap);
        }

        validate_regions(regions)?;
        let metadata_pages = Self::metadata_page_count_for_managed_pages(managed_page_count);
        let metadata_span = select_metadata_span(regions, metadata_pages)
            .ok_or(InitError::MetadataPlacementFailed)?;

        Self::initialize(regions, storage, managed_page_count, metadata_span)
    }

    pub fn required_storage_bytes_for_managed_pages(managed_page_count: usize) -> usize {
        let bitmap_len = bitmap_len_bytes(managed_page_count);
        bitmap_len * 2
    }

    pub fn metadata_page_count_for_managed_pages(managed_page_count: usize) -> usize {
        let total = Self::required_storage_bytes_for_managed_pages(managed_page_count);
        total.div_ceil(PAGE_SIZE).max(1)
    }

    pub fn free_page_count(&self) -> usize {
        self.free_page_count
    }

    pub fn managed_page_count(&self) -> usize {
        self.managed_page_count
    }

    pub fn metadata_span(&self) -> PageSpan {
        self.metadata_span
    }

    pub fn is_page_allocatable(&self, page_index: usize) -> bool {
        page_index < self.managed_page_count && bit_is_set(self.allocatable_bits, page_index)
    }

    pub fn is_page_free(&self, page_index: usize) -> bool {
        self.is_page_allocatable(page_index) && !bit_is_set(self.used_bits, page_index)
    }

    pub fn allocate_page(&mut self) -> AllocationResult {
        self.allocate_pages(1)
    }

    pub fn allocate_pages(&mut self, page_count: usize) -> AllocationResult {
        if page_count == 0 {
            return AllocationResult::InvalidRequest;
        }
        if page_count > self.free_page_count {
            return AllocationResult::OutOfMemory;
        }

        let start = self
            .find_free_run(page_count)
            .or_else(|| self.find_free_run_from(0, page_count));
        let Some(start_page_index) = start else {
            return AllocationResult::OutOfMemory;
        };

        for page_index in start_page_index..start_page_index + page_count {
            set_bit(self.used_bits, page_index, true);
        }

        self.free_page_count -= page_count;
        self.last_search_page = start_page_index + page_count;

        AllocationResult::Allocated(
            PageSpan::new(start_page_index, page_count).expect("page_count validated"),
        )
    }

    pub fn free_pages(&mut self, span: PageSpan) -> AllocationResult {
        if span.page_count == 0 || span.start_page_index + span.page_count > self.managed_page_count
        {
            return AllocationResult::InvalidFree;
        }

        for page_index in span.start_page_index..span.start_page_index + span.page_count {
            if !bit_is_set(self.allocatable_bits, page_index)
                || !bit_is_set(self.used_bits, page_index)
            {
                return AllocationResult::InvalidFree;
            }
        }

        for page_index in span.start_page_index..span.start_page_index + span.page_count {
            set_bit(self.used_bits, page_index, false);
        }

        self.free_page_count += span.page_count;
        self.last_search_page = span.start_page_index;

        AllocationResult::Released(span)
    }

    fn initialize(
        regions: &[MemoryRegion],
        storage: &'a mut [u8],
        managed_page_count: usize,
        metadata_span: PageSpan,
    ) -> Result<Self, InitError> {
        let required_storage_bytes =
            Self::required_storage_bytes_for_managed_pages(managed_page_count);
        if storage.len() < required_storage_bytes {
            return Err(InitError::StorageTooSmall);
        }

        storage[..required_storage_bytes].fill(0);
        let (used_bits, allocatable_bits) =
            storage[..required_storage_bytes].split_at_mut(bitmap_len_bytes(managed_page_count));
        used_bits.fill(0xff);
        allocatable_bits.fill(0x00);

        let mut allocator = Self {
            used_bits,
            allocatable_bits,
            managed_page_count,
            free_page_count: 0,
            metadata_span,
            last_search_page: 0,
        };

        for region in regions {
            allocator.apply_region(*region);
        }

        for page_index in metadata_span.start_page_index
            ..metadata_span.start_page_index + metadata_span.page_count
        {
            if allocator.is_page_free(page_index) {
                allocator.free_page_count -= 1;
            }
            set_bit(allocator.allocatable_bits, page_index, false);
            set_bit(allocator.used_bits, page_index, true);
        }

        Ok(allocator)
    }

    fn apply_region(&mut self, region: MemoryRegion) {
        let end_page_index = region.end_page_index().min(self.managed_page_count);

        for page_index in region.start_page_index..end_page_index {
            match region.kind {
                RegionKind::Usable => {
                    if !self.metadata_span.contains_page(page_index) {
                        if !bit_is_set(self.allocatable_bits, page_index) {
                            self.free_page_count += 1;
                        }
                        set_bit(self.allocatable_bits, page_index, true);
                        set_bit(self.used_bits, page_index, false);
                    }
                }
                _ => {
                    if self.is_page_free(page_index) {
                        self.free_page_count -= 1;
                    }
                    set_bit(self.allocatable_bits, page_index, false);
                    set_bit(self.used_bits, page_index, true);
                }
            }
        }
    }

    fn find_free_run(&self, page_count: usize) -> Option<usize> {
        self.find_free_run_from(self.last_search_page, page_count)
    }

    fn find_free_run_from(&self, start: usize, page_count: usize) -> Option<usize> {
        let mut run_start = 0usize;
        let mut run_len = 0usize;

        for page_index in start..self.managed_page_count {
            if self.is_page_free(page_index) {
                if run_len == 0 {
                    run_start = page_index;
                }
                run_len += 1;
                if run_len == page_count {
                    return Some(run_start);
                }
            } else {
                run_len = 0;
            }
        }

        None
    }
}

impl BitmapAllocator<'static> {
    pub unsafe fn from_boot_snapshot(
        snapshot: &BootMemoryMapSnapshot<'_>,
    ) -> Result<Self, InitError> {
        let managed_page_count = snapshot.managed_page_count();
        if managed_page_count == 0 {
            return Err(InitError::EmptyMemoryMap);
        }

        validate_regions(snapshot.regions)?;
        let metadata_pages = Self::metadata_page_count_for_managed_pages(managed_page_count);
        let metadata_span = select_metadata_span(snapshot.regions, metadata_pages)
            .ok_or(InitError::MetadataPlacementFailed)?;
        let storage_len = metadata_pages * PAGE_SIZE;
        let storage = unsafe {
            // Safety: metadata_span points at allocator-reserved physical pages selected from a
            // usable region. This is the only place that aliases that memory as raw bitmap storage.
            slice::from_raw_parts_mut(metadata_span.start_phys_addr as *mut u8, storage_len)
        };

        Self::initialize(snapshot.regions, storage, managed_page_count, metadata_span)
    }
}

fn managed_page_count(regions: &[MemoryRegion]) -> usize {
    regions
        .iter()
        .map(MemoryRegion::end_page_index)
        .max()
        .unwrap_or(0)
}

fn validate_regions(regions: &[MemoryRegion]) -> Result<(), InitError> {
    for (index, region) in regions.iter().enumerate() {
        if region.page_count == 0 {
            continue;
        }

        for other in regions.iter().skip(index + 1) {
            if other.page_count == 0 {
                continue;
            }

            let overlaps = region.start_page_index < other.end_page_index()
                && other.start_page_index < region.end_page_index();
            if overlaps {
                return Err(InitError::InvalidRegionLayout);
            }
        }
    }

    Ok(())
}

fn select_metadata_span(regions: &[MemoryRegion], required_pages: usize) -> Option<PageSpan> {
    regions
        .iter()
        .find(|region| region.kind == RegionKind::Usable && region.page_count >= required_pages)
        .and_then(|region| PageSpan::new(region.start_page_index, required_pages))
}

fn bitmap_len_bytes(managed_page_count: usize) -> usize {
    managed_page_count.div_ceil(8)
}

fn bit_is_set(bytes: &[u8], bit_index: usize) -> bool {
    let byte_index = bit_index / 8;
    let bit_offset = bit_index % 8;
    bytes
        .get(byte_index)
        .map(|byte| byte & (1 << bit_offset) != 0)
        .unwrap_or(false)
}

fn set_bit(bytes: &mut [u8], bit_index: usize, value: bool) {
    let byte_index = bit_index / 8;
    let bit_offset = bit_index % 8;
    let mask = 1 << bit_offset;

    if let Some(byte) = bytes.get_mut(byte_index) {
        if value {
            *byte |= mask;
        } else {
            *byte &= !mask;
        }
    }
}
