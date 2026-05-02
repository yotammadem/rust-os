#![cfg_attr(target_os = "uefi", no_std)]
#![cfg_attr(target_os = "uefi", no_main)]

#[cfg(target_os = "uefi")]
use core::panic::PanicInfo;
#[cfg(target_os = "uefi")]
use rust_os::{
    arch::x86_64::{halt, serial::SerialPort},
    boot::uefi::{EFI_ABORTED, EfiHandle, EfiStatus, SystemTable, capture_boot_memory_snapshot},
    kernel::hello,
    memory::{BitmapAllocator, MAX_MEMORY_REGIONS, MemoryRegion, UEFI_MEMORY_MAP_STORAGE_BYTES},
};

#[cfg(not(target_os = "uefi"))]
fn main() {}

#[cfg(target_os = "uefi")]
#[unsafe(no_mangle)]
pub extern "efiapi" fn efi_main(
    _image_handle: EfiHandle,
    system_table: *mut SystemTable,
) -> EfiStatus {
    let mut serial = unsafe { SerialPort::com1() };
    unsafe { serial.initialize() };
    match hello::render(&mut serial) {
        Ok(()) => {}
        Err(_) => return EFI_ABORTED,
    }

    let mut raw_memory_map_storage = [0u8; UEFI_MEMORY_MAP_STORAGE_BYTES];
    let mut region_storage = [MemoryRegion::EMPTY; MAX_MEMORY_REGIONS];
    let snapshot = match unsafe {
        capture_boot_memory_snapshot(
            system_table,
            &mut raw_memory_map_storage,
            &mut region_storage,
        )
    } {
        Ok(snapshot) => snapshot,
        Err(status) => return status,
    };

    let _allocator = match unsafe { BitmapAllocator::from_boot_snapshot(&snapshot) } {
        Ok(allocator) => allocator,
        Err(_) => return EFI_ABORTED,
    };

    halt::halt_forever()
}

#[cfg(target_os = "uefi")]
#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    halt::halt_forever()
}
