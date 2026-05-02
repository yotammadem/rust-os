# Data Model: Virtual Memory Manager

## AddressSpace

- **Purpose**: Complete translation state for either the kernel or one process.
- **Fields**:
  - `root_table_phys_addr`: Physical address of the top-level paging structure
  - `root_table_virt_addr`: Kernel-accessible virtual address used to edit the
    root table after bootstrap
  - `kind`: One of `Kernel` or `Process`
  - `owned_table_pages`: Collection of physical pages owned exclusively by this
    address space for paging structures
  - `shared_kernel_template_id`: Identifier or reference to the shared kernel
    mapping template used by process address spaces
  - `private_mapping_floor` and `private_mapping_ceiling`: Bounds that define
    the process-private virtual range for on-demand mappings
- **Validation Rules**:
  - `root_table_phys_addr` must be page-aligned and allocator-owned
  - Kernel shared mappings must not appear inside another process's
    `owned_table_pages`
  - Process-private mapping bounds must not overlap the shared kernel higher-half

## KernelMappingTemplate

- **Purpose**: Reusable description of the kernel's higher-half mappings that
  every process address space must share.
- **Fields**:
  - `kernel_virtual_base`: Start of the kernel higher-half virtual range
  - `mapped_regions`: Kernel code, data, stack, boot-critical buffers, and any
    transition trampoline mappings required during activation
  - `shared_table_pages`: Paging-structure pages that may be referenced from
    multiple process roots
  - `transition_alias_range`: Temporary low-address alias retained only during
    the bootstrap switch
- **Validation Rules**:
  - Shared mappings must cover all kernel regions required after the transition
  - `transition_alias_range` must be removable once higher-half execution begins
  - Shared table pages must never be reclaimed by process teardown

## PagingStructurePage

- **Purpose**: One physical 4 KiB page that stores entries for one level of the
  paging hierarchy.
- **Fields**:
  - `phys_addr`: Backing physical page address
  - `level`: One of `Pml4`, `Pdpt`, `Pd`, or `Pt`
  - `entry_count`: Number of active entries currently used
  - `owner`: Address space or shared-kernel ownership classification
  - `parent_entry`: Optional parent structure and slot that reference this page
- **Validation Rules**:
  - `phys_addr` must be unique within the combined ownership graph
  - `entry_count` must not exceed the architecture-defined maximum entry count
  - Shared-kernel pages must not contain process-private lower-half mappings

## MappingRequest

- **Purpose**: Canonical request to ensure a virtual range is represented in an
  address space using caller-supplied physical backing pages.
- **Fields**:
  - `start_virt_addr`: Inclusive virtual start address
  - `page_count`: Number of 4 KiB pages to cover
  - `target_phys_start`: Physical start address of the already-allocated backing
    range to map
  - `flags`: Writable, executable, global, and kernel/user accessibility bits
  - `allow_overwrite`: Whether an existing compatible mapping may be replaced
- **Validation Rules**:
  - `page_count` must be greater than zero
  - Start and end addresses must be page-aligned
  - Requests that overlap incompatible existing mappings are rejected

## KernelVirtualAllocation

- **Purpose**: Represents a VMM-owned higher-half kernel allocation whose
  physical backing pages and paging structures are both obtained through the
  bitmap allocator.
- **Fields**:
  - `virt_start_addr`: Inclusive kernel virtual start address
  - `backing_span`: Physical page span allocated for the payload
  - `page_count`: Number of 4 KiB pages mapped into the kernel space
  - `flags`: Mapping attributes applied to the kernel range
  - `owner_record`: Ownership handle tying the allocation to allocator-backed
    backing pages and any newly created paging-structure pages
- **Validation Rules**:
  - The virtual range must lie within the kernel higher-half allocation region
  - `backing_span.page_count` must equal `page_count`
  - Releasing the allocation must return both its backing pages and any
    allocation-local paging structures that are no longer referenced

## PagingAllocationRecord

- **Purpose**: Tracks paging pages acquired during one address-space creation or
  mapping operation so rollback and teardown are deterministic.
- **Fields**:
  - `operation_id`: Identifier for the in-flight creation or expansion request
  - `new_table_pages`: Ordered list of newly allocated paging-structure pages
  - `published`: Whether the new structures became visible in the target address
    space
  - `rollback_boundary`: Last safe point before exposing new mappings
- **Validation Rules**:
  - Unpublished records must be fully releasable to the allocator
  - Published records become part of the owning address space's permanent
    `owned_table_pages`
  - Rollback must not free shared kernel template pages

## BootstrapTransitionPlan

- **Purpose**: Describes the one-time switch from the bootloader-provided
  translation state to the kernel-owned higher-half paging hierarchy.
- **Fields**:
  - `old_execution_addr`: Virtual address where the transition begins
  - `higher_half_entry_addr`: Virtual address where execution continues
  - `temporary_alias_pages`: Pages mapped at both the old and higher-half
    virtual addresses during the handoff
  - `final_unmap_ranges`: Low-address ranges removed once the higher-half
    continuation is active
- **Validation Rules**:
  - The active instruction stream must remain mapped across the root switch
  - `final_unmap_ranges` must include the temporary low alias
  - Completion leaves the kernel executing only through the higher-half mapping

## State Transitions

1. `AllocatorReady` -> `KernelTemplateBuilding`
2. `KernelTemplateBuilding` -> `KernelTemplateReady`
3. `KernelTemplateReady` -> `BootstrapTransitionReady`
4. `BootstrapTransitionReady` -> `KernelHigherHalfActive`
5. `KernelTemplateReady` -> `ProcessAddressSpaceBuilding`
6. `ProcessAddressSpaceBuilding` -> `ProcessAddressSpaceReady`
7. `ProcessAddressSpaceReady` -> `ProcessMappingsExpanded`
8. `ProcessAddressSpaceReady` -> `AddressSpaceDestroyed`
9. `ProcessMappingsExpanded` -> `AddressSpaceDestroyed`

Invalid transitions:

- `KernelHigherHalfActive` without a valid `BootstrapTransitionPlan`
- `ProcessAddressSpaceReady` without shared kernel mappings present
- `AddressSpaceDestroyed` while published paging pages remain unreturned to the
  allocator
- Any transition that frees shared kernel paging pages as part of one process's
  teardown
