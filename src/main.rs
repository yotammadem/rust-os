#![cfg_attr(target_os = "uefi", no_std)]
#![cfg_attr(target_os = "uefi", no_main)]

#[cfg(target_os = "uefi")]
mod arch;
#[cfg(target_os = "uefi")]
mod boot;
#[cfg(target_os = "uefi")]
mod kernel;

#[cfg(target_os = "uefi")]
use arch::x86_64::{halt, serial::SerialPort};
#[cfg(target_os = "uefi")]
use boot::multiboot::{EfiHandle, EfiStatus, SystemTable};
#[cfg(target_os = "uefi")]
use core::panic::PanicInfo;

#[cfg(not(target_os = "uefi"))]
fn main() {}

#[cfg(target_os = "uefi")]
#[unsafe(no_mangle)]
pub extern "efiapi" fn efi_main(
    _image_handle: EfiHandle,
    _system_table: *mut SystemTable,
) -> EfiStatus {
    let mut serial = unsafe { SerialPort::com1() };

    match kernel::hello::render(&mut serial) {
        Ok(()) => {}
        Err(status) => return status,
    }

    halt::halt_forever()
}

#[cfg(target_os = "uefi")]
#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    halt::halt_forever()
}
