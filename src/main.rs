#![cfg_attr(target_os = "uefi", no_std)]
#![cfg_attr(target_os = "uefi", no_main)]

mod arch;
mod boot;
mod kernel;

#[cfg(target_os = "uefi")]
use arch::x86_64::{framebuffer::FramebufferConsole, halt};
#[cfg(target_os = "uefi")]
use boot::multiboot::{EFI_ABORTED, EfiHandle, EfiStatus, SystemTable};
#[cfg(target_os = "uefi")]
use core::panic::PanicInfo;

#[cfg(not(target_os = "uefi"))]
fn main() {}

#[cfg(target_os = "uefi")]
#[unsafe(no_mangle)]
pub extern "efiapi" fn efi_main(
    _image_handle: EfiHandle,
    system_table: *mut SystemTable,
) -> EfiStatus {
    let mut console = match unsafe { FramebufferConsole::from_system_table(system_table) } {
        Some(console) => console,
        None => return EFI_ABORTED,
    };

    match kernel::hello::render(&mut console) {
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
