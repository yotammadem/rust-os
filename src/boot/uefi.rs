use core::{ffi::c_void, mem, ptr};

use crate::memory::map::{
    BootMemoryMapSnapshot, MemoryRegion, PAGE_SIZE, RegionKind, align_down, align_up,
};

pub type EfiHandle = *mut c_void;
pub type EfiStatus = usize;

pub const EFI_SUCCESS: EfiStatus = 0;
pub const EFI_BUFFER_TOO_SMALL: EfiStatus = 0x8000_0000_0000_0005;
pub const EFI_ABORTED: EfiStatus = 0x8000_0000_0000_0015;

const EFI_LOADER_CODE: u32 = 1;
const EFI_LOADER_DATA: u32 = 2;
const EFI_BOOT_SERVICES_CODE: u32 = 3;
const EFI_BOOT_SERVICES_DATA: u32 = 4;
const EFI_CONVENTIONAL_MEMORY: u32 = 7;

#[repr(C)]
pub struct TableHeader {
    pub signature: u64,
    pub revision: u32,
    pub header_size: u32,
    pub crc32: u32,
    pub reserved: u32,
}

#[repr(C)]
pub struct SimpleTextOutputMode {
    pub max_mode: i32,
    pub mode: i32,
    pub attribute: i32,
    pub cursor_column: i32,
    pub cursor_row: i32,
    pub cursor_visible: bool,
}

#[repr(C)]
pub struct SimpleTextOutputProtocol {
    pub reset: usize,
    pub output_string: unsafe extern "efiapi" fn(
        this: *mut SimpleTextOutputProtocol,
        string: *const u16,
    ) -> EfiStatus,
    pub test_string: usize,
    pub query_mode: usize,
    pub set_mode: usize,
    pub set_attribute: usize,
    pub clear_screen: unsafe extern "efiapi" fn(this: *mut SimpleTextOutputProtocol) -> EfiStatus,
    pub set_cursor_position: usize,
    pub enable_cursor: usize,
    pub mode: *mut SimpleTextOutputMode,
}

#[repr(C)]
pub struct MemoryDescriptor {
    pub memory_type: u32,
    pub pad: u32,
    pub physical_start: u64,
    pub virtual_start: u64,
    pub number_of_pages: u64,
    pub attribute: u64,
}

#[repr(C)]
pub struct BootServices {
    pub hdr: TableHeader,
    pub raise_tpl: usize,
    pub restore_tpl: usize,
    pub allocate_pages: usize,
    pub free_pages: usize,
    pub get_memory_map: unsafe extern "efiapi" fn(
        memory_map_size: *mut usize,
        memory_map: *mut MemoryDescriptor,
        map_key: *mut usize,
        descriptor_size: *mut usize,
        descriptor_version: *mut u32,
    ) -> EfiStatus,
}

#[repr(C)]
pub struct SystemTable {
    pub hdr: TableHeader,
    pub firmware_vendor: *mut u16,
    pub firmware_revision: u32,
    pub console_in_handle: EfiHandle,
    pub con_in: *mut c_void,
    pub console_out_handle: EfiHandle,
    pub con_out: *mut SimpleTextOutputProtocol,
    pub standard_error_handle: EfiHandle,
    pub std_err: *mut SimpleTextOutputProtocol,
    pub runtime_services: *mut c_void,
    pub boot_services: *mut BootServices,
    pub number_of_table_entries: usize,
    pub configuration_table: *mut c_void,
}

pub unsafe fn capture_boot_memory_snapshot<'a>(
    system_table: *mut SystemTable,
    raw_memory_map_storage: &mut [u8],
    region_storage: &'a mut [MemoryRegion],
) -> Result<BootMemoryMapSnapshot<'a>, EfiStatus> {
    if system_table.is_null() {
        return Err(EFI_ABORTED);
    }

    let boot_services = unsafe { (*system_table).boot_services };
    if boot_services.is_null() {
        return Err(EFI_ABORTED);
    }

    let mut required_size = 0usize;
    let mut map_key = 0usize;
    let mut descriptor_size = 0usize;
    let mut descriptor_version = 0u32;

    let probe_status = unsafe {
        ((*boot_services).get_memory_map)(
            &mut required_size,
            ptr::null_mut(),
            &mut map_key,
            &mut descriptor_size,
            &mut descriptor_version,
        )
    };

    if probe_status != EFI_BUFFER_TOO_SMALL || descriptor_size < mem::size_of::<MemoryDescriptor>()
    {
        return Err(if probe_status == EFI_SUCCESS {
            EFI_ABORTED
        } else {
            probe_status
        });
    }

    if required_size > raw_memory_map_storage.len() {
        return Err(EFI_BUFFER_TOO_SMALL);
    }

    let mut memory_map_size = raw_memory_map_storage.len();
    let status = unsafe {
        ((*boot_services).get_memory_map)(
            &mut memory_map_size,
            raw_memory_map_storage
                .as_mut_ptr()
                .cast::<MemoryDescriptor>(),
            &mut map_key,
            &mut descriptor_size,
            &mut descriptor_version,
        )
    };
    if status != EFI_SUCCESS {
        return Err(status);
    }

    let descriptor_count = memory_map_size / descriptor_size;
    let descriptor_bytes = &raw_memory_map_storage[..memory_map_size];
    let mut region_count = 0usize;
    let mut highest_usable_address = 0u64;

    for index in 0..descriptor_count {
        let offset = index * descriptor_size;
        let descriptor = unsafe {
            &*descriptor_bytes
                .as_ptr()
                .add(offset)
                .cast::<MemoryDescriptor>()
        };

        let kind = descriptor_kind(descriptor.memory_type);
        let byte_len = descriptor.number_of_pages.saturating_mul(PAGE_SIZE as u64);
        let aligned_start = align_up(descriptor.physical_start, PAGE_SIZE as u64);
        let aligned_end = align_down(
            descriptor.physical_start.saturating_add(byte_len),
            PAGE_SIZE as u64,
        );

        if aligned_end <= aligned_start {
            continue;
        }

        if region_count >= region_storage.len() {
            return Err(EFI_ABORTED);
        }

        if kind == RegionKind::Usable {
            highest_usable_address = highest_usable_address.max(aligned_end);
        }

        region_storage[region_count] =
            MemoryRegion::from_aligned_range(aligned_start, aligned_end, kind);
        region_count += 1;
    }

    Ok(BootMemoryMapSnapshot {
        regions: &region_storage[..region_count],
        descriptor_count,
        descriptor_size,
        page_size: PAGE_SIZE,
        highest_usable_address,
    })
}

fn descriptor_kind(memory_type: u32) -> RegionKind {
    match memory_type {
        EFI_CONVENTIONAL_MEMORY => RegionKind::Usable,
        EFI_LOADER_CODE | EFI_LOADER_DATA => RegionKind::Kernel,
        EFI_BOOT_SERVICES_CODE | EFI_BOOT_SERVICES_DATA => RegionKind::Boot,
        _ => RegionKind::Reserved,
    }
}
