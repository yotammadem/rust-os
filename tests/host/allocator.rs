use rust_os::memory::{
    AllocationResult, BitmapAllocator, MemoryRegion, PAGE_SIZE, PageSpan, RegionKind,
};

fn allocator_fixture() -> BitmapAllocator<'static> {
    let regions = [
        MemoryRegion::from_aligned_range(0x0000, 0x1000, RegionKind::Reserved),
        MemoryRegion::from_aligned_range(0x1000, 0x9000, RegionKind::Usable),
        MemoryRegion::from_aligned_range(0x9000, 0xb000, RegionKind::Kernel),
        MemoryRegion::from_aligned_range(0xb000, 0xc000, RegionKind::Boot),
    ];

    let storage_len = BitmapAllocator::required_storage_bytes_for_managed_pages(12);
    let storage = vec![0u8; storage_len].into_boxed_slice();
    let storage = Box::leak(storage);

    BitmapAllocator::from_regions(&regions, storage).expect("allocator should initialize")
}

#[test]
fn initialization_marks_reserved_kernel_boot_and_metadata_pages_unavailable() {
    let allocator = allocator_fixture();
    let metadata = allocator.metadata_span();

    assert!(!allocator.is_page_allocatable(0));
    assert!(!allocator.is_page_allocatable(metadata.start_page_index));
    assert!(!allocator.is_page_allocatable(9));
    assert!(!allocator.is_page_allocatable(11));
    assert!(allocator.is_page_allocatable(metadata.start_page_index + metadata.page_count));
}

#[test]
fn initialization_counts_only_free_usable_pages() {
    let allocator = allocator_fixture();

    assert_eq!(allocator.free_page_count(), 7);
    assert_eq!(allocator.managed_page_count(), 12);
}

#[test]
fn allocate_page_returns_first_free_page_after_metadata() {
    let mut allocator = allocator_fixture();

    let AllocationResult::Allocated(span) = allocator.allocate_page() else {
        panic!("expected allocated page");
    };

    assert_eq!(span.start_phys_addr, (2 * PAGE_SIZE) as u64);
    assert_eq!(allocator.free_page_count(), 6);
}

#[test]
fn allocate_multiple_pages_requires_contiguous_run() {
    let mut allocator = allocator_fixture();

    let _ = allocator.allocate_page();
    let AllocationResult::Allocated(span) = allocator.allocate_pages(2) else {
        panic!("expected contiguous allocation");
    };

    assert_eq!(span, PageSpan::new(3, 2).unwrap());
}

#[test]
fn zero_page_request_is_rejected() {
    let mut allocator = allocator_fixture();

    assert_eq!(
        allocator.allocate_pages(0),
        AllocationResult::InvalidRequest
    );
}

#[test]
fn exhausted_allocator_reports_out_of_memory() {
    let mut allocator = allocator_fixture();

    while matches!(allocator.allocate_page(), AllocationResult::Allocated(_)) {}

    assert_eq!(allocator.allocate_page(), AllocationResult::OutOfMemory);
}

#[test]
fn free_releases_allocated_pages_for_reuse() {
    let mut allocator = allocator_fixture();

    let AllocationResult::Allocated(span) = allocator.allocate_pages(2) else {
        panic!("expected allocated span");
    };
    assert_eq!(allocator.free_pages(span), AllocationResult::Released(span));

    let AllocationResult::Allocated(reused) = allocator.allocate_pages(2) else {
        panic!("expected reused span");
    };
    assert_eq!(reused, span);
}

#[test]
fn invalid_free_of_metadata_page_is_rejected_without_state_change() {
    let mut allocator = allocator_fixture();
    let metadata = allocator.metadata_span();
    let free_pages_before = allocator.free_page_count();

    assert_eq!(
        allocator.free_pages(metadata),
        AllocationResult::InvalidFree
    );
    assert_eq!(allocator.free_page_count(), free_pages_before);
}

#[test]
fn invalid_free_of_never_allocated_span_is_rejected() {
    let mut allocator = allocator_fixture();

    assert_eq!(
        allocator.free_pages(PageSpan::new(4, 2).unwrap()),
        AllocationResult::InvalidFree
    );
}

#[test]
fn rebasing_bootstrap_storage_moves_allocator_bitmap_accesses() {
    let regions = [
        MemoryRegion::from_aligned_range(0x0000, 0x1000, RegionKind::Reserved),
        MemoryRegion::from_aligned_range(0x1000, 0x9000, RegionKind::Usable),
        MemoryRegion::from_aligned_range(0x9000, 0xb000, RegionKind::Kernel),
        MemoryRegion::from_aligned_range(0xb000, 0xc000, RegionKind::Boot),
    ];

    let storage_len = BitmapAllocator::required_storage_bytes_for_managed_pages(12);
    let low_storage = vec![0u8; storage_len].into_boxed_slice();
    let low_storage = Box::leak(low_storage);
    let low_base = low_storage.as_mut_ptr() as usize as u64;

    let mut allocator =
        BitmapAllocator::from_regions(&regions, low_storage).expect("allocator should initialize");

    let mut high_storage = vec![0u8; storage_len].into_boxed_slice();
    high_storage.copy_from_slice(unsafe {
        core::slice::from_raw_parts(low_base as *const u8, storage_len)
    });
    let high_base = high_storage.as_mut_ptr() as usize as u64;

    let low_byte_before = unsafe { *(low_base as *const u8) };
    let high_byte_before = high_storage[0];

    assert!(unsafe { allocator.rebase_bootstrap_storage(low_base, high_base) });

    let AllocationResult::Allocated(span) = allocator.allocate_page() else {
        panic!("expected allocated page");
    };
    assert_eq!(span.start_phys_addr, (2 * PAGE_SIZE) as u64);

    let low_byte_after = unsafe { *(low_base as *const u8) };
    let high_byte_after = unsafe { *(high_base as *const u8) };

    assert_eq!(low_byte_after, low_byte_before);
    assert_ne!(high_byte_after, high_byte_before);
}
