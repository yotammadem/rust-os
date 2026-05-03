#![cfg_attr(target_os = "uefi", no_std)]
#![cfg_attr(target_os = "uefi", no_main)]

#[cfg(target_os = "uefi")]
use core::fmt::Write;
#[cfg(target_os = "uefi")]
use core::panic::PanicInfo;
#[cfg(target_os = "uefi")]
use rust_os::{
    DIRECT_MAP_SMOKE_PREFIX, PAGING_DIAGNOSTIC_PREFIX,
    arch::x86_64::{
        debugcon::DebugCon, halt, interrupts,
        paging::{ActivationPlan, current_instruction_pointer},
        serial::SerialPort,
    },
    boot::uefi::{
        EFI_ABORTED, EfiHandle, EfiStatus, SystemTable, apply_pe_relocations,
        capture_boot_memory_snapshot, loaded_image_metadata,
    },
    kernel::runtime,
    memory::{
        AllocationResult, BitmapAllocator, EntryFlags, KERNEL_VIRT_BASE, MAX_MEMORY_REGIONS,
        MemoryRegion, PAGE_SIZE, PageSpan, RegionKind, UEFI_MEMORY_MAP_STORAGE_BYTES,
        VirtualAddressLayout, align_down, align_up, set_runtime_page_access_active,
    },
};

#[cfg(target_os = "uefi")]
const ACTIVE_CODE_WINDOW_PAGE_COUNT: usize = 512;
#[cfg(target_os = "uefi")]
const ACTIVE_STACK_WINDOW_PAGE_COUNT: usize = 32;
#[cfg(target_os = "uefi")]
const BOOT_MARKER_PREFIX: &str = "boot-step:";
#[cfg(target_os = "uefi")]
const BOOT_ERROR_PREFIX: &str = "boot-error:";
#[cfg(target_os = "uefi")]
const BOOT_PANIC_PREFIX: &str = "boot-panic";
#[cfg(target_os = "uefi")]
static mut RAW_MEMORY_MAP_STORAGE: [u8; UEFI_MEMORY_MAP_STORAGE_BYTES] =
    [0; UEFI_MEMORY_MAP_STORAGE_BYTES];
#[cfg(target_os = "uefi")]
static mut REGION_STORAGE: [MemoryRegion; MAX_MEMORY_REGIONS] = [MemoryRegion::EMPTY; MAX_MEMORY_REGIONS];

#[cfg(not(target_os = "uefi"))]
fn main() {}

