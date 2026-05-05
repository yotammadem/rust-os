use core::ffi::c_void;

use rust_os::boot::handoff::{BootInfo, MemoryMapInfo, PhysicalRange};
use rust_os::boot::multiboot::{
    EFI_ABORTED, EFI_BUFFER_TOO_SMALL, EFI_LOADER_DATA, EFI_SUCCESS, EfiHandle, EfiStatus,
    LOADED_IMAGE_PROTOCOL_GUID, LoadedImageProtocol, MemoryDescriptor, SystemTable,
};

pub fn collect(
    image_handle: EfiHandle,
    system_table: *mut SystemTable,
) -> Result<BootInfo, EfiStatus> {
    let boot_services = boot_services(system_table)?;
    let loaded_image = loaded_image(boot_services, image_handle)?;
    let memory_map = memory_map(boot_services)?;

    let image_start = loaded_image.image_base as usize as u64;
    let image_end = image_start + loaded_image.image_size;

    Ok(BootInfo::new(
        PhysicalRange {
            start: image_start,
            end: image_end,
        },
        memory_map,
    ))
}

pub fn exit_boot_services(
    image_handle: EfiHandle,
    system_table: *mut SystemTable,
    boot_info: &mut BootInfo,
) -> Result<(), EfiStatus> {
    let boot_services = boot_services(system_table)?;

    loop {
        let memory_map = memory_map(boot_services)?;
        boot_info.memory_map = memory_map;

        let status =
            unsafe { (boot_services.exit_boot_services)(image_handle, memory_map.map_key) };

        if status == EFI_SUCCESS {
            return Ok(());
        }
    }
}

pub(crate) fn boot_services(
    system_table: *mut SystemTable,
) -> Result<&'static rust_os::boot::multiboot::BootServices, EfiStatus> {
    if system_table.is_null() {
        return Err(EFI_ABORTED);
    }

    let boot_services = unsafe { (*system_table).boot_services };
    if boot_services.is_null() {
        return Err(EFI_ABORTED);
    }

    Ok(unsafe { &*boot_services })
}

pub(crate) fn loaded_image(
    boot_services: &rust_os::boot::multiboot::BootServices,
    image_handle: EfiHandle,
) -> Result<&'static LoadedImageProtocol, EfiStatus> {
    let mut interface: *mut c_void = core::ptr::null_mut();
    let status = unsafe {
        (boot_services.handle_protocol)(image_handle, &LOADED_IMAGE_PROTOCOL_GUID, &mut interface)
    };
    if status != EFI_SUCCESS || interface.is_null() {
        return Err(status);
    }

    Ok(unsafe { &*(interface as *const LoadedImageProtocol) })
}

pub fn memory_map(
    boot_services: &rust_os::boot::multiboot::BootServices,
) -> Result<MemoryMapInfo, EfiStatus> {
    let mut map_size = 0usize;
    let mut map_key = 0usize;
    let mut descriptor_size = 0usize;
    let mut descriptor_version = 0u32;

    let status = unsafe {
        (boot_services.get_memory_map)(
            &mut map_size,
            core::ptr::null_mut(),
            &mut map_key,
            &mut descriptor_size,
            &mut descriptor_version,
        )
    };

    if status != EFI_BUFFER_TOO_SMALL {
        return Err(status);
    }

    map_size += descriptor_size.saturating_mul(2);

    let mut buffer: *mut c_void = core::ptr::null_mut();
    let status = unsafe { (boot_services.allocate_pool)(EFI_LOADER_DATA, map_size, &mut buffer) };
    if status != EFI_SUCCESS || buffer.is_null() {
        return Err(status);
    }

    let status = unsafe {
        (boot_services.get_memory_map)(
            &mut map_size,
            buffer as *mut MemoryDescriptor,
            &mut map_key,
            &mut descriptor_size,
            &mut descriptor_version,
        )
    };
    if status != EFI_SUCCESS {
        return Err(status);
    }

    Ok(MemoryMapInfo {
        map: buffer.cast(),
        map_size,
        map_key,
        descriptor_size,
        descriptor_version,
    })
}
