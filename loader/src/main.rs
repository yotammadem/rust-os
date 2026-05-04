#![cfg_attr(target_os = "uefi", no_std)]
#![cfg_attr(target_os = "uefi", no_main)]

#[cfg(target_os = "uefi")]
use core::panic::PanicInfo;
#[cfg(target_os = "uefi")]
use rust_os::LOADER_SERIAL_BANNER;
#[cfg(target_os = "uefi")]
use rust_os::arch::x86_64::{halt, serial::SerialPort};
#[cfg(target_os = "uefi")]
use rust_os::boot::multiboot::{EfiHandle, EfiStatus, SystemTable};

#[cfg(not(target_os = "uefi"))]
fn main() {}

#[cfg(target_os = "uefi")]
#[unsafe(no_mangle)]
pub extern "efiapi" fn efi_main(
    _image_handle: EfiHandle,
    _system_table: *mut SystemTable,
) -> EfiStatus {
    let mut serial = unsafe { SerialPort::com1() };
    serial.write_bytes(LOADER_SERIAL_BANNER);
    halt::halt_forever()
}

#[cfg(target_os = "uefi")]
#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    halt::halt_forever()
}