#[cfg(target_os = "uefi")]
#[unsafe(no_mangle)]
pub extern "efiapi" fn efi_main(
    image_handle: EfiHandle,
    system_table: *mut SystemTable,
) -> EfiStatus {
    let mut debugcon = DebugCon::new();
    let _ = print_boot_marker_debugcon(&mut debugcon, "0 efi-entry");

    let mut serial = unsafe { SerialPort::com1() };
    unsafe { serial.initialize() };
    let _ = print_boot_marker_debugcon(&mut debugcon, "1 serial-init");
    let _ = print_boot_marker(&mut serial, "1 serial-ready");

    let snapshot = match unsafe {
        capture_boot_memory_snapshot(
            system_table,
            core::slice::from_raw_parts_mut(
                core::ptr::addr_of_mut!(RAW_MEMORY_MAP_STORAGE).cast::<u8>(),
                UEFI_MEMORY_MAP_STORAGE_BYTES,
            ),
            core::slice::from_raw_parts_mut(
                core::ptr::addr_of_mut!(REGION_STORAGE).cast::<MemoryRegion>(),
                MAX_MEMORY_REGIONS,
            ),
        )
    } {
        Ok(snapshot) => {
            let _ = print_boot_marker_debugcon(&mut debugcon, "2 memory-map");
            let _ = print_boot_marker(&mut serial, "2 memory-map");
            snapshot
        }
        Err(status) => {
            let _ = print_boot_error_debugcon(&mut debugcon, "memory-map");
            let _ = print_boot_error(&mut serial, "memory-map");
            return status;
        }
    };

    let mut allocator = match unsafe { BitmapAllocator::from_boot_snapshot(&snapshot) } {
        Ok(allocator) => {
            let _ = print_boot_marker_debugcon(&mut debugcon, "3 allocator-ready");
            let _ = print_boot_marker(&mut serial, "3 allocator-ready");
            allocator
        }
        Err(_) => {
            let _ = print_boot_error_debugcon(&mut debugcon, "allocator-init");
            let _ = print_boot_error(&mut serial, "allocator-init");
            return EFI_ABORTED;
        }
    };

    if print_memory_diagnostics(&mut serial, &snapshot, &allocator).is_err() {
        let _ = print_boot_error_debugcon(&mut debugcon, "memory-diagnostics");
        let _ = print_boot_error(&mut serial, "memory-diagnostics");
        return EFI_ABORTED;
    }
    let _ = print_boot_marker_debugcon(&mut debugcon, "4 memory-diagnostics");
    let _ = print_boot_marker(&mut serial, "4 memory-diagnostics");

    let image_metadata = match unsafe { loaded_image_metadata(image_handle, system_table) } {
        Ok(metadata) => {
            let _ = writeln!(
                serial,
                "loaded image range: 0x{:016x}-0x{:016x}",
                metadata.loaded_base,
                metadata.loaded_base + metadata.loaded_size
            );
            let _ = print_boot_marker_debugcon(&mut debugcon, "4 loaded-image");
            metadata
        }
        Err(_) => {
            let _ = print_boot_error_debugcon(&mut debugcon, "loaded-image");
            let _ = print_boot_error(&mut serial, "loaded-image");
            return EFI_ABORTED;
        }
    };
    let image_base = image_metadata.loaded_base;
    let image_end = image_base + image_metadata.loaded_size;

    let transition_alias_start = align_down(image_base, PAGE_SIZE as u64);
    let transition_alias_end = align_up(image_base + image_metadata.size_of_image as u64, PAGE_SIZE as u64);
    let transition_alias_page_count =
        ((transition_alias_end - transition_alias_start) / PAGE_SIZE as u64) as usize;

    let runtime_image_copy = match copy_and_relocate_loaded_image(
        &mut allocator,
        &image_metadata,
        KERNEL_VIRT_BASE,
    ) {
        Ok(copy) => {
            let _ = print_boot_marker_debugcon(&mut debugcon, "5 runtime-image-copy");
            let _ = print_boot_marker(&mut serial, "5 runtime-image-copy");
            copy
        }
        Err(_) => {
            let _ = print_boot_error_debugcon(&mut debugcon, "runtime-image-copy");
            let _ = print_boot_error(&mut serial, "runtime-image-copy");
            return EFI_ABORTED;
        }
    };

    let (kernel_space, template) = match rust_os::memory::AddressSpace::create_kernel_template_for_runtime_image(
        &mut allocator,
        runtime_image_copy.backing_span.start_phys_addr,
        runtime_image_copy.backing_span.page_count,
        transition_alias_start,
        transition_alias_page_count,
    ) {
        Ok(result) => {
            let _ = print_boot_marker_debugcon(&mut debugcon, "5 kernel-template");
            let _ = print_boot_marker(&mut serial, "5 kernel-template");
            result
        }
        Err(_) => {
            let _ = print_boot_error_debugcon(&mut debugcon, "kernel-template");
            let _ = print_boot_error(&mut serial, "kernel-template");
            return EFI_ABORTED;
        }
    };
    let mut kernel_space = kernel_space;

    let higher_half_entry_addr = match higher_half_continuation_addr(&image_metadata) {
        Some(addr) => addr,
        None => {
            let _ = print_boot_error_debugcon(&mut debugcon, "higher-half-entry");
            let _ = print_boot_error(&mut serial, "higher-half-entry");
            return EFI_ABORTED;
        }
    };

    let continuation_stack = match kernel_space.allocate_kernel_virtual(
        &mut allocator,
        ACTIVE_STACK_WINDOW_PAGE_COUNT,
        EntryFlags::WRITABLE | EntryFlags::NO_EXECUTE,
    ) {
        Ok(allocation) => allocation,
        Err(_) => {
            let _ = print_boot_error_debugcon(&mut debugcon, "stack-allocate");
            let _ = print_boot_error(&mut serial, "stack-allocate");
            return EFI_ABORTED;
        }
    };
    let higher_half_stack_pointer =
        continuation_stack.virt_start_addr + (continuation_stack.page_count * PAGE_SIZE) as u64;

    let activation_plan = ActivationPlan::from_template(
        kernel_space.root_table_phys_addr,
        higher_half_entry_addr,
        higher_half_stack_pointer,
        &template,
    );
    let _ = print_boot_marker_debugcon(&mut debugcon, "6 activation-plan");
    let _ = print_boot_marker(&mut serial, "6 activation-plan");

    let managed_phys_limit = kernel_space.managed_phys_limit();
    if print_paging_diagnostics(&mut serial, &activation_plan, managed_phys_limit)
        .is_err()
    {
        let _ = print_boot_error_debugcon(&mut debugcon, "paging-diagnostics");
        let _ = print_boot_error(&mut serial, "paging-diagnostics");
        return EFI_ABORTED;
    }
    let _ = print_boot_marker_debugcon(&mut debugcon, "7 pre-activate");
    let _ = print_boot_marker(&mut serial, "7 pre-activate");
    let _ = writeln!(
        serial,
        "higher-half image window: 0x{:016x}-0x{:016x}",
        KERNEL_VIRT_BASE,
        KERNEL_VIRT_BASE + image_metadata.size_of_image as u64
    );
    let _ = writeln!(
        serial,
        "continuation low: 0x{:016x}",
        higher_half_continuation as *const () as usize as u64
    );
    let _ = writeln!(
        serial,
        "entry-mapped: {}",
        kernel_space.translate(higher_half_entry_addr).is_some()
    );
    let _ = writeln!(
        serial,
        "probe-mapped: {}",
        kernel_space.translate(KERNEL_VIRT_BASE + 0x1e442).is_some()
    );
    let _ = writeln!(
        serial,
        "image-in-window: {}",
        true
    );

    runtime::install(
        kernel_space,
        allocator,
        managed_phys_limit,
        activation_plan.transition_alias_start,
        activation_plan.transition_alias_page_count,
        runtime_image_copy,
        image_metadata,
    );

    unsafe { rust_os::arch::x86_64::paging::activate(activation_plan) };
}

