# Feature Specification: Live VM Handoff

**Feature Branch**: `004-live-vm-handoff`  
**Created**: 2026-05-02  
**Status**: Draft  
**Input**: User description: "Extend the virtual memory work beyond the existing paging-model groundwork and implement the live bootstrap handoff."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Enter Kernel-Owned Virtual Memory (Priority: P1)

As a kernel developer, I want the kernel to activate its own virtual-memory
layout and keep running afterward so execution no longer depends on the
boot-time translation state supplied by firmware or the bootloader.

**Why this priority**: This is the first moment where the kernel truly owns its
address space. Without a successful handoff, the virtual-memory manager remains
only a preparatory model.

**Independent Test**: Boot the kernel in QEMU, perform the virtual-memory
handoff, and verify that the first visible success message appears only after
the new translation state is active.

**Acceptance Scenarios**:

1. **Given** free physical pages are available for paging structures, **When**
   the kernel prepares its runtime translation state, **Then** it creates a
   complete top-level paging hierarchy for the live handoff.
2. **Given** the kernel is still executing on its boot-time mapping, **When**
   the handoff is initiated, **Then** the currently executing code and stack
   remain reachable long enough to load the new root and continue at the target
   higher-half location.
3. **Given** the new translation state becomes active, **When** execution
   resumes, **Then** the kernel prints `hello world` after the handoff without
   relying on a retained low-address fallback.
4. **Given** the handoff completes successfully, **When** the kernel no longer
   needs the temporary low-address alias, **Then** that alias is removed from
   the active translation state.

---

### User Story 2 - Access Physical Memory Through A Direct Map (Priority: P2)

As a kernel developer, I want all allocator-managed physical RAM to be reachable
through a stable kernel virtual-address window so physical pages can be
initialized and manipulated through normal kernel memory access after the
handoff.

**Why this priority**: Direct access to allocator-managed physical memory
simplifies page-table construction, page initialization, and later memory
subsystems that need predictable access to owned physical pages.

**Independent Test**: After the handoff, allocate a physical page, access it
through the direct-map virtual range, zero it, write to it, read the data back,
and release it without destabilizing allocator ownership.

**Acceptance Scenarios**:

1. **Given** a physical page owned by the bitmap allocator, **When** the kernel
   translates it into the direct-map virtual range, **Then** the resulting
   virtual address reaches that same physical page through the active
   translation state.
2. **Given** a newly allocated page table page or kernel-owned data page,
   **When** the kernel initializes it, **Then** initialization occurs through a
   valid kernel virtual address rather than through boot-time assumptions about
   physical accessibility.
3. **Given** the kernel runs a direct-map smoke test after the handoff,
   **When** it writes a known value through the direct-map window, **Then** a
   subsequent read returns the same value.
4. **Given** the kernel finishes with an allocated test page, **When** it
   releases that page, **Then** allocator ownership returns to a consistent free
   state.

---

### User Story 3 - Build Process Address Spaces From The Live Kernel Template (Priority: P3)

As a kernel developer, I want new process address spaces to inherit the live
kernel mappings while keeping their private mappings independent so the kernel
can prepare future process isolation on top of the runtime paging layout it
actually uses.

**Why this priority**: Process address spaces are only meaningful if they are
derived from the kernel mappings that remain active after the live handoff.

**Independent Test**: Create multiple process address spaces after the kernel
handoff and verify that they share the kernel runtime mappings while private
changes in one process do not alter the others.

**Acceptance Scenarios**:

1. **Given** the live kernel translation state is active, **When** the kernel
   creates a new process address space, **Then** that process receives its own
   root paging structure plus the shared kernel higher-half and direct-map
   regions.
2. **Given** two separate process address spaces, **When** one receives a new
   private mapping, **Then** the other process keeps its previous private
   mapping state unchanged.
3. **Given** a process address space is destroyed, **When** its private paging
   structures are reclaimed, **Then** shared kernel mappings remain intact for
   the kernel and other process roots.

### Edge Cases

- What happens when the allocator cannot supply every paging-structure page
  required before the live handoff completes?
- How does the kernel fail safely if the current execution path or stack is not
  fully covered by the temporary low-address bootstrap alias?
- What happens if the direct-map virtual range would overlap the kernel image,
  the higher-half continuation path, or future kernel allocation space?
- How does the system behave if a requested mapping conflicts with an existing
  live kernel or process mapping?
- What happens when a process address space is created or destroyed after only
  part of its private paging hierarchy has ever been allocated?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST create a processor-consumable paging hierarchy for
  the kernel runtime address space using physical pages from the bitmap
  allocator.
- **FR-002**: The system MUST place kernel runtime execution in a designated
  higher-half virtual range.
- **FR-003**: The system MUST define a direct-map virtual range that covers all
  physical RAM managed by the bitmap allocator.
- **FR-004**: The system MUST provide translation helpers between physical
  addresses and the direct-map virtual range for pages inside that range.
