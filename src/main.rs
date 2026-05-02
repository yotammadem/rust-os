#![cfg_attr(target_os = "uefi", no_std)]
#![cfg_attr(target_os = "uefi", no_main)]

#[cfg(target_os = "uefi")]
use core::fmt::Write;
#[cfg(target_os = "uefi")]
use core::mem::MaybeUninit;
#[cfg(target_os = "uefi")]
use core::panic::PanicInfo;
#[cfg(target_os = "uefi")]
use rust_os::{
    DIRECT_MAP_SMOKE_PREFIX, PAGING_DIAGNOSTIC_PREFIX,
    arch::x86_64::{
        debugcon::DebugCon, halt,
        paging::{
            ActivationPlan, current_instruction_pointer, flush_runtime_mappings,
            higher_half_alias_addr,
        },
        serial::SerialPort,
    },
    boot::uefi::{EFI_ABORTED, EfiHandle, EfiStatus, SystemTable, capture_boot_memory_snapshot},
    kernel::hello,
    memory::{
        AddressSpace, AllocationResult, BitmapAllocator, EntryFlags, MAX_MEMORY_REGIONS,
        MemoryRegion, PAGE_SIZE, PageSpan, RegionKind, UEFI_MEMORY_MAP_STORAGE_BYTES,
        VirtualAddressLayout, align_down, unmap_range,
    },
};

#[cfg(target_os = "uefi")]
const ACTIVE_CODE_WINDOW_PAGE_COUNT: usize = 512;
#[cfg(target_os = "uefi")]
const HIGHER_HALF_STACK_PAGE_COUNT: usize = 32;
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
static mut REGION_STORAGE: [MemoryRegion; MAX_MEMORY_REGIONS] =
    [MemoryRegion::EMPTY; MAX_MEMORY_REGIONS];
#[cfg(target_os = "uefi")]
static mut BOOT_RUNTIME: MaybeUninit<BootRuntime> = MaybeUninit::uninit();

#[cfg(target_os = "uefi")]
struct BootRuntime {
    allocator: BitmapAllocator<'static>,
    kernel_space: AddressSpace,
    transition_alias_start: u64,
    transition_alias_page_count: usize,
}

#[cfg(not(target_os = "uefi"))]
fn main() {}

