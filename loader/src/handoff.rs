use crate::kernel_image::LoadedKernelImage;
use crate::memory::EarlyLayout;
use crate::paging::BuiltPageTables;
use rust_os::boot::handoff::{
    BootInfo, KernelImageInfo, MAX_USABLE_RANGES, PagingInfo, PhysicalRange,
};

const PAGE_SIZE: u64 = 4096;

#[derive(Clone, Copy)]
pub struct PreparedHandoff {
    pub physical_address: u64,
    pub boot_info: &'static BootInfo,
}

#[derive(Clone, Copy)]
pub struct HandoffError {
    pub stage: &'static [u8],
}

pub fn prepare(
    mut boot_info: BootInfo,
    layout: EarlyLayout,
    loaded_kernel: LoadedKernelImage,
    page_tables: BuiltPageTables,
) -> Result<PreparedHandoff, HandoffError> {
    let physical_address = layout.boot_info_region.start;
    if layout.boot_info_region.is_empty() {
        return Err(HandoffError {
            stage: b"boot_info_region",
        });
    }

    let boot_info_size = core::mem::size_of::<BootInfo>() as u64;
    if boot_info_size > layout.boot_info_region.size_bytes() {
        return Err(HandoffError {
            stage: b"boot_info_size",
        });
    }

    boot_info.usable_range_count = 0;
    let post_kernel_usable = remaining_kernel_usable(layout.kernel_usable_region, loaded_kernel);
    if !post_kernel_usable.is_empty() {
        if MAX_USABLE_RANGES == 0 {
            return Err(HandoffError {
                stage: b"usable_range_capacity",
            });
        }
        boot_info.usable_ranges[0] = post_kernel_usable;
        boot_info.usable_range_count = 1;
    }

    boot_info.kernel_image = KernelImageInfo {
        physical_range: PhysicalRange {
            start: loaded_kernel.physical_start,
            end: loaded_kernel.physical_end,
        },
        virtual_range: kernel_virtual_range(loaded_kernel)?,
        entry_point: loaded_kernel.entry_point,
    };
    boot_info.paging = PagingInfo {
        pml4_physical_start: page_tables.pml4_physical_start,
        kernel_stack_physical: page_tables.kernel_stack_physical,
        kernel_stack_virtual: page_tables.kernel_stack_virtual,
    };

    let boot_info_ptr = physical_address as usize as *mut BootInfo;
    unsafe {
        boot_info_ptr.write(boot_info);
    }

    Ok(PreparedHandoff {
        physical_address,
        boot_info: unsafe { &*boot_info_ptr },
    })
}

fn remaining_kernel_usable(
    kernel_usable_region: PhysicalRange,
    loaded_kernel: LoadedKernelImage,
) -> PhysicalRange {
    if kernel_usable_region.is_empty() {
        return PhysicalRange::empty();
    }

    let start = align_up(loaded_kernel.physical_end, PAGE_SIZE);
    if start >= kernel_usable_region.end {
        return PhysicalRange::empty();
    }

    PhysicalRange {
        start,
        end: kernel_usable_region.end,
    }
}

fn kernel_virtual_range(loaded_kernel: LoadedKernelImage) -> Result<PhysicalRange, HandoffError> {
    let mut start = u64::MAX;
    let mut end = 0u64;

    for segment in loaded_kernel.segments[..loaded_kernel.segment_count]
        .iter()
        .copied()
    {
        if segment.memory_size == 0 {
            continue;
        }

        start = start.min(segment.virtual_address);
        end = end.max(segment.virtual_address.saturating_add(segment.memory_size));
    }

    if start >= end {
        return Err(HandoffError {
            stage: b"kernel_virtual_range",
        });
    }

    Ok(PhysicalRange { start, end })
}

fn align_up(value: u64, align: u64) -> u64 {
    let mask = align - 1;
    value.saturating_add(mask) & !mask
}