- **FR-005**: The system MUST create a temporary low-address bootstrap alias
  covering the currently executing code and stack required to survive the root
  switch.
- **FR-006**: The system MUST load a new kernel-owned root page-table pointer
  and continue execution at a known higher-half continuation point.
- **FR-007**: The system MUST remove the temporary low-address bootstrap alias
  once higher-half execution is established.
- **FR-008**: The first visible kernel success message after the handoff MUST be
  produced after the new translation state becomes active.
- **FR-009**: The system MUST allow the kernel to allocate a physical page,
  access it through the direct-map virtual range, initialize it, verify its
  contents, and release it afterward.
- **FR-010**: The system MUST obtain paging-structure pages through the existing
  bitmap allocator and track ownership explicitly.
- **FR-011**: Failed paging-structure creation, direct-map setup, or handoff
  operations MUST leave allocator ownership and paging metadata in a consistent
  state.
- **FR-012**: The system MUST reject invalid, empty, unaligned, or conflicting
  mapping requests without corrupting existing address-space state.
- **FR-013**: The system MUST support creating a fresh process address space
  from the live kernel runtime mapping template.
- **FR-014**: Each process address space MUST include the shared kernel
  higher-half and direct-map regions while keeping process-private mappings
  isolated from other processes.
- **FR-015**: The system MUST support reclaiming process-private paging
  structures without reclaiming shared kernel mappings.

### Implementation Constraints *(mandatory for this project)*

- **IC-001**: The feature MUST be implemented in Rust and remain compatible with
  the project's `no_std` runtime assumptions unless explicitly approved otherwise.
- **IC-002**: The feature MUST NOT introduce third-party dependencies; if external
  code seems necessary, the spec MUST treat that as blocked pending a constitution
  amendment.
- **IC-003**: Any required assembly MUST be identified explicitly, with a reason
  that explains why Rust alone is insufficient.
- **IC-004**: Any new or expanded `unsafe` boundary MUST identify the invariant it
  relies on and the module that will contain it.
- **IC-005**: The spec MUST define how the feature will be validated, including
  automated checks where practical and manual boot or emulator checks where needed.

### Key Entities *(include if feature involves data)*

- **Kernel Runtime Address Space**: The live translation state the kernel uses
  after loading its own root page-table pointer.
- **Direct-Map Region**: A kernel virtual-address window that gives the kernel a
  stable view of allocator-managed physical RAM through a fixed address
  translation scheme.
- **Bootstrap Alias**: A temporary low-address mapping that keeps the current
  execution path valid across the root switch until higher-half execution takes
  over.
- **Paging Allocation Record**: The ownership record that tracks paging pages
  acquired during one operation so rollback and teardown remain deterministic.
- **Process Address Space**: A root paging structure and its private derived
  mappings layered on top of the shared live kernel mappings.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: The kernel boots in QEMU, loads a kernel-owned translation root,
  and prints `hello world` after the handoff in every successful validation run.
- **SC-002**: The kernel can allocate one physical page after the handoff,
  access it through the direct-map range, zero it, write a known test value,
  read that value back, and free the page successfully in every validation run.
- **SC-003**: Failed handoff preparation attempts do not leave unrecoverable
  paging-page leaks and do not corrupt unrelated allocator state.
- **SC-004**: Two independently created process address spaces can receive
  separate private mappings without unintentionally changing each other's
  private state.
- **SC-005**: Destroying a process address space returns all reclaimable private
  paging structures to the allocator while preserving shared kernel mappings for
  continued operation.

## Assumptions

- The existing bitmap allocator remains the only source of physical-page
  ownership for live paging structures and direct-map smoke-test allocations.
- The kernel already has enough boot-time knowledge to identify the execution
  path, stack, and runtime data that must remain reachable across the handoff.
- The direct-map range only needs to cover physical RAM managed by the
  allocator in this milestone, not firmware-owned or permanently reserved
  regions outside allocator control.
- The serial output path remains available immediately before and after the
  virtual-memory handoff so post-switch proof can be observed.
- Full multitasking, user mode, demand paging, huge pages, and SMP
  synchronization remain out of scope for this milestone.

## Low-Level Impact *(mandatory)*

- **Architecture Impact**: Adds a live x86_64 root-switch handoff, a higher-half
  continuation path, and a kernel direct-map memory window to the boot and
  memory-management flow.
- **Dependency Impact**: No new dependencies are expected.
- **Assembly Impact**: May require narrowly scoped control-register or jump
  transition logic if Rust-only control flow is insufficient for the handoff.
- **Unsafe Impact**: Expected around page-table entry publication, direct-map
  memory access, and root-switch activation; these boundaries must remain small
  and explicitly documented.
- **Validation Plan**: Validate with `cargo test`, `cargo check --target
  x86_64-unknown-uefi`, `make build`, and a QEMU run that proves serial output
  occurs after the root switch and that a direct-map physical-page smoke test
  succeeds.