#[cfg(target_os = "uefi")]
#[unsafe(no_mangle)]
pub extern "efiapi" fn efi_main(
    _image_handle: EfiHandle,
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

    let current_ip = current_instruction_pointer();
    let code_window_start = align_down(
        current_ip,
        (ACTIVE_CODE_WINDOW_PAGE_COUNT * PAGE_SIZE) as u64,
    );

    let (kernel_space, template) = match AddressSpace::create_kernel_template(
        &mut allocator,
        code_window_start,
        ACTIVE_CODE_WINDOW_PAGE_COUNT,
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

    let higher_half_stack = match kernel_space.allocate_kernel_virtual(
        &mut allocator,
        HIGHER_HALF_STACK_PAGE_COUNT,
        EntryFlags::WRITABLE | EntryFlags::NO_EXECUTE,
    ) {
        Ok(allocation) => allocation,
        Err(err) => {
            let _ = print_boot_error_debugcon(&mut debugcon, paging_error_marker(err));
            let _ = print_boot_error(&mut serial, paging_error_marker(err));
            return EFI_ABORTED;
        }
    };

    let higher_half_entry_addr = match higher_half_alias_addr(
        post_activate_entry as *const () as usize as u64,
        template.transition_alias_start,
    ) {
            Some(addr) => addr,
            None => {
                let _ = print_boot_error_debugcon(&mut debugcon, "higher-half-entry");
                let _ = print_boot_error(&mut serial, "higher-half-entry");
                return EFI_ABORTED;
            }
        };
    // Match the stack shape a normal call would create before entering a Rust function.
    let higher_half_stack_top =
        higher_half_stack.virt_start_addr + (higher_half_stack.page_count * PAGE_SIZE) as u64 - 8;
    let runtime_context_addr = core::ptr::addr_of_mut!(BOOT_RUNTIME) as u64;
    let activation_plan = ActivationPlan::from_template(
        kernel_space.root_table_phys_addr,
        higher_half_entry_addr,
        higher_half_stack_top,
        runtime_context_addr,
        &template,
    );
    let _ = print_boot_marker_debugcon(&mut debugcon, "6 activation-plan");
    let _ = print_boot_marker(&mut serial, "6 activation-plan");

    if print_paging_diagnostics(&mut serial, &activation_plan, kernel_space.managed_phys_limit())
        .is_err()
    {
        let _ = print_boot_error_debugcon(&mut debugcon, "paging-diagnostics");
        let _ = print_boot_error(&mut serial, "paging-diagnostics");
        return EFI_ABORTED;
    }
    let _ = print_boot_marker_debugcon(&mut debugcon, "7 pre-activate");
    let _ = print_boot_marker(&mut serial, "7 pre-activate");

    unsafe {
        core::ptr::write(core::ptr::addr_of_mut!(BOOT_RUNTIME).cast::<BootRuntime>(), BootRuntime {
            allocator,
            kernel_space,
            transition_alias_start: activation_plan.transition_alias_start,
            transition_alias_page_count: activation_plan.transition_alias_page_count,
        });
        rust_os::arch::x86_64::paging::activate_and_continue(activation_plan);
    }
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
fn paging_error_marker(err: rust_os::memory::PagingError) -> &'static str {
    match err {
        rust_os::memory::PagingError::AddressOutOfRange => "higher-half-stack-address-range",
        rust_os::memory::PagingError::AllocatorStateCorrupted => "higher-half-stack-allocator-corrupt",
        rust_os::memory::PagingError::CapacityExceeded => "higher-half-stack-capacity",
        rust_os::memory::PagingError::EmptyRequest => "higher-half-stack-empty",
        rust_os::memory::PagingError::MappingConflict => "higher-half-stack-conflict",
        rust_os::memory::PagingError::OutOfMemory => "higher-half-stack-oom",
        rust_os::memory::PagingError::UnalignedAddress => "higher-half-stack-unaligned",
    }
}

#[cfg(target_os = "uefi")]
extern "C" fn post_activate_entry(runtime_context_addr: u64) -> ! {
    let mut runtime = unsafe { core::ptr::read(runtime_context_addr as *const BootRuntime) };
    let mut debugcon = DebugCon::new();
    let mut serial = unsafe { SerialPort::com1() };
    unsafe { serial.initialize() };

    let _ = print_boot_marker_debugcon(&mut debugcon, "8 higher-half-entry");
    let _ = print_boot_marker(&mut serial, "8 higher-half-entry");

    if unmap_range(
        &mut runtime.kernel_space,
        runtime.transition_alias_start,
        runtime.transition_alias_page_count,
    )
    .is_err()
    {
        let _ = print_boot_error_debugcon(&mut debugcon, "transition-alias-remove");
        let _ = print_boot_error(&mut serial, "transition-alias-remove");
        halt::halt_forever();
    }

    unsafe { flush_runtime_mappings(runtime.kernel_space.root_table_phys_addr) };
    let _ = print_boot_marker_debugcon(&mut debugcon, "9 transition-alias-removed");
    let _ = print_boot_marker(&mut serial, "9 transition-alias-removed");

    if smoke_test_direct_map_page(
        &mut serial,
        &mut runtime.allocator,
        runtime.kernel_space.managed_phys_limit(),
    )
    .is_err()
    {
        let _ = print_boot_error_debugcon(&mut debugcon, "direct-map-smoke");
        let _ = print_boot_error(&mut serial, "direct-map-smoke");
        halt::halt_forever();
    }
    let _ = print_boot_marker_debugcon(&mut debugcon, "10 post-direct-map-smoke");
    let _ = print_boot_marker(&mut serial, "10 post-direct-map-smoke");

    match hello::render(&mut serial) {
        Ok(()) => {
            let _ = print_boot_marker_debugcon(&mut debugcon, "11 hello-rendered");
            let _ = print_boot_marker(&mut serial, "11 hello-rendered");
        }
        Err(_) => {
            let _ = print_boot_error_debugcon(&mut debugcon, "hello-render");
            let _ = print_boot_error(&mut serial, "hello-render");
            halt::halt_forever();
        }
    }

    halt::halt_forever()
}

#[cfg(target_os = "uefi")]
fn smoke_test_direct_map_page(
    serial: &mut SerialPort,
    allocator: &mut BitmapAllocator<'_>,
    managed_phys_limit: u64,
) -> Result<(), ()> {
    const TEST_PATTERN: u64 = 0x5a17_c0de_d15e_a5e5;

    let AllocationResult::Allocated(span) = allocator.allocate_page() else {
        return Err(());
    };

    let direct_map_virt_addr =
        VirtualAddressLayout::phys_to_direct_map_virt(span.start_phys_addr, managed_phys_limit)
            .ok_or(())?;
    let page_ptr = direct_map_virt_addr as *mut u64;
    unsafe {
        core::ptr::write_bytes(page_ptr.cast::<u8>(), 0, PAGE_SIZE);
        core::ptr::write_volatile(page_ptr, TEST_PATTERN);
        if core::ptr::read_volatile(page_ptr) != TEST_PATTERN {
            return Err(());
        }
    }

    match allocator.free_pages(span) {
        AllocationResult::Released(_) => writeln!(
            serial,
            "{DIRECT_MAP_SMOKE_PREFIX} phys=0x{:016x} virt=0x{:016x}",
            span.start_phys_addr,
            direct_map_virt_addr
        )
        .map_err(|_| ()),
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
        "higher-half stack top: 0x{:016x}",
        activation_plan.higher_half_stack_top
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
fn panic(_info: &PanicInfo<'_>) -> ! {
    let mut debugcon = DebugCon::new();
    let mut serial = unsafe { SerialPort::com1() };
    unsafe { serial.initialize() };
    let _ = writeln!(debugcon, "{BOOT_PANIC_PREFIX}");
    let _ = writeln!(serial, "{BOOT_PANIC_PREFIX}");
    halt::halt_forever()
}
