#![cfg_attr(target_os = "uefi", no_std)]
#![cfg_attr(target_os = "uefi", no_main)]

#[cfg(target_os = "uefi")]
mod bootinfo;
#[cfg(target_os = "uefi")]
mod memory;

#[cfg(target_os = "uefi")]
use self::memory::{EarlyLayout, PhysicalRange};
#[cfg(target_os = "uefi")]
use core::panic::PanicInfo;
#[cfg(target_os = "uefi")]
use rust_os::LOADER_SERIAL_BANNER;
#[cfg(target_os = "uefi")]
use rust_os::arch::x86_64::{halt, serial::SerialPort};
#[cfg(target_os = "uefi")]
use rust_os::boot::handoff::BootInfo;
#[cfg(target_os = "uefi")]
use rust_os::boot::multiboot::{EfiHandle, EfiStatus, SystemTable};

#[cfg(not(target_os = "uefi"))]
fn main() {}

#[cfg(target_os = "uefi")]
#[unsafe(no_mangle)]
pub extern "efiapi" fn efi_main(
    image_handle: EfiHandle,
    system_table: *mut SystemTable,
) -> EfiStatus {
    let mut serial = unsafe { SerialPort::com1() };
    serial.write_bytes(LOADER_SERIAL_BANNER);
    serial.write_bytes(b"collecting boot info...\r\n");

    let boot_info = match bootinfo::collect(image_handle, system_table) {
        Ok(boot_info) => boot_info,
        Err(status) => {
            serial.write_bytes(b"boot info collection failed: ");
            write_hex_usize(&mut serial, status);
            serial.write_bytes(b"\r\n");
            return status;
        }
    };

    print_boot_info(&mut serial, &boot_info);
    halt::halt_forever()
}

#[cfg(target_os = "uefi")]
fn print_boot_info(serial: &mut SerialPort, boot_info: &BootInfo) {
    serial.write_bytes(b"loader image start: ");
    write_hex_u64(serial, boot_info.loader_image.start);
    serial.write_bytes(b"\r\n");

    serial.write_bytes(b"loader image end:   ");
    write_hex_u64(serial, boot_info.loader_image.end);
    serial.write_bytes(b"\r\n");

    serial.write_bytes(b"memory map buffer:  ");
    write_hex_u64(serial, boot_info.memory_map.map as usize as u64);
    serial.write_bytes(b"\r\n");

    serial.write_bytes(b"memory map size:    ");
    write_hex_usize(serial, boot_info.memory_map.map_size);
    serial.write_bytes(b"\r\n");

    serial.write_bytes(b"memory map key:     ");
    write_hex_usize(serial, boot_info.memory_map.map_key);
    serial.write_bytes(b"\r\n");

    serial.write_bytes(b"descriptor size:    ");
    write_hex_usize(serial, boot_info.memory_map.descriptor_size);
    serial.write_bytes(b"\r\n");

    serial.write_bytes(b"descriptor version: ");
    write_hex_u32(serial, boot_info.memory_map.descriptor_version);
    serial.write_bytes(b"\r\n");

    serial.write_bytes(b"descriptor count:   ");
    let count = boot_info.memory_map.descriptor_count();
    write_hex_usize(serial, count);
    serial.write_bytes(b"\r\n");

    let layout = EarlyLayout::from_boot_info(boot_info);

    print_candidate(serial, b"early allocation region", layout.region);
    print_candidate(serial, b"kernel usable region", layout.kernel_usable_region);
    print_candidate(serial, b"boot-info region", layout.boot_info_region);
    print_candidate(serial, b"page-table region", layout.page_table_region);
}

#[cfg(target_os = "uefi")]
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

#[cfg(target_os = "uefi")]
fn write_hex_u32(serial: &mut SerialPort, value: u32) {
    write_hex_u64(serial, value as u64);
}

#[cfg(target_os = "uefi")]
fn write_hex_usize(serial: &mut SerialPort, value: usize) {
    write_hex_u64(serial, value as u64);
}

#[cfg(target_os = "uefi")]
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

#[cfg(target_os = "uefi")]
fn print_candidate(serial: &mut SerialPort, title: &[u8], range: PhysicalRange) {
    serial.write_bytes(title);
    serial.write_bytes(b": ");
    print_range(serial, range);
    serial.write_bytes(b"\r\n");
}

#[cfg(target_os = "uefi")]
fn print_range(serial: &mut SerialPort, range: PhysicalRange) {
    write_hex_u64(serial, range.start);
    serial.write_bytes(b"..");
    write_hex_u64(serial, range.end);
    serial.write_bytes(b" bytes=");
    write_decimal_u64(serial, range.size_bytes());
    serial.write_bytes(b" (");
    write_decimal_u64(serial, range.size_bytes() / (1024 * 1024));
    serial.write_bytes(b" MiB)");
}

#[cfg(target_os = "uefi")]
fn hex_digit(value: u8) -> u8 {
    match value {
        0..=9 => b'0' + value,
        _ => b'a' + (value - 10),
    }
}

#[cfg(target_os = "uefi")]
#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    halt::halt_forever()
}
