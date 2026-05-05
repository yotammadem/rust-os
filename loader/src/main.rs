#![cfg_attr(target_os = "uefi", no_std)]
#![cfg_attr(target_os = "uefi", no_main)]

#[cfg(target_os = "uefi")]
mod bootinfo;
#[cfg(target_os = "uefi")]
mod elf;
#[cfg(target_os = "uefi")]
mod kernel_image;
#[cfg(target_os = "uefi")]
mod memory;
#[cfg(target_os = "uefi")]
mod paging;

#[cfg(target_os = "uefi")]
use self::kernel_image::{LoadError, LoadedKernelImage, LoadedSegment};
#[cfg(target_os = "uefi")]
use self::memory::{EarlyLayout, PhysicalRange};
#[cfg(target_os = "uefi")]
use self::paging::{BuildError, BuiltPageTables};
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

    print_boot_info(&mut serial, image_handle, system_table, &boot_info);
    halt::halt_forever()
}

#[cfg(target_os = "uefi")]
fn print_boot_info(
    serial: &mut SerialPort,
    image_handle: EfiHandle,
    system_table: *mut SystemTable,
    boot_info: &BootInfo,
) {
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
    print_candidate(serial, b"kernel stack region", layout.kernel_stack_region);
    print_candidate(serial, b"boot-info region", layout.boot_info_region);
    print_candidate(serial, b"page-table region", layout.page_table_region);

    serial.write_bytes(b"loading kernel image...\r\n");
    let loaded_kernel = match kernel_image::load(
        image_handle,
        system_table,
        layout.kernel_usable_region.start,
        layout.kernel_usable_region.size_bytes() as usize,
    ) {
        Ok(loaded_kernel) => loaded_kernel,
        Err(error) => {
            print_load_error(serial, error);
            serial.write_bytes(b"\r\n");
            return;
        }
    };

    print_loaded_kernel(serial, loaded_kernel);
    print_kernel_segments(serial, loaded_kernel);

    serial.write_bytes(b"building page tables...\r\n");
    let page_tables = match paging::build(boot_info, layout, loaded_kernel) {
        Ok(page_tables) => page_tables,
        Err(error) => {
            print_paging_error(serial, error);
            serial.write_bytes(b"\r\n");
            return;
        }
    };
    print_page_tables(serial, page_tables, loaded_kernel);
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
fn print_loaded_kernel(serial: &mut SerialPort, loaded_kernel: LoadedKernelImage) {
    serial.write_bytes(b"kernel file size:    ");
    write_decimal_u64(serial, loaded_kernel.file_size as u64);
    serial.write_bytes(b" bytes\r\n");

    serial.write_bytes(b"kernel entry point:  ");
    write_hex_u64(serial, loaded_kernel.entry_point);
    serial.write_bytes(b"\r\n");

    serial.write_bytes(b"kernel loaded start: ");
    write_hex_u64(serial, loaded_kernel.physical_start);
    serial.write_bytes(b"\r\n");

    serial.write_bytes(b"kernel loaded end:   ");
    write_hex_u64(serial, loaded_kernel.physical_end);
    serial.write_bytes(b"\r\n");
}

#[cfg(target_os = "uefi")]
fn print_load_error(serial: &mut SerialPort, error: LoadError) {
    serial.write_bytes(b"kernel load failed at ");
    serial.write_bytes(error.stage);
    serial.write_bytes(b": ");
    write_hex_usize(serial, error.status);
}

#[cfg(target_os = "uefi")]
fn print_kernel_segments(serial: &mut SerialPort, loaded_kernel: LoadedKernelImage) {
    serial.write_bytes(b"loadable segments:\r\n");
    for (index, segment) in loaded_kernel.segments[..loaded_kernel.segment_count]
        .iter()
        .copied()
        .enumerate()
    {
        if segment.memory_size == 0 {
            continue;
        }
        print_segment(serial, index, segment);
    }
}

#[cfg(target_os = "uefi")]
fn print_segment(serial: &mut SerialPort, index: usize, segment: LoadedSegment) {
    serial.write_bytes(b"  [");
    write_decimal_u64(serial, index as u64);
    serial.write_bytes(b"] off=");
    write_hex_u64(serial, segment.file_offset);
    serial.write_bytes(b" paddr=");
    write_hex_u64(serial, segment.physical_start);
    serial.write_bytes(b"..");
    write_hex_u64(serial, segment.physical_end);
    serial.write_bytes(b" vaddr=");
    write_hex_u64(serial, segment.virtual_address);
    serial.write_bytes(b" filesz=");
    write_hex_u64(serial, segment.file_size);
    serial.write_bytes(b" memsz=");
    write_hex_u64(serial, segment.memory_size);
    serial.write_bytes(b" flags=");
    write_hex_u32(serial, segment.flags);
    serial.write_bytes(b" align=");
    write_hex_u64(serial, segment.align);
    serial.write_bytes(b"\r\n");
}

#[cfg(target_os = "uefi")]
fn print_page_tables(
    serial: &mut SerialPort,
    page_tables: BuiltPageTables,
    loaded_kernel: LoadedKernelImage,
) {
    serial.write_bytes(b"page-table root:     ");
    write_hex_u64(serial, page_tables.pml4_physical_start);
    serial.write_bytes(b"\r\n");

    serial.write_bytes(b"page-table pages:    ");
    write_decimal_u64(serial, page_tables.pages_used as u64);
    serial.write_bytes(b"\r\n");

    print_candidate(serial, b"loader stack window", page_tables.stack_window);
    print_candidate(serial, b"memory-map window", page_tables.memory_map_window);
    serial.write_bytes(b"kernel stack mapping: ");
    write_hex_u64(serial, page_tables.kernel_stack_physical.start);
    serial.write_bytes(b"..");
    write_hex_u64(serial, page_tables.kernel_stack_physical.end);
    serial.write_bytes(b" -> ");
    write_hex_u64(serial, page_tables.kernel_stack_virtual.start);
    serial.write_bytes(b"..");
    write_hex_u64(serial, page_tables.kernel_stack_virtual.end);
    serial.write_bytes(b"\r\n");

    serial.write_bytes(b"kernel higher-half mappings:\r\n");
    for (index, segment) in loaded_kernel.segments[..loaded_kernel.segment_count]
        .iter()
        .copied()
        .enumerate()
    {
        if segment.memory_size == 0 {
            continue;
        }

        serial.write_bytes(b"  [");
        write_decimal_u64(serial, index as u64);
        serial.write_bytes(b"] ");
        write_hex_u64(serial, segment.physical_start);
        serial.write_bytes(b"..");
        write_hex_u64(serial, segment.physical_end);
        serial.write_bytes(b" -> ");
        write_hex_u64(serial, segment.virtual_address);
        serial.write_bytes(b"..");
        write_hex_u64(
            serial,
            segment.virtual_address.saturating_add(segment.memory_size),
        );
        serial.write_bytes(b"\r\n");
    }
}

#[cfg(target_os = "uefi")]
fn print_paging_error(serial: &mut SerialPort, error: BuildError) {
    serial.write_bytes(b"page-table build failed at ");
    serial.write_bytes(error.stage);
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
