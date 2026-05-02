#![cfg_attr(target_os = "uefi", no_std)]
#![cfg_attr(target_os = "uefi", no_main)]

#[cfg(target_os = "uefi")]
use core::fmt::Write;
#[cfg(target_os = "uefi")]
use core::panic::PanicInfo;
#[cfg(target_os = "uefi")]
use rust_os::{
    KERNEL_BOOT_PHYS_BASE, PAGING_DIAGNOSTIC_PREFIX,
    arch::x86_64::{halt, paging::ActivationPlan, serial::SerialPort},
    boot::uefi::{EFI_ABORTED, EfiHandle, EfiStatus, SystemTable, capture_boot_memory_snapshot},
    kernel::hello,
    memory::{
        AllocationResult, BitmapAllocator, MAX_MEMORY_REGIONS, MemoryRegion, PAGE_SIZE, PageSpan,
        RegionKind, UEFI_MEMORY_MAP_STORAGE_BYTES,
    },
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

    let mut allocator = match unsafe { BitmapAllocator::from_boot_snapshot(&snapshot) } {
        Ok(allocator) => allocator,
        Err(_) => return EFI_ABORTED,
    };

    if smoke_test_allocated_page(&mut allocator).is_err() {
        return EFI_ABORTED;
    }

    if print_memory_diagnostics(&mut serial, &snapshot, &allocator).is_err() {
        return EFI_ABORTED;
    }

    let activation_plan = ActivationPlan {
        root_table_phys_addr: 0,
        higher_half_entry_addr: rust_os::memory::paging::KERNEL_VIRT_BASE,
        transition_alias_start: KERNEL_BOOT_PHYS_BASE,
        transition_alias_page_count: 4,
    };

    if print_paging_diagnostics(&mut serial, &activation_plan).is_err() {
        return EFI_ABORTED;
    }

    match hello::render(&mut serial) {
        Ok(()) => {}
        Err(_) => return EFI_ABORTED,
    }

    halt::halt_forever()
}

#[cfg(target_os = "uefi")]
fn smoke_test_allocated_page(allocator: &mut BitmapAllocator<'_>) -> Result<(), ()> {
    const TEST_PATTERN: u64 = 0x5a17_c0de_d15e_a5e5;

    let AllocationResult::Allocated(span) = allocator.allocate_page() else {
        return Err(());
    };

    let page_ptr = span.start_phys_addr as *mut u64;
    unsafe {
        core::ptr::write_volatile(page_ptr, TEST_PATTERN);
        if core::ptr::read_volatile(page_ptr) != TEST_PATTERN {
            return Err(());
        }
    }

    match allocator.free_pages(span) {
        AllocationResult::Released(_) => Ok(()),
        _ => Err(()),
    }
}

#[cfg(target_os = "uefi")]
fn print_memory_diagnostics(
    serial: &mut SerialPort,
    snapshot: &rust_os::memory::BootMemoryMapSnapshot<'_>,
    allocator: &BitmapAllocator<'_>,
) -> Result<(), ()> {
    let available_bytes = allocator.free_page_count() * PAGE_SIZE;
    writeln!(
        serial,
        "available memory: {} KiB ({} pages)",
        available_bytes / 1024,
        allocator.free_page_count()
    )
    .map_err(|_| ())?;
    writeln!(serial, "allocatable ranges:").map_err(|_| ())?;

    for region in snapshot.regions {
        if region.kind != RegionKind::Usable {
            continue;
        }

        print_allocatable_segments(serial, *region, allocator.metadata_span())?;
    }

    Ok(())
}

#[cfg(target_os = "uefi")]
fn print_allocatable_segments(
    serial: &mut SerialPort,
    region: MemoryRegion,
    metadata_span: PageSpan,
) -> Result<(), ()> {
    let region_start = region.start_page_index;
    let region_end = region.end_page_index();
    let metadata_start = metadata_span.start_page_index;
    let metadata_end = metadata_span.start_page_index + metadata_span.page_count;

    if metadata_end <= region_start || metadata_start >= region_end {
        return print_page_range(serial, region_start, region_end);
    }

    if region_start < metadata_start {
        print_page_range(serial, region_start, metadata_start)?;
    }

    if metadata_end < region_end {
        print_page_range(serial, metadata_end, region_end)?;
    }

    Ok(())
}

#[cfg(target_os = "uefi")]
fn print_page_range(serial: &mut SerialPort, start_page: usize, end_page: usize) -> Result<(), ()> {
    if start_page >= end_page {
        return Ok(());
    }

    let start_phys = start_page * PAGE_SIZE;
    let end_phys = end_page * PAGE_SIZE;
    writeln!(
        serial,
        "  0x{start_phys:016x}-0x{end_phys:016x} ({} KiB)",
        (end_phys - start_phys) / 1024
    )
    .map_err(|_| ())
}

#[cfg(target_os = "uefi")]
fn print_paging_diagnostics(
    serial: &mut SerialPort,
    activation_plan: &ActivationPlan,
) -> Result<(), ()> {
    writeln!(
        serial,
        "{PAGING_DIAGNOSTIC_PREFIX} 0x{:016x}",
        activation_plan.root_table_phys_addr
    )
    .map_err(|_| ())?;
    writeln!(
        serial,
        "higher-half entry: 0x{:016x}",
        activation_plan.higher_half_entry_addr
    )
    .map_err(|_| ())?;
    writeln!(
        serial,
        "transition alias: 0x{:016x} ({} pages)",
        activation_plan.transition_alias_start,
        activation_plan.transition_alias_page_count
    )
    .map_err(|_| ())
}

#[cfg(target_os = "uefi")]
#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    halt::halt_forever()
}