#[cfg(target_os = "uefi")]
fn higher_half_continuation_addr(
    image_metadata: &rust_os::boot::uefi::PeImageMetadata,
) -> Option<u64> {
    let continuation_addr = higher_half_continuation as *const () as usize as u64;
    let image_end = image_metadata
        .loaded_base
        .checked_add(image_metadata.size_of_image as u64)?;

    if continuation_addr < image_metadata.loaded_base || continuation_addr >= image_end {
        return None;
    }

    KERNEL_VIRT_BASE.checked_add(continuation_addr - image_metadata.loaded_base)
}

#[cfg(target_os = "uefi")]
fn copy_and_relocate_loaded_image(
    allocator: &mut BitmapAllocator<'static>,
    image_metadata: &rust_os::boot::uefi::PeImageMetadata,
    runtime_virt_base: u64,
) -> Result<runtime::RuntimeImageCopy, ()> {
    let page_count = (align_up(image_metadata.size_of_image as u64, PAGE_SIZE as u64)
        / PAGE_SIZE as u64) as usize;
    let AllocationResult::Allocated(backing_span) = allocator.allocate_pages(page_count) else {
        return Err(());
    };

    unsafe {
        // Safety: early boot still runs with the firmware-provided low mapping
        // active, so both the loaded image bytes and the newly allocated pages
        // are reachable through their current low physical addresses.
        core::ptr::write_bytes(
            backing_span.start_phys_addr as *mut u8,
            0,
            page_count * PAGE_SIZE,
        );
        core::ptr::copy_nonoverlapping(
            image_metadata.loaded_base as *const u8,
            backing_span.start_phys_addr as *mut u8,
            image_metadata.size_of_image as usize,
        );

        let copied_image = core::slice::from_raw_parts_mut(
            backing_span.start_phys_addr as *mut u8,
            image_metadata.size_of_image as usize,
        );
        apply_pe_relocations(copied_image, image_metadata, runtime_virt_base).map_err(|_| ())?;
    }

    Ok(runtime::RuntimeImageCopy {
        backing_span,
        image_size: image_metadata.size_of_image as u64,
        runtime_virt_base,
    })
}

