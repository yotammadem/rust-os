use rust_os::memory::{
    BootMemoryMapSnapshot, MemoryRegion, PAGE_SIZE, RegionKind, align_down, align_up,
};

#[test]
fn alignment_helpers_match_page_size() {
    assert_eq!(align_up(0x1003, PAGE_SIZE as u64), 0x2000);
    assert_eq!(align_down(0x3fff, PAGE_SIZE as u64), 0x3000);
}

#[test]
fn normalized_region_discards_sub_page_fragments() {
    let region = MemoryRegion::normalized(0x1003, 0x2ffd, RegionKind::Usable).unwrap();

    assert_eq!(region.start_phys_addr, 0x2000);
    assert_eq!(region.page_count, 2);
    assert_eq!(region.length_bytes, (PAGE_SIZE * 2) as u64);
}

#[test]
fn snapshot_reports_managed_page_count_from_highest_usable_address() {
    let regions = [
        MemoryRegion::from_aligned_range(0x1000, 0x4000, RegionKind::Reserved),
        MemoryRegion::from_aligned_range(0x4000, 0x9000, RegionKind::Usable),
    ];
    let snapshot = BootMemoryMapSnapshot {
        regions: &regions,
        descriptor_count: 2,
        descriptor_size: 48,
        page_size: PAGE_SIZE,
        highest_usable_address: 0x9000,
    };

    assert_eq!(snapshot.managed_page_count(), 9);
}
