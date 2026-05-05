use crate::boot::multiboot::MemoryDescriptor;

pub const MAX_USABLE_RANGES: usize = 8;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BootInfo {
    pub loader_image: PhysicalRange,
    pub memory_map: MemoryMapInfo,
    pub usable_range_count: usize,
    pub usable_ranges: [PhysicalRange; MAX_USABLE_RANGES],
    pub kernel_image: KernelImageInfo,
    pub paging: PagingInfo,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PhysicalRange {
    pub start: u64,
    pub end: u64,
}

impl PhysicalRange {
    pub const fn empty() -> Self {
        Self { start: 0, end: 0 }
    }

    pub fn size_bytes(&self) -> u64 {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct KernelImageInfo {
    pub physical_range: PhysicalRange,
    pub virtual_range: PhysicalRange,
    pub entry_point: u64,
}

impl KernelImageInfo {
    pub const fn empty() -> Self {
        Self {
            physical_range: PhysicalRange::empty(),
            virtual_range: PhysicalRange::empty(),
            entry_point: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PagingInfo {
    pub pml4_physical_start: u64,
    pub kernel_stack_physical: PhysicalRange,
    pub kernel_stack_virtual: PhysicalRange,
}

impl PagingInfo {
    pub const fn empty() -> Self {
        Self {
            pml4_physical_start: 0,
            kernel_stack_physical: PhysicalRange::empty(),
            kernel_stack_virtual: PhysicalRange::empty(),
        }
    }
}

impl BootInfo {
    pub const fn new(loader_image: PhysicalRange, memory_map: MemoryMapInfo) -> Self {
        Self {
            loader_image,
            memory_map,
            usable_range_count: 0,
            usable_ranges: [PhysicalRange::empty(); MAX_USABLE_RANGES],
            kernel_image: KernelImageInfo::empty(),
            paging: PagingInfo::empty(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MemoryMapInfo {
    pub map: *const u8,
    pub map_size: usize,
    pub map_key: usize,
    pub descriptor_size: usize,
    pub descriptor_version: u32,
}

impl MemoryMapInfo {
    pub fn descriptor_count(&self) -> usize {
        self.map_size / self.descriptor_size
    }

    pub fn descriptors(&self) -> MemoryMapIter {
        MemoryMapIter {
            current: self.map,
            remaining: self.descriptor_count(),
            descriptor_size: self.descriptor_size,
        }
    }
}

pub struct MemoryMapIter {
    current: *const u8,
    remaining: usize,
    descriptor_size: usize,
}

impl Iterator for MemoryMapIter {
    type Item = &'static MemoryDescriptor;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }

        let descriptor = unsafe { &*(self.current as *const MemoryDescriptor) };
        self.current = unsafe { self.current.add(self.descriptor_size) };
        self.remaining -= 1;
        Some(descriptor)
    }
}
