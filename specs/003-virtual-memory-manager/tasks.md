# Tasks: Virtual Memory Manager

**Input**: Design documents from `/specs/003-virtual-memory-manager/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Include validation tasks for every feature. Automated Rust tests are strongly preferred
when practical, and boot or emulator validation tasks are mandatory for kernel-critical changes.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Single project**: `src/`, `tests/` at repository root
- Paths below assume the repository layout captured in `plan.md`

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create the VM module scaffolding and documentation entry points used by later phases

- [ ] T001 Create the paging module scaffolding in `src/memory/paging/mod.rs`, `src/memory/paging/table.rs`, `src/memory/paging/mapper.rs` and `src/memory/paging/address_space.rs`
- [ ] T002 Create the x86_64 paging activation module and export it in `src/arch/x86_64/paging.rs` and `src/arch/x86_64/mod.rs`
- [ ] T003 [P] Wire the new memory paging modules in `src/memory/mod.rs` and `src/lib.rs`
- [ ] T004 [P] Add the VM host test entry point and registration in `tests/host/paging.rs` and `tests/host.rs`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Define shared paging types, ownership tracking, and safe low-level boundaries before any story-specific work

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [ ] T005 Define shared paging constants, index helpers, and entry flag types in `src/memory/paging/table.rs`
- [ ] T006 [P] Define `AddressSpace`, `PagingAllocationRecord`, and kernel/shared ownership types in `src/memory/paging/address_space.rs`
- [ ] T007 [P] Implement table-page allocation and rollback helpers on top of `BitmapAllocator` in `src/memory/paging/mapper.rs`
- [ ] T008 Contain raw page-table entry read/write and zero-initialization invariants in `src/memory/paging/table.rs`
- [ ] T009 [P] Add foundational host tests for paging indices, entry flags, and allocator-backed rollback helpers in `tests/host/paging.rs`

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Build Kernel Address Translation State (Priority: P1) 🎯 MVP

**Goal**: Construct and activate the kernel's higher-half paging hierarchy without retaining an identity mapping

**Independent Test**: Build the kernel paging hierarchy from the physical allocator, verify required kernel regions are represented, and confirm boot continues through the higher-half mapping after activation

### Validation for User Story 1 ⚠️

- [ ] T010 [P] [US1] Add host tests for kernel template construction, transition alias removal, and bootstrap mapping failure cases in `tests/host/paging.rs`
- [ ] T011 [US1] Record the kernel bring-up validation flow for higher-half activation in `specs/003-virtual-memory-manager/quickstart.md`

### Implementation for User Story 1

- [ ] T012 [P] [US1] Implement kernel higher-half layout and shared mapping template construction in `src/memory/paging/address_space.rs`
- [ ] T013 [P] [US1] Implement page-table walking and caller-supplied physical range mapping for kernel bootstrap mappings in `src/memory/paging/mapper.rs`
- [ ] T014 [US1] Implement x86_64 page-table root loading and higher-half transition helpers in `src/arch/x86_64/paging.rs`
- [ ] T015 [US1] Integrate kernel paging bootstrap with UEFI boot memory data and the bitmap allocator in `src/main.rs`
- [ ] T016 [US1] Update module exports and activation call sites in `src/arch/x86_64/mod.rs` and `src/lib.rs`

**Checkpoint**: At this point, User Story 1 should be fully functional and testable independently

---

## Phase 4: User Story 2 - Manage Paging Structure Allocation (Priority: P2)

**Goal**: Allocate page-table pages on demand through the bitmap allocator, support kernel-owned virtual allocations, and reclaim paging ownership safely on failure or teardown

**Independent Test**: Request mapping growth and kernel-owned allocations, verify that all paging-structure pages come from the bitmap allocator, and confirm partial failures roll back without leaks

### Validation for User Story 2 ⚠️

- [ ] T017 [P] [US2] Add host tests for allocator-backed table-page ownership, kernel virtual allocations, and rollback-on-failure behavior in `tests/host/paging.rs`
- [ ] T018 [US2] Extend validation guidance for kernel-owned allocation and reclaim behavior in `specs/003-virtual-memory-manager/quickstart.md`

### Implementation for User Story 2

- [ ] T019 [P] [US2] Implement allocator-backed paging-structure ownership tracking and release paths in `src/memory/paging/address_space.rs`
- [ ] T020 [P] [US2] Implement the caller-supplied `map_range` path and the kernel-owned allocate-and-map path in `src/memory/paging/mapper.rs`
- [ ] T021 [US2] Export the virtual memory manager surface and kernel allocation helpers in `src/memory/paging/mod.rs` and `src/memory/mod.rs`
- [ ] T022 [US2] Integrate kernel-owned virtual allocation diagnostics or smoke coverage in `src/main.rs`
- [ ] T023 [US2] Document the finalized mapping and allocation API expectations in `specs/003-virtual-memory-manager/contracts/paging-interface.md`

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently

---

## Phase 5: User Story 3 - Create Process Address Spaces (Priority: P3)

**Goal**: Create fresh process address spaces with a distinct root page table, shared kernel mappings, independent private mappings, and safe teardown

**Independent Test**: Create multiple process address spaces, mutate private mappings in only one of them, and verify that shared kernel mappings remain intact while private tables stay isolated and reclaimable

### Validation for User Story 3 ⚠️

- [ ] T024 [P] [US3] Add host tests for process address-space creation, private mapping isolation, and destroy-time reclamation in `tests/host/paging.rs`
- [ ] T025 [US3] Extend the reproducible validation notes for process address-space scenarios in `specs/003-virtual-memory-manager/quickstart.md`

### Implementation for User Story 3

- [ ] T026 [P] [US3] Implement fresh process root creation and shared kernel mapping installation in `src/memory/paging/address_space.rs`
- [ ] T027 [P] [US3] Implement process-private mapping expansion and conflict handling in `src/memory/paging/mapper.rs`
- [ ] T028 [US3] Implement address-space destroy and reclaim behavior for private paging pages in `src/memory/paging/address_space.rs`
- [ ] T029 [US3] Export process address-space creation and teardown APIs in `src/memory/paging/mod.rs` and `src/memory/mod.rs`

**Checkpoint**: All user stories should now be independently functional

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Final consistency, documentation, and end-to-end validation across all user stories

- [ ] T030 [P] Add end-to-end boot regression coverage notes or harness updates in `tests/e2e_boot_serial.py` and `tests/host/run_contract.rs`
- [ ] T031 [P] Reduce or document any remaining unsafe/architecture touchpoints in `src/memory/paging/table.rs`, `src/memory/paging/mapper.rs` and `src/arch/x86_64/paging.rs`
- [ ] T032 Update the implementation narrative and file references in `specs/003-virtual-memory-manager/plan.md`
- [ ] T033 Run the full validation flow and record the verified results in `specs/003-virtual-memory-manager/quickstart.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Foundational completion
- **User Story 2 (Phase 4)**: Depends on Foundational completion and benefits from the kernel paging primitives delivered in US1
- **User Story 3 (Phase 5)**: Depends on Foundational completion and on the shared kernel mapping machinery from US1 and ownership/reclaim behavior from US2
- **Polish (Phase 6)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - MVP path
- **User Story 2 (P2)**: Can start after Foundational, but should be finished after US1 establishes the kernel paging bootstrap path
- **User Story 3 (P3)**: Builds on US1 shared kernel mappings and US2 ownership/reclaim helpers

