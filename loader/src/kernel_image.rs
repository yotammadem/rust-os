use core::ffi::c_void;

use crate::bootinfo::{boot_services, loaded_image};
use crate::elf::{ElfImage, required_header_bytes};
use rust_os::boot::multiboot::{
    EFI_ABORTED, EFI_BUFFER_TOO_SMALL, EFI_DEVICE_ERROR, EFI_FILE_MODE_READ, EFI_LOADER_DATA,
    EFI_SUCCESS, EFI_VOLUME_CORRUPTED, EfiHandle, EfiStatus, FILE_INFO_GUID, FileInfo,
    FileProtocol, SIMPLE_FILE_SYSTEM_PROTOCOL_GUID, SimpleFileSystemProtocol, SystemTable,
};

const EFI_DIR_UTF16: &[u16] = &['E' as u16, 'F' as u16, 'I' as u16, 0];
const BOOT_DIR_UTF16: &[u16] = &['B' as u16, 'O' as u16, 'O' as u16, 'T' as u16, 0];
const KERNEL_IMAGE_NAME_UTF16: &[u16] = &[
    'K' as u16, 'E' as u16, 'R' as u16, 'N' as u16, 'E' as u16, 'L' as u16, '.' as u16, 'B' as u16,
    'I' as u16, 'N' as u16, 0,
];
const ELF_EHDR_BYTES: usize = 64;
const MAX_LOAD_SEGMENTS: usize = 16;

#[derive(Clone, Copy)]
pub struct LoadedSegment {
    pub file_offset: u64,
    pub physical_start: u64,
    pub physical_end: u64,
    pub virtual_address: u64,
    pub file_size: u64,
    pub memory_size: u64,
    pub flags: u32,
    pub align: u64,
}

impl LoadedSegment {
    const fn empty() -> Self {
        Self {
            file_offset: 0,
            physical_start: 0,
            physical_end: 0,
            virtual_address: 0,
            file_size: 0,
            memory_size: 0,
            flags: 0,
            align: 0,
        }
    }
}

#[derive(Clone, Copy)]
pub struct LoadedKernelImage {
    pub physical_start: u64,
    pub physical_end: u64,
    pub file_size: usize,
    pub entry_point: u64,
    pub segment_count: usize,
    pub segments: [LoadedSegment; MAX_LOAD_SEGMENTS],
}

#[derive(Clone, Copy)]
pub struct LoadError {
    pub stage: &'static [u8],
    pub status: EfiStatus,
}

pub fn load(
    image_handle: EfiHandle,
    system_table: *mut SystemTable,
    physical_start: u64,
    max_size: usize,
) -> Result<LoadedKernelImage, LoadError> {
    let boot_services = boot_services(system_table).map_err(|status| LoadError {
        stage: b"boot_services",
        status,
    })?;
    let loaded_image = loaded_image(boot_services, image_handle).map_err(|status| LoadError {
        stage: b"loaded_image",
        status,
    })?;
    let volume = volume(boot_services, loaded_image.device_handle).map_err(|status| LoadError {
        stage: b"simple_fs",
        status,
    })?;
    let root = open_volume(volume).map_err(|status| LoadError {
        stage: b"open_volume",
        status,
    })?;
    let file = open_kernel_file(root).map_err(|status| LoadError {
        stage: b"open_kernel",
        status,
    })?;

    let result = load_from_file(boot_services, file, physical_start, max_size);
    let _ = unsafe { ((*file).close)(file) };
    let _ = unsafe { ((*root).close)(root) };
    result
}

