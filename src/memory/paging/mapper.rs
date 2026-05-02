use crate::memory::{BitmapAllocator, PAGE_SIZE, PhysAddr};

use super::address_space::{AddressSpace, PagingAllocationRecord};
use super::table::{
    EntryFlags, MappingRequest, PAGE_TABLE_ADDR_MASK, PageTableLevel, VirtualAddressLayout,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PagingError {
    AddressOutOfRange,
    AllocatorStateCorrupted,
    CapacityExceeded,
    EmptyRequest,
    MappingConflict,
    OutOfMemory,
    UnalignedAddress,
}

pub fn map_range(
    address_space: &mut AddressSpace,
    allocator: &mut BitmapAllocator<'_>,
    request: MappingRequest,
) -> Result<(), PagingError> {
    validate_request(address_space, request)?;

    let mut record = PagingAllocationRecord::new();
    for page in 0..request.page_count {
        let virt_addr = request.start_virt_addr + (page * PAGE_SIZE) as u64;
        let phys_addr = request.target_phys_start + (page * PAGE_SIZE) as u64;
        if let Err(err) = map_single_page(
            address_space,
            allocator,
            &mut record,
            virt_addr,
            phys_addr,
            request.flags,
            request.allow_overwrite,
        ) {
            record.rollback(allocator)?;
            return Err(err);
        }
    }

    record.publish();
    for span in record.spans() {
        address_space.record_owned_span(span)?;
    }
    Ok(())
}

pub fn unmap_range(
    address_space: &mut AddressSpace,
    start_virt_addr: u64,
    page_count: usize,
) -> Result<(), PagingError> {
    if page_count == 0 {
        return Err(PagingError::EmptyRequest);
    }
    if !VirtualAddressLayout::is_page_aligned(start_virt_addr) {
        return Err(PagingError::UnalignedAddress);
    }

    for page in 0..page_count {
        let virt_addr = start_virt_addr + (page * PAGE_SIZE) as u64;
        clear_single_page(address_space, virt_addr)?;
    }

    Ok(())
}

fn validate_request(
    address_space: &AddressSpace,
    request: MappingRequest,
) -> Result<(), PagingError> {
    if request.page_count == 0 {
        return Err(PagingError::EmptyRequest);
    }
    if !VirtualAddressLayout::is_page_aligned(request.start_virt_addr)
        || !VirtualAddressLayout::is_page_aligned(request.target_phys_start)
    {
        return Err(PagingError::UnalignedAddress);
    }

    let end = request.end_virt_addr_exclusive();
    let allowed = if VirtualAddressLayout::is_kernel_address(request.start_virt_addr) {
        true
    } else {
        request.start_virt_addr >= address_space.private_mapping_floor
            && end <= address_space.private_mapping_ceiling
    };
    if !allowed {
        return Err(PagingError::AddressOutOfRange);
    }
    Ok(())
}

fn map_single_page(
    address_space: &mut AddressSpace,
    allocator: &mut BitmapAllocator<'_>,
    record: &mut PagingAllocationRecord,
    virt_addr: u64,
    phys_addr: PhysAddr,
    flags: EntryFlags,
    allow_overwrite: bool,
) -> Result<(), PagingError> {
    let indexes = VirtualAddressLayout::indexes(virt_addr);
    let mut table_phys = address_space.root_table_phys_addr;
    let mut level = PageTableLevel::Pml4;

    for index in indexes[..3].iter().copied() {
        let entry = address_space.read_table_entry(table_phys, index);
        let child_phys = if entry == 0 {
            let child_phys = address_space.allocate_table_page(allocator, record)?;
            address_space.write_table_entry(
                table_phys,
                index,
                child_phys | EntryFlags::PRESENT.bits() | EntryFlags::WRITABLE.bits(),
            )?;
            child_phys
        } else {
            entry & PAGE_TABLE_ADDR_MASK
        };

        table_phys = child_phys;
        level = level.child().expect("non-leaf");
    }

    let leaf_index = indexes[3];
    let existing = address_space.read_table_entry(table_phys, leaf_index);
    if existing != 0 && !allow_overwrite {
        return Err(PagingError::MappingConflict);
    }

    address_space.write_table_entry(
        table_phys,
        leaf_index,
        phys_addr | flags.bits() | EntryFlags::PRESENT.bits(),
    )?;
    Ok(())
}

fn clear_single_page(address_space: &mut AddressSpace, virt_addr: u64) -> Result<(), PagingError> {
    let indexes = VirtualAddressLayout::indexes(virt_addr);
    let mut table_phys = address_space.root_table_phys_addr;

    for index in indexes[..3].iter().copied() {
        let entry = address_space.read_table_entry(table_phys, index);
        if entry == 0 {
            return Ok(());
        }
        table_phys = entry & PAGE_TABLE_ADDR_MASK;
    }

    address_space.clear_table_entry(table_phys, indexes[3]);
    Ok(())
}
