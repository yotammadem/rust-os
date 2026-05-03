use core::cell::UnsafeCell;

use crate::arch::x86_64::paging::invalidate_tlb_page;
use crate::memory::{AddressSpace, BitmapAllocator, KERNEL_VIRT_BASE, PagingError, PhysAddr, unmap_range};

struct RuntimeState {
    kernel_space: AddressSpace,
    allocator: BitmapAllocator<'static>,
    managed_phys_limit: PhysAddr,
    transition_alias_start: u64,
    transition_alias_page_count: usize,
}

struct RuntimeStateCell(UnsafeCell<Option<RuntimeState>>);

unsafe impl Sync for RuntimeStateCell {}

static RUNTIME_STATE: RuntimeStateCell = RuntimeStateCell(UnsafeCell::new(None));

pub fn install(
    kernel_space: AddressSpace,
    mut allocator: BitmapAllocator<'static>,
    managed_phys_limit: PhysAddr,
    transition_alias_start: u64,
    transition_alias_page_count: usize,
) {
    assert!(
        unsafe { allocator.rebase_bootstrap_storage(transition_alias_start, KERNEL_VIRT_BASE) },
        "runtime allocator bitmap storage must be reachable through the higher-half kernel mapping"
    );
    unsafe {
        // Safety: early boot is single-threaded and installs runtime state once
        // before handing control to the higher-half continuation.
        *RUNTIME_STATE.0.get() = Some(RuntimeState {
            kernel_space,
            allocator,
            managed_phys_limit,
            transition_alias_start,
            transition_alias_page_count,
        });
    }
}

pub unsafe fn allocator() -> &'static mut BitmapAllocator<'static> {
    unsafe {
        // Safety: the runtime state is installed before continuation and remains
        // owned by the kernel for the rest of early boot.
        &mut (*RUNTIME_STATE.0.get())
            .as_mut()
            .expect("runtime allocator installed")
            .allocator
    }
}

pub fn managed_phys_limit() -> PhysAddr {
    unsafe {
        // Safety: the runtime state is installed before continuation and then
        // read-only for this scalar field.
        (*RUNTIME_STATE.0.get())
            .as_ref()
            .expect("runtime state installed")
            .managed_phys_limit
    }
}

pub fn transition_alias_start() -> u64 {
    unsafe {
        (*RUNTIME_STATE.0.get())
            .as_ref()
            .expect("runtime state installed")
            .transition_alias_start
    }
}

pub fn image_addr_to_runtime_virt(addr: u64) -> Option<u64> {
    unsafe {
        let state = (*RUNTIME_STATE.0.get()).as_ref().expect("runtime state installed");
        let image_size = (state.transition_alias_page_count * crate::memory::PAGE_SIZE) as u64;
        let image_end = state.transition_alias_start.checked_add(image_size)?;
        let runtime_image_end = KERNEL_VIRT_BASE.checked_add(image_size)?;

        if addr >= KERNEL_VIRT_BASE && addr < runtime_image_end {
            return Some(addr);
        }

        if addr < state.transition_alias_start || addr >= image_end {
            return None;
        }

        KERNEL_VIRT_BASE.checked_add(addr - state.transition_alias_start)
    }
}

pub unsafe fn remove_transition_alias() -> Result<(), PagingError> {
    unsafe {
        let state = (*RUNTIME_STATE.0.get())
            .as_mut()
            .expect("runtime state installed");
        unmap_range(
            &mut state.kernel_space,
            state.transition_alias_start,
            state.transition_alias_page_count,
        )?;

        for page in 0..state.transition_alias_page_count {
            let virt_addr = state.transition_alias_start + (page * crate::memory::PAGE_SIZE) as u64;
            invalidate_tlb_page(virt_addr);
        }

        Ok(())
    }
}
