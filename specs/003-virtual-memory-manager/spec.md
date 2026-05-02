# Feature Specification: Virtual Memory Manager

**Feature Branch**: `003-virtual-memory-manager`  
**Created**: 2026-05-02  
**Status**: Draft  
**Input**: User description: "I want the kernel to start using virtual memory. For this it will need to be able to create page directory and page tables. Now that we have the working bitmap allocator, it should be possible to use it in order to allocate pages to be used by the virtual memory manager. As this is going to be a multi process os at some point, the memory manager should be able to allocate a process page directory (and all derived page tables) from scratch."

## Clarifications

### Session 2026-05-02

- Q: Should the kernel keep a temporary identity mapping during virtual-memory bring-up, or switch directly to higher-half mappings only? → A: Switch directly to higher-half mappings only, with no identity mapping kept after enable.
- Q: Should process address spaces share the same kernel higher-half mapping, or should each process hold a separate copy of kernel mappings? → A: Every process address space includes the same shared kernel higher-half mapping, plus its own private process mappings.
- Q: Should the virtual memory manager reserve a fixed private buffer for page directories and page tables, or allocate and track those pages through the bitmap allocator? → A: Allocate paging-structure pages on demand through the bitmap allocator and track ownership explicitly; do not rely on a fixed preallocated VMM pool.
- Q: When mapping a virtual range, should the caller provide already-allocated physical backing pages, or should the virtual memory manager allocate them automatically? → A: Support both modes: explicit mapping of caller-supplied physical pages and VMM-owned kernel allocations that allocate physical backing pages and map them into the higher-half kernel space.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Build Kernel Address Translation State (Priority: P1)

As a kernel developer, I want the operating system to construct the page
directory and page tables needed for the kernel address space, including higher
virtual addresses for kernel memory, so the kernel can begin operating with
virtual memory enabled.

**Why this priority**: Virtual memory cannot be adopted until the kernel can
create a complete and valid translation structure for its own execution
environment.

**Independent Test**: Build kernel paging structures from available physical
pages and verify that the resulting address translation state is complete enough
to support enabling virtual memory without losing required kernel access.

**Acceptance Scenarios**:

1. **Given** a physical page allocator with free pages available, **When** the
   kernel requests initialization of virtual memory state, **Then** the system
   creates a kernel page directory and every required page table page.
2. **Given** the kernel's required code, data, stack, and boot-critical memory
   regions, **When** the kernel translation state is prepared, **Then** those
   regions remain addressable after virtual memory is enabled through the
   intended kernel higher-address mappings.
3. **Given** the kernel transitions into virtual memory, **When** the new
   translation state becomes active, **Then** execution continues using the
   higher-address kernel mappings without relying on a retained identity
   mapping.
4. **Given** insufficient free physical pages for the required paging
   structures, **When** kernel translation state is requested, **Then** the
   system reports failure without leaving partially owned paging pages behind.

---

### User Story 2 - Manage Paging Structure Allocation (Priority: P2)

As a kernel developer, I want paging structure pages to be allocated through the
existing physical page allocator so virtual memory bookkeeping uses the same
trusted source of free physical memory as the rest of the kernel.

**Why this priority**: Virtual memory state must consume real pages safely and
consistently, or it will conflict with other kernel memory users.

**Independent Test**: Request creation and growth of paging structures and
verify that every required page comes from tracked free physical memory, is
marked in use while owned by the virtual memory manager, and can be released if
construction fails.

**Acceptance Scenarios**:

1. **Given** a request that requires a new page table page, **When** the
   virtual memory manager expands an address space, **Then** it allocates the
   backing physical page through the physical page allocator.
2. **Given** a paging-structure allocation request that fails partway through,
   **When** the operation aborts, **Then** all pages acquired for that failed
   operation are returned or left in a consistent owned state defined by the
   request outcome.
3. **Given** paging structures that are no longer needed, **When** the
   associated address space is destroyed, **Then** the paging pages become
   reclaimable by the physical page allocator.

---

### User Story 3 - Create Process Address Spaces (Priority: P3)

