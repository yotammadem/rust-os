use core::cell::UnsafeCell;
use core::mem::size_of;

use crate::arch::x86_64::{debugcon::DebugCon, serial::SerialPort};
use crate::arch::x86_64::paging::invalidate_tlb_page;
use crate::boot::uefi::PeImageMetadata;
use crate::memory::{
    AddressSpace, AllocationResult, BitmapAllocator, KERNEL_VIRT_BASE, PAGE_SIZE, PageSpan,
    PagingError, PhysAddr, VirtualAddressLayout, unmap_range,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeImageCopy {
    pub backing_span: PageSpan,
    pub image_size: u64,
    pub runtime_virt_base: u64,
}

struct RuntimeState {
    kernel_space: AddressSpace,
    allocator: BitmapAllocator<'static>,
    managed_phys_limit: PhysAddr,
    transition_alias_start: u64,
    transition_alias_page_count: usize,
    runtime_image_copy: RuntimeImageCopy,
    runtime_image_metadata: PeImageMetadata,
}

struct RuntimeStatePointerCell(UnsafeCell<*mut RuntimeState>);

unsafe impl Sync for RuntimeStatePointerCell {}

static RUNTIME_STATE_PTR: RuntimeStatePointerCell =
    RuntimeStatePointerCell(UnsafeCell::new(core::ptr::null_mut()));

pub fn install(
    kernel_space: AddressSpace,
    mut allocator: BitmapAllocator<'static>,
    managed_phys_limit: PhysAddr,
    transition_alias_start: u64,
    transition_alias_page_count: usize,
    runtime_image_copy: RuntimeImageCopy,
    runtime_image_metadata: PeImageMetadata,
) {
    assert!(
        unsafe { allocator.rebase_bootstrap_storage(transition_alias_start, KERNEL_VIRT_BASE) },
        "runtime allocator bitmap storage must be reachable through the higher-half kernel mapping"
    );
    let state_storage_page_count =
        (size_of::<RuntimeState>().div_ceil(PAGE_SIZE)).max(1);
    let AllocationResult::Allocated(storage_span) = allocator.allocate_pages(state_storage_page_count) else {
        panic!("runtime state storage allocation must succeed before continuation");
    };
    let state_storage_virt_addr =
        VirtualAddressLayout::phys_to_direct_map_virt(storage_span.start_phys_addr, managed_phys_limit)
            .expect("runtime state storage must be reachable through the direct map");
    let state_ptr = state_storage_virt_addr as *mut RuntimeState;
    unsafe {
        // Safety: early boot is single-threaded and installs runtime state once
        // before handing control to the copied higher-half continuation. The
        // runtime state itself lives in allocator-owned memory that both the
        // original image and the copied image can reference through the direct
        // map.
        state_ptr.write(RuntimeState {
            kernel_space,
            allocator,
            managed_phys_limit,
            transition_alias_start,
            transition_alias_page_count,
            runtime_image_copy,
            runtime_image_metadata,
        });
        publish_runtime_state_ptr(state_ptr, &runtime_image_copy, &runtime_image_metadata);
    }
}

pub unsafe fn allocator() -> &'static mut BitmapAllocator<'static> {
    unsafe {
        &mut runtime_state_mut().allocator
    }
}

pub fn managed_phys_limit() -> PhysAddr {
    unsafe {
        runtime_state_ref().managed_phys_limit
    }
}

pub fn transition_alias_start() -> u64 {
    unsafe {
        runtime_state_ref().transition_alias_start
    }
}

pub fn image_addr_to_runtime_virt(addr: u64) -> Option<u64> {
    unsafe {
        let state = runtime_state_ref();
        let image_end = state
            .runtime_image_metadata
            .loaded_base
            .checked_add(state.runtime_image_metadata.size_of_image as u64)?;
        let runtime_image_end = state
            .runtime_image_copy
            .runtime_virt_base
            .checked_add(state.runtime_image_copy.image_size)?;

        if addr >= state.runtime_image_copy.runtime_virt_base && addr < runtime_image_end {
            return Some(addr);
        }

        if addr < state.runtime_image_metadata.loaded_base || addr >= image_end {
            return None;
        }

        state
            .runtime_image_copy
            .runtime_virt_base
            .checked_add(addr - state.runtime_image_metadata.loaded_base)
    }
}

pub fn runtime_image_copy() -> RuntimeImageCopy {
    unsafe {
        runtime_state_ref().runtime_image_copy
    }
}

pub fn runtime_image_metadata() -> PeImageMetadata {
    unsafe {
        runtime_state_ref().runtime_image_metadata
    }
}

pub unsafe fn remove_transition_alias() -> Result<(), PagingError> {
    unsafe {
        let state = runtime_state_mut();
        unmap_range(
            &mut state.kernel_space,
            state.transition_alias_start,
            state.transition_alias_page_count,
        )?;

        for page in 0..state.transition_alias_page_count {
            let virt_addr = state.transition_alias_start + (page * PAGE_SIZE) as u64;
            invalidate_tlb_page(virt_addr);
        }

        Ok(())
    }
}

unsafe fn runtime_state_ref() -> &'static RuntimeState {
    let state_ptr = unsafe { *RUNTIME_STATE_PTR.0.get() };
    if state_ptr.is_null() {
        let mut debugcon = DebugCon::new();
        let mut serial = unsafe { SerialPort::com1() };
        let _ = core::fmt::Write::write_str(&mut debugcon, "runtime-state-null\r\n");
        let _ = core::fmt::Write::write_str(&mut serial, "runtime-state-null\r\n");
        panic!("runtime state pointer installed before continuation");
    }
    unsafe { &*state_ptr }
}

unsafe fn runtime_state_mut() -> &'static mut RuntimeState {
    let state_ptr = unsafe { *RUNTIME_STATE_PTR.0.get() };
    if state_ptr.is_null() {
        let mut debugcon = DebugCon::new();
        let mut serial = unsafe { SerialPort::com1() };
        let _ = core::fmt::Write::write_str(&mut debugcon, "runtime-state-null\r\n");
        let _ = core::fmt::Write::write_str(&mut serial, "runtime-state-null\r\n");
        panic!("runtime state pointer installed before continuation");
    }
    unsafe { &mut *state_ptr }
}

unsafe fn publish_runtime_state_ptr(
    state_ptr: *mut RuntimeState,
    runtime_image_copy: &RuntimeImageCopy,
    runtime_image_metadata: &PeImageMetadata,
) {
    unsafe {
        // Safety: `RUNTIME_STATE_PTR` is private to this module and its layout
        // is a single pointer-sized `UnsafeCell`. We write the same direct-map
        // state pointer into both the currently executing image and the copied
        // runtime image so either instance resolves to the shared state block.
        *RUNTIME_STATE_PTR.0.get() = state_ptr;
    }

    let current_ptr_addr = core::ptr::addr_of!(RUNTIME_STATE_PTR) as usize as u64;
    let offset = current_ptr_addr
        .checked_sub(runtime_image_metadata.loaded_base)
        .expect("runtime state pointer must live inside the loaded image");
    let runtime_ptr_addr = runtime_image_copy
        .runtime_virt_base
        .checked_add(offset)
        .expect("runtime state pointer offset must fit in the runtime image");

    unsafe {
        *(runtime_ptr_addr as *mut *mut RuntimeState) = state_ptr;
    }
}