fn load_from_file(
    boot_services: &rust_os::boot::multiboot::BootServices,
    file: *mut FileProtocol,
    physical_start: u64,
    max_size: usize,
) -> Result<LoadedKernelImage, LoadError> {
    let file_size = file_size(boot_services, file).map_err(|status| LoadError {
        stage: b"file_size",
        status,
    })?;

    let header_size = read_header_size(file).map_err(|status| LoadError {
        stage: b"read_ehdr",
        status,
    })?;
    let header_bytes = allocate_pool(boot_services, header_size).map_err(|status| LoadError {
        stage: b"alloc_ehdr",
        status,
    })?;

    let result = (|| {
        read_exact_at(file, 0, header_bytes).map_err(|status| LoadError {
            stage: b"read_headers",
            status,
        })?;

        let elf_image = ElfImage::parse(header_bytes).map_err(|error| LoadError {
            stage: error.stage,
            status: EFI_ABORTED,
        })?;
        let entry_point = elf_image.entry_point().map_err(|error| LoadError {
            stage: error.stage,
            status: EFI_ABORTED,
        })?;

        let mut raw_segments = [None; MAX_LOAD_SEGMENTS];
        let mut segment_count = 0usize;
        let mut lowest_virtual = u64::MAX;
        let mut highest_virtual = 0u64;

        for segment in elf_image.load_segments() {
            let segment = segment.map_err(|error| LoadError {
                stage: error.stage,
                status: EFI_ABORTED,
            })?;

            if segment_count == MAX_LOAD_SEGMENTS {
                return Err(LoadError {
                    stage: b"segment_capacity",
                    status: EFI_ABORTED,
                });
            }

            let file_end = segment.file_offset.saturating_add(segment.file_size);
            if file_end > file_size as u64 || segment.memory_size < segment.file_size {
                return Err(LoadError {
                    stage: b"segment_bounds",
                    status: EFI_ABORTED,
                });
            }

            lowest_virtual = lowest_virtual.min(segment.virtual_address);
            highest_virtual =
                highest_virtual.max(segment.virtual_address.saturating_add(segment.memory_size));
            raw_segments[segment_count] = Some(segment);
            segment_count += 1;
        }

        if segment_count == 0 || lowest_virtual >= highest_virtual {
            return Err(LoadError {
                stage: b"segment_plan",
                status: EFI_ABORTED,
            });
        }

        let total_span = highest_virtual.saturating_sub(lowest_virtual);
        if total_span == 0 || total_span > max_size as u64 {
            return Err(LoadError {
                stage: b"bounds_check",
                status: EFI_ABORTED,
            });
        }

        let mut segments = [LoadedSegment::empty(); MAX_LOAD_SEGMENTS];
        let mut loaded_end = physical_start;

        for (index, raw_segment) in raw_segments[..segment_count].iter().enumerate() {
            let raw_segment = match raw_segment {
                Some(segment) => *segment,
                None => {
                    return Err(LoadError {
                        stage: b"segment_plan",
                        status: EFI_ABORTED,
                    });
                }
            };

            let offset = raw_segment.virtual_address.saturating_sub(lowest_virtual);
            let physical_segment_start = physical_start.saturating_add(offset);
            let physical_segment_end =
                physical_segment_start.saturating_add(raw_segment.memory_size);
            if physical_segment_end > physical_start.saturating_add(total_span) {
                return Err(LoadError {
                    stage: b"segment_span",
                    status: EFI_ABORTED,
                });
            }

            unsafe {
                core::ptr::write_bytes(
                    physical_segment_start as usize as *mut u8,
                    0,
                    raw_segment.memory_size as usize,
                );
            }

            if raw_segment.file_size != 0 {
                let destination = unsafe {
                    core::slice::from_raw_parts_mut(
                        physical_segment_start as usize as *mut u8,
                        raw_segment.file_size as usize,
                    )
                };
                read_exact_at(file, raw_segment.file_offset, destination).map_err(|status| {
                    LoadError {
                        stage: b"read_segment",
                        status,
                    }
                })?;
            }

            segments[index] = LoadedSegment {
                file_offset: raw_segment.file_offset,
                physical_start: physical_segment_start,
                physical_end: physical_segment_end,
                virtual_address: raw_segment.virtual_address,
                file_size: raw_segment.file_size,
                memory_size: raw_segment.memory_size,
                flags: raw_segment.flags,
                align: raw_segment.align,
            };
            loaded_end = loaded_end.max(physical_segment_end);
        }

        Ok(LoadedKernelImage {
            physical_start,
            physical_end: loaded_end,
            file_size,
            entry_point,
            segment_count,
            segments,
        })
    })();

    let free_status =
        unsafe { (boot_services.free_pool)(header_bytes.as_mut_ptr() as *mut c_void) };
    if free_status != EFI_SUCCESS {
        return Err(LoadError {
            stage: b"free_ehdr",
            status: free_status,
        });
    }

    result
}

fn volume(
    boot_services: &rust_os::boot::multiboot::BootServices,
    device_handle: EfiHandle,
) -> Result<&'static mut SimpleFileSystemProtocol, EfiStatus> {
    let mut interface: *mut c_void = core::ptr::null_mut();
    let status = unsafe {
        (boot_services.handle_protocol)(
            device_handle,
            &SIMPLE_FILE_SYSTEM_PROTOCOL_GUID,
            &mut interface,
        )
    };
    if status != EFI_SUCCESS || interface.is_null() {
        return Err(status);
    }

    Ok(unsafe { &mut *(interface as *mut SimpleFileSystemProtocol) })
}

