use crate::kernel_image::LoadedKernelImage;
use crate::memory::{EarlyLayout, PhysicalRange};
use core::arch::asm;
use rust_os::boot::handoff::BootInfo;

const PAGE_SIZE: u64 = 4096;
const STACK_WINDOW_BYTES: u64 = 16 * PAGE_SIZE;
const ENTRY_COUNT: usize = 512;
const ADDRESS_MASK: u64 = 0x000f_ffff_ffff_f000;
const PRESENT: u64 = 1 << 0;
const WRITABLE: u64 = 1 << 1;
const NO_EXECUTE: u64 = 1 << 63;
const ELF_FLAG_EXECUTABLE: u32 = 1 << 0;
const ELF_FLAG_WRITABLE: u32 = 1 << 1;

#[repr(C, align(4096))]
struct PageTable {
    entries: [u64; ENTRY_COUNT],
}

#[derive(Clone, Copy)]
pub struct BuiltPageTables {
    pub pml4_physical_start: u64,
    pub pages_used: usize,
    pub stack_window: PhysicalRange,
    pub memory_map_window: PhysicalRange,
}

#[derive(Clone, Copy)]
pub struct BuildError {
    pub stage: &'static [u8],
}

pub fn build(
    boot_info: &BootInfo,
    layout: EarlyLayout,
    loaded_kernel: LoadedKernelImage,
) -> Result<BuiltPageTables, BuildError> {
    let stack_window = current_stack_window();
    let memory_map_window = page_align_range(PhysicalRange {
        start: boot_info.memory_map.map as usize as u64,
        end: boot_info.memory_map.map as usize as u64 + boot_info.memory_map.map_size as u64,
    });
    let loader_image = PhysicalRange {
        start: boot_info.loader_image.start,
        end: boot_info.loader_image.end,
    };

    let mut builder = PageTableBuilder::new(layout.page_table_region)?;
    builder.map_identity(loader_image, WRITABLE)?;
    builder.map_identity(layout.boot_info_region, WRITABLE)?;
    builder.map_identity(layout.page_table_region, WRITABLE)?;
    builder.map_identity(stack_window, WRITABLE)?;
    builder.map_identity(memory_map_window, WRITABLE)?;

    for segment in loaded_kernel.segments[..loaded_kernel.segment_count]
        .iter()
        .copied()
    {
        if segment.memory_size == 0 {
            continue;
        }

        let mut flags = PRESENT;
        if segment.flags & ELF_FLAG_WRITABLE != 0 {
            flags |= WRITABLE;
        }
        if segment.flags & ELF_FLAG_EXECUTABLE == 0 {
            flags |= NO_EXECUTE;
        }

        builder.map_range(
            segment.virtual_address,
            segment.physical_start,
            segment.memory_size,
            flags,
        )?;
    }

    Ok(BuiltPageTables {
        pml4_physical_start: builder.pml4_physical_start,
        pages_used: builder.pages_used,
        stack_window,
        memory_map_window,
    })
}

struct PageTableBuilder {
    next_free: u64,
    region_end: u64,
    pml4_physical_start: u64,
    pages_used: usize,
}

impl PageTableBuilder {
    fn new(page_table_region: PhysicalRange) -> Result<Self, BuildError> {
        if page_table_region.is_empty() || page_table_region.size_bytes() < PAGE_SIZE {
            return Err(BuildError {
                stage: b"page_table_region",
            });
        }

        let pml4_physical_start = align_up(page_table_region.start, PAGE_SIZE);
        let mut builder = Self {
            next_free: pml4_physical_start,
            region_end: page_table_region.end,
            pml4_physical_start,
            pages_used: 0,
        };
        let _ = builder.allocate_table()?;
        Ok(builder)
    }

    fn map_identity(&mut self, range: PhysicalRange, flags: u64) -> Result<(), BuildError> {
        if range.is_empty() {
            return Ok(());
        }

        self.map_range(range.start, range.start, range.size_bytes(), flags)
    }

