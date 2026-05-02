# Data Model: Live VM Handoff

## KernelRuntimeAddressSpace

- **Purpose**: Represents the live translation state the kernel activates after
  the CR3 handoff.
- **Fields**:
  - `root_table_phys_addr`: Physical address of the active top-level paging
    structure
  - `higher_half_base`: Virtual base of the kernel image and continuation range
  - `direct_map_base`: Virtual base of the physical-memory direct-map window
  - `direct_map_limit`: Exclusive upper bound of the direct-map window
  - `shared_kernel_regions`: Mapped higher-half regions that every process must
    inherit
  - `owned_table_pages`: Paging-structure pages owned by the live kernel root
- **Validation Rules**:
  - `root_table_phys_addr` must be page-aligned and allocator-owned
  - `direct_map_base..direct_map_limit` must not overlap kernel image or kernel
    allocation ranges
  - Shared kernel regions must remain mapped after the low bootstrap alias is
    removed

## DirectMapRegion

- **Purpose**: Describes the fixed-offset kernel virtual-address window used to
  reach allocator-managed physical RAM.
- **Fields**:
  - `virt_base`: Inclusive virtual start of the window
  - `phys_base`: Physical address represented by `virt_base`
  - `length_bytes`: Covered physical span
  - `translation_offset`: Fixed offset between physical and virtual addresses
- **Validation Rules**:
  - Every allocator-managed physical page must either fall inside the covered
    span or be explicitly out of scope
  - Translation must be reversible for all covered pages
  - The window must not overlap higher-half kernel image, bootstrap alias, or
    process-private address ranges

## BootstrapAlias

- **Purpose**: Represents the temporary low-address mapping used to keep the
  active execution path valid across the CR3 switch.
- **Fields**:
  - `low_virt_start`: Low virtual start address of the alias
  - `higher_half_target`: Corresponding higher-half continuation address
  - `page_count`: Number of 4 KiB pages retained during the switch
  - `covered_roles`: Execution elements covered by the alias, such as code,
    stack, and immediate runtime data
- **Validation Rules**:
  - The active instruction stream and stack must both fall inside the alias
  - The alias must be removable after higher-half execution begins
  - The alias must not remain active after the handoff success path completes

## DirectMapSmokeAllocation

- **Purpose**: Tracks the allocator-backed page used to prove that the direct
  map works after the handoff.
- **Fields**:
  - `backing_page`: Physical page allocated from the bitmap allocator
  - `direct_map_virt_addr`: Virtual address used to touch the page
  - `test_pattern`: Value written and read back during validation
  - `released`: Whether the page has been returned to the allocator
- **Validation Rules**:
  - `direct_map_virt_addr` must translate back to `backing_page`
  - Reads after writes must return the written pattern before release
  - `released` must be true by the end of a successful smoke test

## ProcessAddressSpace

- **Purpose**: A process root and its private mappings layered on top of the
  shared live kernel runtime mappings.
- **Fields**:
  - `root_table_phys_addr`: Physical address of the process root
  - `shared_runtime_template_ref`: Reference to the active kernel higher-half
    and direct-map template
  - `private_mapping_floor`: Lower bound of process-private mappings
  - `private_mapping_ceiling`: Upper bound of process-private mappings
  - `owned_private_table_pages`: Paging-structure pages reclaimable when the
    process address space is destroyed
- **Validation Rules**:
  - Shared runtime template pages must never be reclaimed by process teardown
  - Private mappings must not overlap the shared kernel higher-half or direct
    map
  - Adding private mappings in one process must not alter another process root

## State Transitions

1. `AllocatorReady` -> `RuntimeRootBuilding`
2. `RuntimeRootBuilding` -> `BootstrapAliasReady`
3. `BootstrapAliasReady` -> `RootSwitchReady`
4. `RootSwitchReady` -> `KernelRuntimeActive`
5. `KernelRuntimeActive` -> `DirectMapSmokeValidated`
6. `KernelRuntimeActive` -> `ProcessRootBuilding`
7. `ProcessRootBuilding` -> `ProcessRootReady`
8. `ProcessRootReady` -> `ProcessRootExpanded`
9. `ProcessRootReady` -> `ProcessRootDestroyed`
10. `ProcessRootExpanded` -> `ProcessRootDestroyed`

Invalid transitions:

- `KernelRuntimeActive` without a valid bootstrap alias and continuation path
- `DirectMapSmokeValidated` without a successful write/read verification through
  the direct map
- `ProcessRootReady` without inheriting the shared live kernel higher-half and
  direct-map template
- `ProcessRootDestroyed` if shared kernel pages are reclaimed as part of the
  destroy path
