use rust_os::{
    KERNEL_BOOT_PHYS_BASE,
    arch::x86_64::paging::{ActivationPlan, higher_half_alias_addr},
    memory::{
        AddressSpace, BitmapAllocator, EntryFlags, KERNEL_DIRECT_MAP_BASE, MappingRequest,
        MemoryRegion, RegionKind, VirtualAddressLayout, map_range,
        reset_host_shadow_page_tables_for_tests, unmap_range,
    },
};

fn allocator_fixture() -> BitmapAllocator<'static> {
    reset_host_shadow_page_tables_for_tests();

    let regions = [
        MemoryRegion::from_aligned_range(0x0000, 0x1000, RegionKind::Reserved),
        MemoryRegion::from_aligned_range(0x1000, 0x200000, RegionKind::Usable),
    ];

    let storage_len = BitmapAllocator::required_storage_bytes_for_managed_pages(512);
    let storage = vec![0u8; storage_len].into_boxed_slice();
    let storage = Box::leak(storage);

    BitmapAllocator::from_regions(&regions, storage).expect("allocator should initialize")
}

#[test]
fn paging_indices_match_x86_64_layout() {
    let addr = 0xffff_8000_1234_5000u64;
    assert_eq!(VirtualAddressLayout::pml4_index(addr), 256);
    assert_eq!(VirtualAddressLayout::pdpt_index(addr), 0);
    assert_eq!(VirtualAddressLayout::pd_index(addr), 145);
    assert_eq!(VirtualAddressLayout::pt_index(addr), 325);
}

#[test]
fn direct_map_translation_round_trips_within_managed_range() {
    let managed_phys_limit = 0x20_0000;
    let virt_addr =
        VirtualAddressLayout::phys_to_direct_map_virt(0x0014_5000, managed_phys_limit)
            .expect("physical address should fit the direct map");
    assert_eq!(virt_addr, KERNEL_DIRECT_MAP_BASE + 0x0014_5000);
    assert_eq!(
        VirtualAddressLayout::direct_map_virt_to_phys(virt_addr, managed_phys_limit),
        Some(0x0014_5000)
    );
    assert!(
        VirtualAddressLayout::phys_to_direct_map_virt(managed_phys_limit, managed_phys_limit)
            .is_none()
    );
}

#[test]
fn kernel_template_maps_kernel_direct_map_and_transition_alias() {
    let mut allocator = allocator_fixture();
    let (mut kernel, template) =
        AddressSpace::create_kernel_template(&mut allocator, KERNEL_BOOT_PHYS_BASE, 4)
            .expect("kernel template should build");

    let higher_half = kernel
        .translate(0xffff_8000_0000_0000)
        .expect("higher-half mapping present");
    assert_eq!(higher_half.phys_addr, KERNEL_BOOT_PHYS_BASE);

    let alias = kernel
        .translate(KERNEL_BOOT_PHYS_BASE)
        .expect("transition alias present");
    assert_eq!(alias.phys_addr, KERNEL_BOOT_PHYS_BASE);

    let direct_map = kernel
        .translate(KERNEL_DIRECT_MAP_BASE + 0x3000)
        .expect("direct-map translation present");
    assert_eq!(direct_map.phys_addr, 0x3000);
    assert_eq!(template.managed_phys_limit, 0x200000);

    unmap_range(
        &mut kernel,
        template.transition_alias_start,
        template.transition_alias_page_count,
    )
    .expect("alias removal should succeed");
    assert!(kernel.translate(KERNEL_BOOT_PHYS_BASE).is_none());
}

#[test]
fn higher_half_alias_address_uses_transition_alias_offset() {
    let low_addr = KERNEL_BOOT_PHYS_BASE + 0x3456;
    let higher_half =
        higher_half_alias_addr(low_addr, KERNEL_BOOT_PHYS_BASE).expect("offset should fit");
    assert_eq!(higher_half, 0xffff_8000_0000_3456);
}

#[test]
fn activation_plan_carries_higher_half_stack_and_alias_metadata() {
    let mut allocator = allocator_fixture();
    let (kernel, template) =
        AddressSpace::create_kernel_template(&mut allocator, KERNEL_BOOT_PHYS_BASE, 2)
            .expect("kernel template should build");

    let plan = ActivationPlan::from_template(
        kernel.root_table_phys_addr,
        0xffff_8000_0000_2000,
        0xffff_8000_0200_8000,
        0x0000_0000_0010_0000,
        &template,
    );

    assert_eq!(plan.root_table_phys_addr, kernel.root_table_phys_addr);
    assert_eq!(plan.higher_half_entry_addr, 0xffff_8000_0000_2000);
    assert_eq!(plan.higher_half_stack_top, 0xffff_8000_0200_8000);
    assert_eq!(plan.runtime_context_addr, 0x0000_0000_0010_0000);
    assert_eq!(plan.transition_alias_start, KERNEL_BOOT_PHYS_BASE);
    assert_eq!(plan.transition_alias_page_count, 2);
}

