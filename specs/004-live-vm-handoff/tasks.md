# Tasks: Live VM Handoff

**Input**: Design documents from `/specs/004-live-vm-handoff/`
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

**Purpose**: Establish the live handoff feature boundaries, shared constants, and validation scaffolding for the new spec

- [X] T001 Define the live higher-half, direct-map, and temporary low-alias range constants in `src/memory/paging/table.rs`
- [X] T002 [P] Export the new runtime paging constants and translation helpers from `src/memory/paging/mod.rs` and `src/memory/mod.rs`
- [X] T003 [P] Update the QEMU transcript expectations for post-switch proof in `tests/e2e_boot_serial.py` and `tests/host/run_contract.rs`
- [X] T004 Record the live handoff validation commands and expected transcript markers in `specs/004-live-vm-handoff/quickstart.md`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Replace the software-only paging model with hardware-consumable table publication and direct-map-backed write access

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [ ] T005 Replace placeholder table storage with direct-map-backed page-table page access primitives in `src/memory/paging/table.rs`
- [X] T006 [P] Update paging-page allocation, zeroing, and rollback publication paths in `src/memory/paging/address_space.rs` and `src/memory/paging/mapper.rs`
- [X] T007 [P] Add host tests for direct-map translation helpers, paging-page initialization, and rollback behavior in `tests/host/paging.rs`
- [X] T008 Define the bootstrap alias coverage and higher-half continuation inputs needed for the root switch in `src/arch/x86_64/paging.rs` and `src/main.rs`
- [X] T009 Document the contained `unsafe` invariants for direct-map access, page-table entry writes, and CR3 activation in `specs/004-live-vm-handoff/contracts/runtime-paging-interface.md` and `specs/004-live-vm-handoff/plan.md`

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Enter Kernel-Owned Virtual Memory (Priority: P1) 🎯 MVP

**Goal**: Build the live kernel root, switch CR3 safely, continue in the higher half, and remove the temporary low bootstrap alias

**Independent Test**: Boot in QEMU, perform the root switch, and verify that `hello world` is printed only after the higher-half continuation path is active

### Validation for User Story 1 ⚠️

- [X] T010 [P] [US1] Add host tests for bootstrap alias construction, continuation reachability, and low-alias removal rules in `tests/host/paging.rs`
- [X] T011 [US1] Update the boot transcript validation for post-switch `hello world` proof in `tests/e2e_boot_serial.py` and `specs/004-live-vm-handoff/quickstart.md`

### Implementation for User Story 1

- [X] T012 [P] [US1] Build the live kernel runtime root with higher-half image mappings and temporary low bootstrap aliases in `src/memory/paging/address_space.rs`
- [X] T013 [P] [US1] Publish real allocator-backed page-table entries for the runtime root in `src/memory/paging/mapper.rs`
- [ ] T014 [US1] Implement CR3 loading and the higher-half continuation handoff path in `src/arch/x86_64/paging.rs`
- [X] T015 [US1] Replace the diagnostic-only boot path with the live root switch and post-switch `hello world` output in `src/main.rs`
- [ ] T016 [US1] Remove the temporary low bootstrap alias after successful continuation in `src/memory/paging/address_space.rs`, `src/arch/x86_64/paging.rs`, and `src/main.rs`

**Checkpoint**: At this point, User Story 1 should be fully functional and testable independently

---

## Phase 4: User Story 2 - Access Physical Memory Through A Direct Map (Priority: P2)

**Goal**: Expose allocator-managed physical RAM through a stable virtual direct-map range and prove it works after the CR3 handoff

**Independent Test**: After the handoff, allocate a physical page, access it through the direct map, zero it, write/read a known value, and release it cleanly

### Validation for User Story 2 ⚠️

- [X] T017 [P] [US2] Add host tests for direct-map physical/virtual round-trip translation and direct-map-backed page initialization in `tests/host/paging.rs`
- [X] T018 [US2] Extend boot validation expectations for the post-switch direct-map smoke test in `tests/e2e_boot_serial.py` and `specs/004-live-vm-handoff/quickstart.md`

### Implementation for User Story 2

- [X] T019 [P] [US2] Implement the direct-map address translation helpers and range validation in `src/memory/paging/table.rs` and `src/memory/paging/mod.rs`
- [ ] T020 [P] [US2] Update allocator-backed paging-page and kernel-page initialization to use the active direct-map window in `src/memory/paging/address_space.rs` and `src/memory/paging/mapper.rs`
- [X] T021 [US2] Add the post-switch direct-map smoke test flow in `src/main.rs`
- [ ] T022 [US2] Preserve rollback and reclaim correctness for direct-map-backed paging/data pages in `src/memory/paging/address_space.rs` and `src/memory/paging/mapper.rs`
- [X] T023 [US2] Record the finalized direct-map behavior and failure contract in `specs/004-live-vm-handoff/contracts/runtime-paging-interface.md`

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently

