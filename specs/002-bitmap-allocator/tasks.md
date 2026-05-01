# Tasks: Bitmap Allocator

**Input**: Design documents from `/specs/002-bitmap-allocator/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Include validation tasks for every feature. Automated Rust tests are strongly preferred
when practical, and boot or emulator validation tasks are mandatory for kernel-critical changes.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- Single project: `src/`, `tests/` at repository root

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create the module and test scaffolding the allocator feature will use

- [x] T001 Create the allocator module skeleton in `src/memory/mod.rs`, `src/memory/map.rs`, and `src/memory/bitmap.rs`
- [x] T002 [P] Add host test scaffolding for allocator coverage in `tests/host.rs`, `tests/host/boot_memory_map.rs`, and `tests/host/allocator.rs`
- [x] T003 [P] Rename the boot protocol module from `src/boot/multiboot.rs` to `src/boot/uefi.rs` and update `src/boot/mod.rs`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Establish shared boot-memory and page-model infrastructure required by all user stories

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T004 Define shared page-size, physical-address, and page-span types in `src/memory/map.rs`
- [x] T005 [P] Add normalized memory-region kinds and boot snapshot structures in `src/memory/map.rs`
- [x] T006 [P] Extend UEFI boot definitions for memory-map access in `src/boot/uefi.rs`
- [x] T007 Implement page-aligned memory-region normalization helpers in `src/memory/map.rs`
- [x] T008 Isolate and document boot memory-map `unsafe` invariants in `src/boot/uefi.rs`
- [x] T009 Wire the new memory module and renamed boot module into `src/main.rs` and `src/lib.rs`
- [x] T010 Add host tests for page alignment and region normalization rules in `tests/host/boot_memory_map.rs`

**Checkpoint**: Boot memory information and shared page model are ready for story work

---

## Phase 3: User Story 1 - Initialize Allocator State (Priority: P1) 🎯 MVP

**Goal**: Build the allocator's initial free/used view from boot-provided memory information

**Independent Test**: Start the allocator from boot-provided memory information and verify that usable pages are tracked as available while reserved or occupied pages are not offered for allocation

### Validation for User Story 1 ⚠️

- [x] T011 [P] [US1] Add allocator-initialization host tests for usable, reserved, kernel, and metadata pages in `tests/host/allocator.rs`
- [x] T012 [US1] Record reproducible allocator initialization validation steps in `specs/002-bitmap-allocator/quickstart.md`

### Implementation for User Story 1

- [x] T013 [P] [US1] Implement bitmap sizing and metadata reservation helpers in `src/memory/bitmap.rs`
- [x] T014 [P] [US1] Implement boot memory snapshot capture from the UEFI environment in `src/boot/uefi.rs`
- [x] T015 [US1] Implement allocator state construction from normalized regions in `src/memory/bitmap.rs`
- [x] T016 [US1] Contain and document bitmap-storage `unsafe` invariants in `src/memory/bitmap.rs`
- [x] T017 [US1] Integrate allocator initialization into the boot path in `src/main.rs`
- [x] T018 [US1] Export allocator initialization types needed by tests and later stories in `src/memory/mod.rs` and `src/lib.rs`

**Checkpoint**: User Story 1 should produce a ready allocator with reserved pages excluded from allocation

---

## Phase 4: User Story 2 - Allocate Pages (Priority: P2)

**Goal**: Allocate one or more contiguous free physical pages and mark them used

**Independent Test**: Request pages from an initialized allocator and verify that the allocator returns free page ranges, marks them as used, and refuses requests that exceed the currently available space

### Validation for User Story 2 ⚠️

- [x] T019 [P] [US2] Add host tests for single-page, contiguous multi-page, zero-page, and out-of-memory allocation behavior in `tests/host/allocator.rs`
- [x] T020 [US2] Add allocator allocation validation notes and expected commands in `specs/002-bitmap-allocator/quickstart.md`

### Implementation for User Story 2

- [x] T021 [P] [US2] Implement bitmap bit-set and bit-scan helpers for first-fit allocation in `src/memory/bitmap.rs`
- [x] T022 [P] [US2] Define allocation result variants and allocation request validation in `src/memory/map.rs`
- [x] T023 [US2] Implement single-page and contiguous multi-page allocation APIs in `src/memory/bitmap.rs`
- [x] T024 [US2] Update allocator bookkeeping for free-page counts and failed-request rollback in `src/memory/bitmap.rs`
- [x] T025 [US2] Expose allocation entry points from `src/memory/mod.rs`

**Checkpoint**: User Stories 1 and 2 should now support safe contiguous page allocation with deterministic failure results

---

## Phase 5: User Story 3 - Free Pages (Priority: P3)

**Goal**: Free previously allocated page spans safely and make them reusable

**Independent Test**: Free a page range that was previously allocated and verify that the allocator marks it available again without releasing pages that were never valid for freeing

### Validation for User Story 3 ⚠️

- [x] T026 [P] [US3] Add host tests for valid free, reuse after free, overlapping free, and out-of-range free behavior in `tests/host/allocator.rs`
- [x] T027 [US3] Add invalid-free and reuse validation notes in `specs/002-bitmap-allocator/quickstart.md`

### Implementation for User Story 3

- [x] T028 [P] [US3] Implement page-span ownership checks and range validation for free operations in `src/memory/bitmap.rs`
- [x] T029 [US3] Implement free APIs that clear bitmap state only for valid owned spans in `src/memory/bitmap.rs`
- [x] T030 [US3] Update allocation result handling for invalid frees in `src/memory/map.rs` and `src/memory/mod.rs`
- [x] T031 [US3] Verify freed spans become reusable through allocator integration points in `src/memory/bitmap.rs`

**Checkpoint**: All user stories should now be independently functional, including safe free and reuse behavior

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Final integration, documentation, and reproducible validation across all stories

- [x] T032 [P] Document the allocator architecture and boot-memory source in `README.md` and `docs/session-2026-05-01-hello-boot.md`
- [x] T033 Update the bitmap allocator contract details if implementation names differ in `specs/002-bitmap-allocator/contracts/allocator-interface.md`
- [x] T034 Run and record final validation for `cargo test`, `cargo check --target x86_64-unknown-uefi`, `make build`, and `./run.sh` in `specs/002-bitmap-allocator/quickstart.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies
- **Foundational (Phase 2)**: Depends on Setup completion and blocks all story work
- **User Story 1 (Phase 3)**: Depends on Foundational completion
- **User Story 2 (Phase 4)**: Depends on User Story 1 because allocation requires an initialized allocator
- **User Story 3 (Phase 5)**: Depends on User Story 2 because free behavior requires allocated spans
- **Polish (Phase 6)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: First deliverable and MVP foundation
- **User Story 2 (P2)**: Builds on the initialized allocator from US1
- **User Story 3 (P3)**: Builds on allocation behavior from US2