#[test]
fn kernel_template_leaves_kernel_alloc_window_available() {
    let mut allocator = allocator_fixture();
    let (mut kernel, _) =
        AddressSpace::create_kernel_template(&mut allocator, KERNEL_BOOT_PHYS_BASE, 4)
            .expect("kernel template should build");

    let allocation = kernel
        .allocate_kernel_virtual(&mut allocator, 32, EntryFlags::WRITABLE | EntryFlags::NO_EXECUTE)
        .expect("kernel alloc window should remain usable after template setup");

    assert!(allocation.virt_start_addr >= rust_os::memory::KERNEL_ALLOC_BASE);
}

#[test]
fn map_range_rolls_back_intermediate_allocations_on_conflict() {
    let mut allocator = allocator_fixture();
    let mut space = AddressSpace::new_kernel(&mut allocator).expect("kernel space");
    let before = allocator.free_page_count();

    map_range(
        &mut space,
        &mut allocator,
        MappingRequest {
            start_virt_addr: 0xffff_8000_0000_0000,
            target_phys_start: 0x4000,
            page_count: 2,
            flags: EntryFlags::WRITABLE,
            allow_overwrite: false,
        },
    )
    .expect("initial mapping should work");

    let after_first = allocator.free_page_count();
    let err = map_range(
        &mut space,
        &mut allocator,
        MappingRequest {
            start_virt_addr: 0xffff_8000_0000_1000,
            target_phys_start: 0x9000,
            page_count: 2,
            flags: EntryFlags::WRITABLE,
            allow_overwrite: false,
        },
    )
    .expect_err("overlap should fail");
    assert_eq!(err, rust_os::memory::PagingError::MappingConflict);
    assert_eq!(allocator.free_page_count(), after_first);
    assert!(after_first < before);
}

#[test]
fn kernel_owned_virtual_allocation_consumes_allocator_pages() {
    let mut allocator = allocator_fixture();
    let mut space = AddressSpace::new_kernel(&mut allocator).expect("kernel space");
    let before = allocator.free_page_count();

    let allocation = space
        .allocate_kernel_virtual(&mut allocator, 3, EntryFlags::WRITABLE)
        .expect("allocation should succeed");

    assert_eq!(allocation.page_count, 3);
    assert!(space.translate(allocation.virt_start_addr).is_some());
    assert!(allocator.free_page_count() < before);
}

#[test]
fn process_address_spaces_share_kernel_mapping_but_keep_private_mappings_isolated() {
    let mut allocator = allocator_fixture();
    let (kernel, template) =
        AddressSpace::create_kernel_template(&mut allocator, KERNEL_BOOT_PHYS_BASE, 2)
            .expect("kernel template should build");

    let mut process_a =
        AddressSpace::from_kernel_template(&mut allocator, &template).expect("process a");
    let process_b =
        AddressSpace::from_kernel_template(&mut allocator, &template).expect("process b");

    let kernel_page = kernel
        .translate(0xffff_8000_0000_0000)
        .expect("kernel mapping");
    let process_page = process_a
        .translate(0xffff_8000_0000_0000)
        .expect("shared mapping");
    assert_eq!(kernel_page.phys_addr, process_page.phys_addr);
    assert_eq!(
        process_a
            .translate(KERNEL_DIRECT_MAP_BASE + 0x5000)
            .expect("process inherits direct map")
            .phys_addr,
        0x5000
    );

    map_range(
        &mut process_a,
        &mut allocator,
        MappingRequest {
            start_virt_addr: 0x0040_0000,
            target_phys_start: 0x12000,
            page_count: 1,
            flags: EntryFlags::WRITABLE | EntryFlags::USER,
            allow_overwrite: false,
        },
    )
    .expect("private mapping should succeed");

    assert!(process_a.translate(0x0040_0000).is_some());
    assert!(process_b.translate(0x0040_0000).is_none());
}

#[test]
fn destroy_reclaims_process_private_paging_pages() {
    let mut allocator = allocator_fixture();
    let (_, template) =
        AddressSpace::create_kernel_template(&mut allocator, KERNEL_BOOT_PHYS_BASE, 2)
            .expect("kernel template should build");
    let mut process =
        AddressSpace::from_kernel_template(&mut allocator, &template).expect("process");
    let before_mapping = allocator.free_page_count();

    map_range(
        &mut process,
        &mut allocator,
        MappingRequest {
            start_virt_addr: 0x0080_0000,
            target_phys_start: 0x18000,
            page_count: 4,
            flags: EntryFlags::WRITABLE | EntryFlags::USER,
            allow_overwrite: false,
        },
    )
    .expect("mapping should succeed");
    let after_mapping = allocator.free_page_count();
    assert!(after_mapping < before_mapping);

    process.destroy(&mut allocator).expect("destroy should reclaim");
    assert!(allocator.free_page_count() > after_mapping);
}