### Within Each User Story

- Host-side validation tasks should land before or alongside the implementation they verify
- Shared low-level table and ownership primitives must exist before higher-level integration
- Architecture activation code must remain isolated from generic paging logic
- Boot integration should happen after paging construction logic is testable on the host

### Parallel Opportunities

- `T003` and `T004` can run in parallel after the new module files are created
- `T006`, `T007`, and `T009` can run in parallel in the Foundational phase
- `T012` and `T013` can run in parallel within US1 before boot integration
- `T019` and `T020` can run in parallel within US2 before API export/integration work
- `T026` and `T027` can run in parallel within US3 before destroy-path integration

---

## Parallel Example: User Story 1

```bash
# Launch validation and pure paging construction work for User Story 1 together:
Task: "Add host tests for kernel template construction, transition alias removal, and bootstrap mapping failure cases in tests/host/paging.rs"
Task: "Implement kernel higher-half layout and shared mapping template construction in src/memory/paging/address_space.rs"
Task: "Implement page-table walking and caller-supplied physical range mapping for kernel bootstrap mappings in src/memory/paging/mapper.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: Run host paging tests plus the boot validation flow from `quickstart.md`
5. Demo the kernel booting through the higher-half mapping

### Incremental Delivery

1. Complete Setup + Foundational to establish the paging substrate
2. Add User Story 1 and validate kernel bootstrap into higher-half execution
3. Add User Story 2 and validate allocator-backed table-page ownership plus kernel-owned virtual allocations
4. Add User Story 3 and validate independent process address spaces and teardown
5. Finish with polish and full regression validation

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: Kernel bootstrap path in US1
   - Developer B: Allocator-backed ownership and kernel allocation path in US2
   - Developer C: Process address-space creation in US3 after US1/US2 interfaces stabilize

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] labels map tasks to specific user stories for traceability
- Each user story remains independently testable through host-side coverage plus the documented validation flow
- The task list assumes no new third-party dependencies and no new standalone assembly files
- Exact public function names may still evolve during implementation, but the behavior and file ownership described here should remain stable
