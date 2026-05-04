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