### Within Each User Story

- Host-side validation tasks should be completed before or alongside implementation
- Shared data types before allocator behavior
- Unsafe boundaries must be implemented and documented before integrating the boot path
- Core allocator logic before public re-exports and documentation updates

### Parallel Opportunities

- `T002` and `T003` can run in parallel after setup starts
- `T005` and `T006` can run in parallel in the foundational phase
- `T013` and `T014` can run in parallel in US1
- `T021` and `T022` can run in parallel in US2
- `T026` and `T027` can run in parallel in US3
- `T032` and `T033` can run in parallel in the polish phase

---

## Parallel Example: User Story 1

```bash
# Launch US1 validation and isolated implementation work together:
Task: "Add allocator-initialization host tests for usable, reserved, kernel, and metadata pages in tests/host/allocator.rs"
Task: "Implement bitmap sizing and metadata reservation helpers in src/memory/bitmap.rs"
Task: "Implement boot memory snapshot capture from the UEFI environment in src/boot/uefi.rs"
```

---

## Parallel Example: User Story 2

```bash
# Launch US2 building blocks together:
Task: "Add host tests for single-page, contiguous multi-page, zero-page, and out-of-memory allocation behavior in tests/host/allocator.rs"
Task: "Implement bitmap bit-set and bit-scan helpers for first-fit allocation in src/memory/bitmap.rs"
Task: "Define allocation result variants and allocation request validation in src/memory/map.rs"
```

---

## Parallel Example: User Story 3

```bash
# Launch US3 validation and ownership checks together:
Task: "Add host tests for valid free, reuse after free, overlapping free, and out-of-range free behavior in tests/host/allocator.rs"
Task: "Implement page-span ownership checks and range validation for free operations in src/memory/bitmap.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: User Story 1
4. Stop and validate allocator initialization before adding allocation APIs

### Incremental Delivery

1. Deliver allocator initialization from boot memory data
2. Add contiguous allocation behavior
3. Add free and reuse behavior
4. Finish with documentation and full boot validation

### Parallel Team Strategy

1. One developer handles boot module renaming and UEFI memory-map access
2. One developer handles shared memory-region and page-span modeling
3. After US1 lands, allocation and free logic can be split across separate changes on `src/memory/bitmap.rs` with coordination

---

## Notes

- [P] tasks use different files or isolated non-dependent changes
- Each story remains independently testable at its checkpoint
- No task introduces third-party dependencies or new assembly
- Validation tasks are mandatory because allocator changes affect boot-critical memory management
