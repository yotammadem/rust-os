# Feature Specification: Bitmap Allocator

**Feature Branch**: `002-bitmap-allocator`  
**Created**: 2026-05-01  
**Status**: Draft  
**Input**: User description: "Create basic bitmap memory allocator. The allocator will get the available memory from grub and will manage a bitmap of used pages. It will expose a way to allocate a new page (or number of pages) and mark them as used. It will also allow freeing a used page that is no longer needed"

## Clarifications

### Session 2026-05-01

- Q: Should multi-page allocation return contiguous physical pages or any free pages? → A: Multi-page allocation must return one contiguous physical page range.
- Q: How should invalid free requests behave? → A: Invalid free returns failure and leaves allocator state unchanged.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Initialize Allocator State (Priority: P1)

As a kernel developer, I want the operating system to build a page-tracking view
of available memory during boot so later kernel code can reason about which pages
are free and which are reserved.

**Why this priority**: Allocation cannot be trusted until the allocator has a
correct initial view of available and unavailable memory.

**Independent Test**: Start the allocator from boot-provided memory information
and verify that usable pages are tracked as available while reserved or occupied
pages are not offered for allocation.

**Acceptance Scenarios**:

1. **Given** boot-provided memory information, **When** the allocator is
   initialized, **Then** it records which page ranges are usable and which are
   unavailable.
2. **Given** pages already occupied by the kernel, boot structures, or allocator
   metadata, **When** initialization completes, **Then** those pages are marked as
   unavailable for future allocation.

---

### User Story 2 - Allocate Pages (Priority: P2)

As a kernel developer, I want to request one or more free pages from the
allocator so kernel subsystems can reserve physical memory safely.

**Why this priority**: The allocator is only useful if other kernel code can
obtain free pages in a predictable way.

**Independent Test**: Request pages from an initialized allocator and verify that
the allocator returns free page ranges, marks them as used, and refuses requests
that exceed the currently available space.

**Acceptance Scenarios**:

1. **Given** an initialized allocator with free pages, **When** the caller
   requests one page, **Then** the allocator returns one usable page and marks it
   as used.
2. **Given** an initialized allocator with a large enough contiguous free range,
   **When** the caller requests multiple pages, **Then** the allocator returns one
   contiguous physical page range and marks every page in that range as used.
3. **Given** an initialized allocator without enough free pages to satisfy the
   request, **When** allocation is requested, **Then** the allocator reports
   failure without corrupting its page-tracking state.

---

### User Story 3 - Free Pages (Priority: P3)

As a kernel developer, I want to free pages that are no longer needed so that
memory can be reused safely by later allocations.

**Why this priority**: Reuse is necessary for long-running kernel work and for
preventing avoidable memory exhaustion.

**Independent Test**: Free a page range that was previously allocated and verify
that the allocator marks it available again without releasing pages that were
never valid for freeing.

**Acceptance Scenarios**:

1. **Given** a page that was allocated successfully, **When** the caller frees
   it, **Then** the allocator marks that page as available again.
2. **Given** a multi-page allocation that was allocated successfully, **When**
   the caller frees the full range, **Then** each page in that range becomes
   available again.
3. **Given** a page range that is not a currently owned allocation, **When**
   the caller attempts to free it, **Then** the allocator returns failure and
   leaves allocator state unchanged.

### Edge Cases

- What happens if the boot memory information contains small unusable gaps between
  otherwise usable ranges?
- How does the allocator behave when no free pages remain?
- What happens if a free request overlaps pages that were never allocated by the
  allocator?
- How does the allocator handle requests for zero pages?
- Invalid free requests must fail without changing allocator bookkeeping.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST initialize a page allocator from boot-provided
  memory availability information.
- **FR-002**: The allocator MUST track page usage with a bitmap representation.
- **FR-003**: The allocator MUST distinguish between usable pages and pages that
  are reserved, unavailable, or already occupied during initialization.
- **FR-004**: The allocator MUST expose a way to allocate one free page.
- **FR-005**: The allocator MUST expose a way to allocate multiple pages in a
  single request as one contiguous physical page range.
- **FR-006**: Successful allocation MUST mark the returned pages as used before
  control returns to the caller.
- **FR-007**: If a request cannot be satisfied, the allocator MUST report failure
  without partially reserving unrelated pages.
- **FR-008**: The allocator MUST expose a way to free a page or page range that
  was previously allocated successfully.
- **FR-009**: Successful free operations MUST mark the released pages as
  available again.
- **FR-010**: The allocator MUST prevent or safely reject invalid free operations
  by returning failure and leaving allocator state unchanged.
- **FR-011**: The allocator MUST avoid offering pages already occupied by kernel,
  boot, or allocator-owned metadata.

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
- **IC-005**: The spec MUST define how the allocator will be validated, including
  host-side checks where practical and boot-integrated checks where needed.

### Key Entities *(include if feature involves data)*

- **Memory Region**: A boot-reported address range with availability metadata that
  indicates whether its pages may be managed by the allocator.
- **Page Frame**: A fixed-size physical memory page tracked as free, used, or
  unavailable.
- **Bitmap Allocator State**: The allocator-owned tracking structure that maps page
  indices to used or free bits.
- **Allocation Request**: A request for one or more pages together with the result
  returned to the caller.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: An initialized allocator can classify boot-reported page ranges into
  available and unavailable sets without manual post-processing.
- **SC-002**: A single-page request succeeds whenever at least one free page is
  available and fails cleanly when no such page exists.
- **SC-003**: A multi-page request succeeds only when enough free pages are
  available in one contiguous range for the requested allocation and otherwise
  leaves allocator state unchanged.
- **SC-004**: A previously allocated page range can be freed and reused by a later
  allocation without corrupting allocator bookkeeping.
- **SC-005**: An invalid free request fails deterministically and does not change
  the allocator's tracked free or used page state.

## Assumptions

- The existing boot flow already provides enough memory information for the kernel
  to identify usable and reserved physical ranges.
- The allocator is responsible only for page-granular physical memory management
  in this milestone, not higher-level heap allocation.
- The allocator may assume a fixed page size chosen by the kernel for the current
  architecture baseline.
- Concurrency, locking, and multi-core allocation behavior are out of scope for
  this first allocator milestone.

## Low-Level Impact *(mandatory)*

- **Architecture Impact**: Adds the first physical page allocator and ties kernel
  memory management to boot-provided memory range information.
- **Dependency Impact**: No new dependencies are expected.
- **Assembly Impact**: None expected.
- **Unsafe Impact**: Expected around physical memory bookkeeping and allocator
  metadata placement; these boundaries must be explicitly contained.
- **Validation Plan**: Validate allocator bookkeeping with host-side Rust tests,
  then confirm boot-integrated initialization against the existing GRUB/UEFI boot
  path.