    fn map_range(
        &mut self,
        virtual_start: u64,
        physical_start: u64,
        size_bytes: u64,
        flags: u64,
    ) -> Result<(), BuildError> {
        if size_bytes == 0 {
            return Ok(());
        }

        if (virtual_start & (PAGE_SIZE - 1)) != (physical_start & (PAGE_SIZE - 1)) {
            return Err(BuildError {
                stage: b"page_offset",
            });
        }

        let mapped_end = virtual_start.checked_add(size_bytes).ok_or(BuildError {
            stage: b"range_end",
        })?;
        let mut virtual_page = align_down(virtual_start, PAGE_SIZE);
        let mut physical_page = align_down(physical_start, PAGE_SIZE);
        let end_page = align_up(mapped_end, PAGE_SIZE);

        while virtual_page < end_page {
            self.map_page(virtual_page, physical_page, flags)?;
            virtual_page += PAGE_SIZE;
            physical_page += PAGE_SIZE;
        }

        Ok(())
    }

    fn map_page(
        &mut self,
        virtual_address: u64,
        physical_address: u64,
        flags: u64,
    ) -> Result<(), BuildError> {
        let pdpt =
            self.get_or_create_child(self.pml4_physical_start, table_index(virtual_address, 39))?;
        let pd = self.get_or_create_child(pdpt, table_index(virtual_address, 30))?;
        let pt = self.get_or_create_child(pd, table_index(virtual_address, 21))?;

        let entry_index = table_index(virtual_address, 12);
        self.table_mut(pt).entries[entry_index] = (physical_address & ADDRESS_MASK) | flags;
        Ok(())
    }

    fn get_or_create_child(
        &mut self,
        table_physical: u64,
        entry_index: usize,
    ) -> Result<u64, BuildError> {
        let entry = self.table_mut(table_physical).entries[entry_index];
        if entry & PRESENT != 0 {
            return Ok(table_address(entry));
        }

        let child_physical = self.allocate_table()?;
        self.table_mut(table_physical).entries[entry_index] = child_physical | PRESENT | WRITABLE;
        Ok(child_physical)
    }

    fn allocate_table(&mut self) -> Result<u64, BuildError> {
        let table_physical = align_up(self.next_free, PAGE_SIZE);
        let next = table_physical.checked_add(PAGE_SIZE).ok_or(BuildError {
            stage: b"table_overflow",
        })?;
        if next > self.region_end {
            return Err(BuildError {
                stage: b"page_table_capacity",
            });
        }

        let table_ptr = table_physical as usize as *mut PageTable;
        unsafe {
            core::ptr::write_bytes(table_ptr.cast::<u8>(), 0, PAGE_SIZE as usize);
        }

        self.next_free = next;
        self.pages_used += 1;
        Ok(table_physical)
    }

    fn table_mut(&self, physical_address: u64) -> &mut PageTable {
        unsafe { &mut *(physical_address as usize as *mut PageTable) }
    }
}

fn current_stack_window() -> PhysicalRange {
    let mut stack_pointer = 0u64;
    unsafe {
        asm!("mov {}, rsp", out(reg) stack_pointer, options(nomem, nostack, preserves_flags));
    }

    let end = align_up(stack_pointer.saturating_add(PAGE_SIZE), PAGE_SIZE);
    let start = align_down(end.saturating_sub(STACK_WINDOW_BYTES), PAGE_SIZE);
    PhysicalRange { start, end }
}

fn page_align_range(range: PhysicalRange) -> PhysicalRange {
    if range.is_empty() {
        return PhysicalRange::empty();
    }

    PhysicalRange {
        start: align_down(range.start, PAGE_SIZE),
        end: align_up(range.end, PAGE_SIZE),
    }
}

fn table_index(virtual_address: u64, shift: u64) -> usize {
    ((virtual_address >> shift) & 0x1ff) as usize
}

fn table_address(entry: u64) -> u64 {
    entry & ADDRESS_MASK
}

fn align_up(value: u64, align: u64) -> u64 {
    let mask = align - 1;
    value.saturating_add(mask) & !mask
}

fn align_down(value: u64, align: u64) -> u64 {
    value & !(align - 1)
}