---

## Phase 5: User Story 3 - Build Process Address Spaces From The Live Kernel Template (Priority: P3)

**Goal**: Create process roots from the live kernel runtime template so they inherit the active higher-half and direct-map regions while keeping private mappings isolated

**Independent Test**: Create multiple process address spaces from the live kernel template and verify that shared kernel mappings stay intact while private changes remain isolated

### Validation for User Story 3 ⚠️

- [X] T024 [P] [US3] Add host tests for process-root creation from the live kernel template, shared direct-map inheritance, and destroy-time reclamation in `tests/host/paging.rs`
- [ ] T025 [US3] Extend the reproducible validation notes for process address-space scenarios built from the live kernel template in `specs/004-live-vm-handoff/quickstart.md`

### Implementation for User Story 3

- [X] T026 [P] [US3] Update process-root creation to clone the live kernel higher-half and direct-map template in `src/memory/paging/address_space.rs`
- [ ] T027 [P] [US3] Update process-private mapping growth and conflict handling for the live page-table representation in `src/memory/paging/mapper.rs`
- [X] T028 [US3] Verify destroy/reclaim behavior preserves shared runtime mappings while releasing private pages in `src/memory/paging/address_space.rs`
- [X] T029 [US3] Export revised process address-space creation, translation, and destroy helpers in `src/memory/paging/mod.rs` and `src/memory/mod.rs`

**Checkpoint**: All user stories should now be independently functional

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Final consistency, safety cleanup, and end-to-end proof across all stories

- [X] T030 [P] Finalize transcript and host contract checks for post-switch serial output and direct-map smoke behavior in `tests/e2e_boot_serial.py` and `tests/host/run_contract.rs`
- [X] T031 [P] Reduce or document any remaining `unsafe` and architecture touchpoints in `src/memory/paging/table.rs`, `src/memory/paging/address_space.rs`, `src/memory/paging/mapper.rs`, and `src/arch/x86_64/paging.rs`
- [X] T032 Update the implementation narrative and file references in `specs/004-live-vm-handoff/plan.md` and `specs/004-live-vm-handoff/contracts/runtime-paging-interface.md`
- [ ] T033 Run the full validation flow and record verified results in `specs/004-live-vm-handoff/quickstart.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Foundational completion
- **User Story 2 (Phase 4)**: Depends on Foundational completion and on the live CR3 handoff from US1
- **User Story 3 (Phase 5)**: Depends on the live kernel template established by US1 and the direct-map behavior finalized in US2
- **Polish (Phase 6)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - MVP path
- **User Story 2 (P2)**: Builds on the live kernel runtime state from US1
- **User Story 3 (P3)**: Builds on the live kernel higher-half/direct-map template from US1 and US2

### Within Each User Story

- Host-side validation tasks should land before or alongside the implementation they verify
- Direct-map-backed table access must exist before the kernel publishes live page-table entries
- Architecture activation code must remain isolated from generic paging logic
- The CR3 switch should only happen after the higher-half continuation path and direct-map smoke path are both mapped

### Parallel Opportunities

- `T002`, `T003`, and `T004` can run in parallel once the shared range constants are defined
- `T006`, `T007`, and `T009` can run in parallel within the Foundational phase
- `T012` and `T013` can run in parallel within US1 before boot integration
- `T019` and `T020` can run in parallel within US2 before the smoke-test integration step
- `T026` and `T027` can run in parallel within US3 before destroy-path integration

---

## Parallel Example: User Story 1

```bash
# Launch validation and live paging-construction work for User Story 1 together:
Task: "Add host tests for bootstrap alias construction, continuation reachability, and low-alias removal rules in tests/host/paging.rs"
Task: "Build the live kernel runtime root with higher-half image mappings and temporary low bootstrap aliases in src/memory/paging/address_space.rs"
Task: "Publish real allocator-backed page-table entries for the runtime root in src/memory/paging/mapper.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: Run host paging tests plus the QEMU validation flow from `quickstart.md`
5. Demo `hello world` printing after the CR3 switch

### Incremental Delivery

1. Complete Setup + Foundational to make the paging model hardware-consumable
2. Add User Story 1 and validate the live higher-half handoff
3. Add User Story 2 and validate direct-map-backed physical memory access after the switch
4. Add User Story 3 and validate process roots derived from the live kernel template
5. Finish with polish and full regression validation

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 live handoff
   - Developer B: User Story 2 direct-map access
   - Developer C: User Story 3 process-root inheritance after US1/US2 interfaces stabilize

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] labels map tasks to specific user stories for traceability
- Each user story remains independently testable through host-side coverage plus the documented boot validation flow
- The task list assumes no new third-party dependencies and no new standalone assembly files
- Exact public function names may still evolve during implementation, but the behavior and file ownership described here should remain stable
