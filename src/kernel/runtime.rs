use core::cell::UnsafeCell;

use crate::memory::{BitmapAllocator, PhysAddr};

struct RuntimeState {
    allocator: BitmapAllocator<'static>,
    managed_phys_limit: PhysAddr,
}

struct RuntimeStateCell(UnsafeCell<Option<RuntimeState>>);

unsafe impl Sync for RuntimeStateCell {}

static RUNTIME_STATE: RuntimeStateCell = RuntimeStateCell(UnsafeCell::new(None));

pub fn install(allocator: BitmapAllocator<'static>, managed_phys_limit: PhysAddr) {
    unsafe {
        // Safety: early boot is single-threaded and installs runtime state once
        // before handing control to the higher-half continuation.
        *RUNTIME_STATE.0.get() = Some(RuntimeState {
            allocator,
            managed_phys_limit,
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
