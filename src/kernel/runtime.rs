use core::cell::UnsafeCell;

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

pub unsafe fn remove_transition_alias() -> Result<(), PagingError> {
    unsafe {
        let state = (*RUNTIME_STATE.0.get())
            .as_mut()
            .expect("runtime state installed");
        unmap_range(
            &mut state.kernel_space,
            state.transition_alias_start,
            state.transition_alias_page_count,
        )
    }
}