As a kernel developer, I want to create a process page directory and any derived
page tables from scratch so the kernel can prepare independent process address
spaces for future multi-process execution.

**Why this priority**: Process isolation depends on the ability to create a new
address space without mutating the kernel's own paging state.

**Independent Test**: Request a fresh process address space and verify that it
is built independently of the kernel root paging structure while still
including the shared kernel higher-half mapping and the baseline private
mappings the process needs to execute.

**Acceptance Scenarios**:

1. **Given** a request for a new process address space, **When** the memory
   manager creates it, **Then** it returns a distinct root paging structure for
   that process while preserving the shared kernel higher-half mapping.
2. **Given** a process mapping request that needs additional lower-level paging
   structures, **When** the process address space is updated, **Then** the
   manager creates only the derived page tables required for that process.
3. **Given** two separately created process address spaces, **When** one
   address space receives additional mappings, **Then** the other's private
   paging structures remain unchanged.

### Edge Cases

- What happens when the virtual memory manager needs to create a new page table
  but the physical allocator has no free pages available?
- How does the system fail safely if the higher-address bootstrap mapping cannot
  be activated without retaining an identity mapping?
- How does the system behave if an address space request would require paging
  structures for a range that is already mapped?
- What happens when a process address space is destroyed after only some of its
  derived page tables were ever created?
- How does the system avoid leaking paging pages after a partially completed
  address-space creation attempt fails?
- What happens when the requested mapping range is empty or not aligned to whole
  pages?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST provide a virtual memory manager capable of
  creating the paging structures required for the kernel to begin using virtual
  memory.
- **FR-002**: The virtual memory manager MUST be able to create a root page
  directory for an address space and create derived page tables on demand.
- **FR-003**: The virtual memory manager MUST obtain physical pages for paging
  structures through the existing physical page allocator.
- **FR-003a**: The virtual memory manager MUST allocate paging-structure pages
  on demand and track their ownership explicitly instead of depending on a fixed
  preallocated private buffer.
- **FR-004**: The system MUST prepare kernel address-space mappings that place
  kernel memory in its intended higher virtual address range.
- **FR-005**: The system MUST preserve the kernel memory regions required for
  continued execution when preparing the kernel address space.
- **FR-006**: The virtual memory manager MUST report failure if it cannot obtain
  all paging pages required for a requested operation.
- **FR-007**: Failed paging-structure creation or update operations MUST leave
  allocator ownership and paging metadata in a consistent state.
- **FR-008**: The system MUST support creating a fresh process address space
  with its own root page directory.
- **FR-009**: The system MUST support adding derived page tables to a process
  address space only as needed for that process's mappings.
- **FR-010**: The system MUST keep private paging structures for one process
  address space isolated from private paging structures belonging to another
  process address space.
- **FR-011**: The system MUST support reclaiming paging-structure pages when an
  address space is torn down.
- **FR-012**: The system MUST reject empty, invalid, or conflicting mapping or
  paging requests without corrupting existing address-space state.
- **FR-012a**: The system MUST support mapping a caller-supplied physical page
  range into an address space without reallocating that backing range.
- **FR-012b**: The system MUST support a kernel-owned allocation path that
  obtains physical pages through the bitmap allocator and maps them into the
  kernel higher-half virtual space.
- **FR-013**: The system MUST define a valid bootstrap mapping strategy that
  allows the kernel to transition safely into its higher-address mapping state.
- **FR-014**: The bootstrap transition MUST activate the higher-address kernel
  mapping without retaining an identity mapping after virtual memory is enabled.
- **FR-015**: Each process address space MUST include the shared kernel
  higher-half mapping while keeping its process-private mappings independent
  from other processes.

### Implementation Constraints *(mandatory for this project)*

- **IC-001**: The feature MUST be implemented in Rust and remain compatible with
  the project's `no_std` runtime assumptions unless explicitly approved otherwise.
- **IC-002**: The feature MUST NOT introduce third-party dependencies; if
  external code seems necessary, the spec MUST treat that as blocked pending a
  constitution amendment.
