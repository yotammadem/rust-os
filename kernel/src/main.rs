#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

#[cfg(target_os = "none")]
use core::panic::PanicInfo;
#[cfg(target_os = "none")]
use rust_os::KERNEL_SERIAL_BANNER;
#[cfg(target_os = "none")]
use rust_os::arch::x86_64::{halt, serial::SerialPort};
#[cfg(target_os = "none")]
use rust_os::boot::handoff::BootInfo;

#[cfg(not(target_os = "none"))]
fn main() {}

#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
pub extern "C" fn _start(_boot_info: *const BootInfo) -> ! {
    let mut serial = unsafe { SerialPort::com1() };
    serial.write_bytes(KERNEL_SERIAL_BANNER);
    serial.write_bytes(b"kernel boot_info:    ");
    write_hex_u64(&mut serial, _boot_info as usize as u64);
    serial.write_bytes(b"\r\n");

    let boot_info = unsafe { &*_boot_info };
    serial.write_bytes(b"kernel pml4:         ");
    write_hex_u64(&mut serial, boot_info.paging.pml4_physical_start);
    serial.write_bytes(b"\r\n");

    serial.write_bytes(b"kernel stack range:  ");
    write_hex_u64(&mut serial, boot_info.paging.kernel_stack_virtual.start);
    serial.write_bytes(b"..");
    write_hex_u64(&mut serial, boot_info.paging.kernel_stack_virtual.end);
    serial.write_bytes(b"\r\n");

    serial.write_bytes(b"kernel usable ranges:");
    write_decimal_u64(&mut serial, boot_info.usable_range_count as u64);
    serial.write_bytes(b"\r\n");
    halt::halt_forever()
}

#[cfg(target_os = "none")]
fn write_hex_u64(serial: &mut SerialPort, value: u64) {
    serial.write_bytes(b"0x");
    let mut shift = 60;
    loop {
        serial.write_bytes(&[hex_digit(((value >> shift) & 0xf) as u8)]);
        if shift == 0 {
            break;
        }
        shift -= 4;
    }
}

#[cfg(target_os = "none")]
fn write_decimal_u64(serial: &mut SerialPort, mut value: u64) {
    let mut buf = [0u8; 20];
    let mut idx = buf.len();

    if value == 0 {
        serial.write_bytes(b"0");
        return;
    }

    while value > 0 {
        idx -= 1;
        buf[idx] = b'0' + (value % 10) as u8;
        value /= 10;
    }

    serial.write_bytes(&buf[idx..]);
}

#[cfg(target_os = "none")]
fn hex_digit(value: u8) -> u8 {
    match value {
        0..=9 => b'0' + value,
        _ => b'a' + (value - 10),
    }
}

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    halt::halt_forever()
}