#[cfg(target_os = "uefi")]
unsafe extern "C" fn higher_half_continuation() -> ! {
    const TEST_PATTERN: u64 = 0x5a17_c0de_d15e_a5e5;

    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x402u16,
            in("al") b'A',
            options(nomem, nostack, preserves_flags)
        );
    }
    let mut debugcon = DebugCon::new();
    let mut serial = unsafe { SerialPort::com1() };
    unsafe { interrupts::install_minimal_fault_handlers() };
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x402u16,
            in("al") b'B',
            options(nomem, nostack, preserves_flags)
        );
    }
    let _ = debugcon.write_str("cont-a\r\n");
    let _ = serial.write_str("cont-a\r\n");
    let allocator = unsafe { runtime::allocator() };
    let managed_phys_limit = runtime::managed_phys_limit();
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x402u16,
            in("al") b'C',
            options(nomem, nostack, preserves_flags)
        );
    }
    let _ = debugcon.write_str("cont-b\r\n");
    let _ = serial.write_str("cont-b\r\n");

    let _ = print_boot_marker_debugcon(&mut debugcon, "8 post-activate");
    let _ = print_boot_marker(&mut serial, "8 post-activate");
    let _ = writeln!(serial, "higher-half rip: 0x{:016x}", current_instruction_pointer());
    let _ = print_boot_marker_debugcon(&mut debugcon, "8 fault-idt-ready");
    let _ = print_boot_marker(&mut serial, "8 fault-idt-ready");
    set_runtime_page_access_active(true);
    if unsafe { runtime::remove_transition_alias() }.is_err() {
        let _ = print_boot_error_debugcon(&mut debugcon, "transition-alias-remove");
        let _ = print_boot_error(&mut serial, "transition-alias-remove");
        halt::halt_forever();
    }
    let AllocationResult::Allocated(span) = allocator.allocate_page() else {
        let _ = print_boot_error_debugcon(&mut debugcon, "direct-map-smoke-allocate");
        let _ = print_boot_error(&mut serial, "direct-map-smoke-allocate");
        halt::halt_forever();
    };

    let direct_map_virt_addr =
        match VirtualAddressLayout::phys_to_direct_map_virt(span.start_phys_addr, managed_phys_limit) {
            Some(addr) => addr,
            None => {
                let _ = print_boot_error_debugcon(&mut debugcon, "direct-map-smoke-translate");
                let _ = print_boot_error(&mut serial, "direct-map-smoke-translate");
                halt::halt_forever();
            }
        };
    let page_ptr = direct_map_virt_addr as *mut u64;
    unsafe {
        core::ptr::write_bytes(page_ptr.cast::<u8>(), 0, PAGE_SIZE);
        core::ptr::write_volatile(page_ptr, TEST_PATTERN);
        if core::ptr::read_volatile(page_ptr) != TEST_PATTERN {
            let _ = print_boot_error_debugcon(&mut debugcon, "direct-map-smoke-readback");
            let _ = print_boot_error(&mut serial, "direct-map-smoke-readback");
            halt::halt_forever();
        }
    }

    match allocator.free_pages(span) {
        AllocationResult::Released(_) => {
            if serial.write_str(DIRECT_MAP_SMOKE_PREFIX).is_err()
                || serial.write_str(" ok\r\n").is_err()
            {
                let _ = print_boot_error_debugcon(&mut debugcon, "direct-map-smoke-print");
                halt::halt_forever();
            }
        }
        _ => {
            let _ = print_boot_error_debugcon(&mut debugcon, "direct-map-smoke-free");
            let _ = print_boot_error(&mut serial, "direct-map-smoke-free");
            halt::halt_forever();
        }
    }
    if debugcon.write_str("boot-step: 9 invalid-opcode-proof\r\n").is_err()
        || serial.write_str("boot-step: 9 invalid-opcode-proof\r\n").is_err()
    {
        halt::halt_forever();
    }
    unsafe { interrupts::trigger_invalid_opcode() }
}

#[cfg(target_os = "uefi")]
fn print_boot_marker(serial: &mut SerialPort, marker: &str) -> Result<(), ()> {
    writeln!(serial, "{BOOT_MARKER_PREFIX} {marker}").map_err(|_| ())
}

#[cfg(target_os = "uefi")]
fn print_boot_error(serial: &mut SerialPort, marker: &str) -> Result<(), ()> {
    writeln!(serial, "{BOOT_ERROR_PREFIX} {marker}").map_err(|_| ())
}

#[cfg(target_os = "uefi")]
fn print_boot_marker_debugcon(debugcon: &mut DebugCon, marker: &str) -> Result<(), ()> {
    writeln!(debugcon, "{BOOT_MARKER_PREFIX} {marker}").map_err(|_| ())
}

#[cfg(target_os = "uefi")]
fn print_boot_error_debugcon(debugcon: &mut DebugCon, marker: &str) -> Result<(), ()> {
    writeln!(debugcon, "{BOOT_ERROR_PREFIX} {marker}").map_err(|_| ())
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
    managed_phys_limit: u64,
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
    .map_err(|_| ())?;
    writeln!(
        serial,
        "direct-map window: 0x{:016x}-0x{:016x}",
        rust_os::memory::KERNEL_DIRECT_MAP_BASE,
        rust_os::memory::KERNEL_DIRECT_MAP_BASE + managed_phys_limit
    )
    .map_err(|_| ())
}

#[cfg(target_os = "uefi")]
#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    let mut debugcon = DebugCon::new();
    let mut serial = unsafe { SerialPort::com1() };
    unsafe { serial.initialize() };
    let _ = writeln!(debugcon, "{BOOT_PANIC_PREFIX}");
    let _ = writeln!(serial, "{BOOT_PANIC_PREFIX}");
    if let Some(location) = info.location() {
        let _ = writeln!(
            debugcon,
            "panic-location: {}:{}",
            location.file(),
            location.line()
        );
        let _ = writeln!(
            serial,
            "panic-location: {}:{}",
            location.file(),
            location.line()
        );
    }
    let _ = writeln!(debugcon, "panic-message: {info}");
    let _ = writeln!(serial, "panic-message: {info}");
    halt::halt_forever()
}
