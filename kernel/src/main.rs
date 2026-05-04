#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

#[cfg(target_os = "none")]
use core::panic::PanicInfo;
#[cfg(target_os = "none")]
use rust_os::KERNEL_SERIAL_BANNER;
#[cfg(target_os = "none")]
use rust_os::arch::x86_64::{halt, serial::SerialPort};

#[cfg(not(target_os = "none"))]
fn main() {}

#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let mut serial = unsafe { SerialPort::com1() };
    serial.write_bytes(KERNEL_SERIAL_BANNER);
    halt::halt_forever()
}

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    halt::halt_forever()
}
