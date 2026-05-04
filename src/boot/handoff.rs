use crate::boot::multiboot::MemoryDescriptor;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BootInfo {
    pub loader_image: LoadedImageRange,
    pub memory_map: MemoryMapInfo,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LoadedImageRange {
    pub start: u64,
    pub end: u64,
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