fn open_volume(volume: &mut SimpleFileSystemProtocol) -> Result<*mut FileProtocol, EfiStatus> {
    let mut root: *mut FileProtocol = core::ptr::null_mut();
    let status = unsafe { (volume.open_volume)(volume, &mut root) };
    if status != EFI_SUCCESS || root.is_null() {
        return Err(status);
    }

    Ok(root)
}

fn open_kernel_file(root: *mut FileProtocol) -> Result<*mut FileProtocol, EfiStatus> {
    let efi_dir = open_component(root, EFI_DIR_UTF16)?;
    let boot_dir = open_component(efi_dir, BOOT_DIR_UTF16)?;
    let file = open_component(boot_dir, KERNEL_IMAGE_NAME_UTF16)?;
    let _ = unsafe { ((*efi_dir).close)(efi_dir) };
    let _ = unsafe { ((*boot_dir).close)(boot_dir) };
    Ok(file)
}

fn open_component(parent: *mut FileProtocol, name: &[u16]) -> Result<*mut FileProtocol, EfiStatus> {
    let mut child: *mut FileProtocol = core::ptr::null_mut();
    let status =
        unsafe { ((*parent).open)(parent, &mut child, name.as_ptr(), EFI_FILE_MODE_READ, 0) };
    if status != EFI_SUCCESS || child.is_null() {
        return Err(status);
    }

    Ok(child)
}

fn file_size(
    boot_services: &rust_os::boot::multiboot::BootServices,
    file: *mut FileProtocol,
) -> Result<usize, EfiStatus> {
    let mut info_size = 0usize;
    let status =
        unsafe { ((*file).get_info)(file, &FILE_INFO_GUID, &mut info_size, core::ptr::null_mut()) };

    if status != EFI_BUFFER_TOO_SMALL {
        return Err(status);
    }

    let mut buffer: *mut c_void = core::ptr::null_mut();
    let status = unsafe { (boot_services.allocate_pool)(EFI_LOADER_DATA, info_size, &mut buffer) };
    if status != EFI_SUCCESS || buffer.is_null() {
        return Err(status);
    }

    let status = unsafe { ((*file).get_info)(file, &FILE_INFO_GUID, &mut info_size, buffer) };
    if status != EFI_SUCCESS {
        let _ = unsafe { (boot_services.free_pool)(buffer) };
        return Err(status);
    }

    let info = unsafe { &*(buffer as *const FileInfo) };
    let size = usize::try_from(info.file_size).map_err(|_| EFI_ABORTED)?;
    let free_status = unsafe { (boot_services.free_pool)(buffer) };
    if free_status != EFI_SUCCESS {
        return Err(EFI_DEVICE_ERROR);
    }

    if size == 0 {
        return Err(EFI_VOLUME_CORRUPTED);
    }

    Ok(size)
}

fn read_header_size(file: *mut FileProtocol) -> Result<usize, EfiStatus> {
    let mut header = [0u8; ELF_EHDR_BYTES];
    read_exact_at(file, 0, &mut header)?;
    let required = required_header_bytes(&header).map_err(|_| EFI_ABORTED)?;
    if required < ELF_EHDR_BYTES {
        return Err(EFI_ABORTED);
    }
    Ok(required)
}

fn allocate_pool(
    boot_services: &rust_os::boot::multiboot::BootServices,
    size: usize,
) -> Result<&'static mut [u8], EfiStatus> {
    let mut buffer: *mut c_void = core::ptr::null_mut();
    let status = unsafe { (boot_services.allocate_pool)(EFI_LOADER_DATA, size, &mut buffer) };
    if status != EFI_SUCCESS || buffer.is_null() {
        return Err(status);
    }

    Ok(unsafe { core::slice::from_raw_parts_mut(buffer as *mut u8, size) })
}

fn read_exact_at(file: *mut FileProtocol, offset: u64, buffer: &mut [u8]) -> Result<(), EfiStatus> {
    let status = unsafe { ((*file).set_position)(file, offset) };
    if status != EFI_SUCCESS {
        return Err(status);
    }

    let mut bytes_to_read = buffer.len();
    let status =
        unsafe { ((*file).read)(file, &mut bytes_to_read, buffer.as_mut_ptr() as *mut c_void) };
    if status != EFI_SUCCESS || bytes_to_read != buffer.len() {
        return Err(status);
    }

    Ok(())
}
