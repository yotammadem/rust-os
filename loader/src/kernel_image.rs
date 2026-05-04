use core::ffi::c_void;

use crate::bootinfo::{boot_services, loaded_image};
use rust_os::boot::multiboot::{
    EFI_ABORTED, EFI_BUFFER_TOO_SMALL, EFI_BY_PROTOCOL, EFI_DEVICE_ERROR, EFI_FILE_MODE_READ,
    EFI_LOADER_DATA, EFI_NOT_FOUND, EFI_SUCCESS, EFI_VOLUME_CORRUPTED, EfiHandle, EfiStatus,
    FILE_INFO_GUID, FileInfo, FileProtocol, SIMPLE_FILE_SYSTEM_PROTOCOL_GUID,
    SimpleFileSystemProtocol, SystemTable,
};

const EFI_DIR_UTF16: &[u16] = &['E' as u16, 'F' as u16, 'I' as u16, 0];
const BOOT_DIR_UTF16: &[u16] = &['B' as u16, 'O' as u16, 'O' as u16, 'T' as u16, 0];
const KERNEL_IMAGE_NAME_UTF16: &[u16] = &[
    'K' as u16, 'E' as u16, 'R' as u16, 'N' as u16, 'E' as u16, 'L' as u16, '.' as u16, 'B' as u16,
    'I' as u16, 'N' as u16, 0,
];

#[derive(Clone, Copy)]
pub struct LoadedKernelImage {
    pub physical_start: u64,
    pub physical_end: u64,
    pub file_size: usize,
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
    let file = find_kernel_file(boot_services, loaded_image.device_handle).map_err(|error| error)?;
    let file_size = file_size(boot_services, file).map_err(|status| LoadError {
        stage: b"file_size",
        status,
    })?;

    if file_size == 0 || file_size > max_size {
        return Err(LoadError {
            stage: b"bounds_check",
            status: EFI_ABORTED,
        });
    }

    let mut bytes_to_read = file_size;
    let status = unsafe {
        ((*file).read)(
            file,
            &mut bytes_to_read,
            physical_start as usize as *mut c_void,
        )
    };
    let _ = unsafe { ((*file).close)(file) };
    if status != EFI_SUCCESS || bytes_to_read != file_size {
        return Err(LoadError {
            stage: b"read",
            status,
        });
    }

    Ok(LoadedKernelImage {
        physical_start,
        physical_end: physical_start + file_size as u64,
        file_size,
    })
}

fn find_kernel_file(
    boot_services: &rust_os::boot::multiboot::BootServices,
    preferred_handle: EfiHandle,
) -> Result<*mut FileProtocol, LoadError> {
    if !preferred_handle.is_null() {
        if let Ok(volume) = volume(boot_services, preferred_handle) {
            if let Ok(root) = open_volume(volume) {
                if let Ok(file) = open_kernel_file(root) {
                    let _ = unsafe { ((*root).close)(root) };
                    return Ok(file);
                }
                let _ = unsafe { ((*root).close)(root) };
            }
        }
    }

    let mut handle_count = 0usize;
    let mut handles: *mut EfiHandle = core::ptr::null_mut();
    let status = unsafe {
        (boot_services.locate_handle_buffer)(
            EFI_BY_PROTOCOL,
            &SIMPLE_FILE_SYSTEM_PROTOCOL_GUID,
            core::ptr::null_mut(),
            &mut handle_count,
            &mut handles,
        )
    };
    if status != EFI_SUCCESS || handles.is_null() {
        return Err(LoadError {
            stage: b"locate_fs",
            status,
        });
    }

    let handle_slice = unsafe { core::slice::from_raw_parts(handles, handle_count) };
    for &handle in handle_slice {
        let Ok(volume) = volume(boot_services, handle) else {
            continue;
        };
        let Ok(root) = open_volume(volume) else {
            continue;
        };
        if let Ok(file) = open_kernel_file(root) {
            let _ = unsafe { ((*root).close)(root) };
            let _ = unsafe { (boot_services.free_pool)(handles.cast()) };
            return Ok(file);
        }
        let _ = unsafe { ((*root).close)(root) };
    }

    let _ = unsafe { (boot_services.free_pool)(handles.cast()) };
    Err(LoadError {
        stage: b"find_kernel",
        status: EFI_NOT_FOUND,
    })
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