- **IC-003**: Any required assembly MUST be identified explicitly, with a reason
  that explains why Rust alone is insufficient.
- **IC-004**: Any new or expanded `unsafe` boundary MUST identify the invariant
  it relies on and the module that will contain it.
- **IC-005**: The spec MUST define how the feature will be validated, including
  automated checks where practical and manual boot or emulator checks where
  needed.

### Key Entities *(include if feature involves data)*

- **Address Space**: The complete translation state used by either the kernel or
  one process, rooted at a top-level paging structure.
- **Kernel Higher-Half Mapping**: The virtual address region where kernel code,
  data, and required runtime memory remain accessible after the transition away
  from purely physical or identity-based addressing.
- **Page Directory**: The root paging structure that anchors an address space
  and points to lower-level page tables.
- **Page Table Page**: A physical page dedicated to storing address translation
  entries for part of an address space.
- **Mapping Request**: A request to ensure that a virtual address range is
  represented in an address space with the required paging structures, either by
  mapping caller-supplied physical backing pages or by publishing VMM-owned
  kernel allocations.
- **Paging Allocation Record**: The ownership information needed to track which
  physical pages are reserved for a given address space's paging structures.
- **Kernel Virtual Allocation**: A VMM-owned higher-half kernel mapping whose
  physical backing pages and required paging structures are both obtained through
  the bitmap allocator.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: The kernel can construct a complete initial paging hierarchy for
  its required execution regions, including the target higher-address kernel
  mappings, in one operation without manual page-table setup steps.
- **SC-002**: A request to create a new process address space succeeds whenever
  enough free physical pages exist for the required root and derived paging
  structures, and fails cleanly otherwise.
- **SC-003**: Failed address-space creation or expansion attempts leave no
  unrecoverable paging-page leaks and do not alter unrelated existing address
  spaces.
- **SC-004**: Destroying an address space returns all of its paging-structure
  pages to the pool of reclaimable physical memory.
- **SC-005**: Two independently created process address spaces can be modified
  separately without unintentionally changing each other's private mappings.
- **SC-006**: The kernel can enter virtual memory and continue execution through
  the higher-address kernel mapping without depending on an identity-mapped
  fallback.
- **SC-007**: Newly created process address spaces expose the shared kernel
  higher-half mapping consistently while preserving independent private process
  mappings.

## Assumptions

- The existing physical page allocator is available before virtual memory setup
- The existing physical page allocator remains the only source of physical-page
  ownership for both paging structures and VMM-owned kernel virtual allocations.
- The kernel already knows which physical memory regions must remain accessible
  during and immediately after the transition to virtual memory.
- The intended kernel virtual-address layout includes a higher-address region
  for kernel execution in this feature's target state.
- The transition into virtual memory is expected to land directly in the
  higher-address kernel mapping instead of preserving an identity-mapped
  fallback.
- This milestone covers creation, ownership, and teardown of paging structures,
  not full process scheduling, context switching, demand paging, or swapping.
- The first version targets the paging model already implied by the current
  kernel architecture and page size assumptions.
- New process address spaces may reuse whatever shared kernel mappings are
  required for the kernel to service those processes.

## Low-Level Impact *(mandatory)*

- **Architecture Impact**: Adds architecture-sensitive paging structures,
  address-space construction, higher-address kernel mappings, and
  virtual-memory bootstrap behavior to the kernel memory subsystem.
- **Dependency Impact**: No new dependencies are expected.
- **Assembly Impact**: None expected by default; any required control-register
  transition or architecture bootstrap change must be called out explicitly
  during planning.
- **Unsafe Impact**: Expected around paging-entry writes, physical page
  ownership handoff, and activation of the constructed translation state; these
  boundaries must be explicitly contained.
- **Validation Plan**: Validate address-space construction and teardown with
  host-side Rust tests, run `cargo check --target x86_64-unknown-uefi`, run
  `cargo test`, run `make build`, and confirm the boot path in `./run.sh`
  continues through the virtual-memory bring-up flow.
