# Tasks: Runtime Execution Ownership

**Input**: Design documents from `/specs/005-runtime-execution-ownership/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: Include validation tasks for every feature. Automated Rust tests are strongly preferred when practical, and boot or emulator validation tasks are mandatory for kernel-critical changes.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Refresh feature docs and validation scaffolding for the runtime-ownership work

- [X] T001 Update feature quickstart transcript expectations in `specs/005-runtime-execution-ownership/quickstart.md`
- [X] T002 [P] Add runtime-ownership transcript assertions in `tests/e2e_boot_serial.py`
- [X] T003 [P] Add feature module declarations and placeholders in `src/arch/x86_64/mod.rs`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Establish shared architecture interfaces and ownership boundaries before any story work

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T004 Define higher-half continuation and alias-teardown metadata in `src/arch/x86_64/paging.rs`
- [X] T005 [P] Add shared descriptor-table and trap-frame types in `src/arch/x86_64/gdt.rs`
- [X] T006 [P] Add shared IDT vector and handler registration types in `src/arch/x86_64/idt.rs`
- [X] T007 [P] Add shared PIC/PIT timer control and IRQ constants in `src/arch/x86_64/timer.rs`
- [X] T008 Add host-side ownership invariants for continuation metadata and interrupt proof events in `tests/host/paging.rs`
- [X] T009 Document required assembly and unsafe invariants for continuation and interrupt entry in `specs/005-runtime-execution-ownership/contracts/runtime-execution-interface.md`

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Continue In The Higher Half (Priority: P1) 🎯 MVP

**Goal**: Resume from a deterministic higher-half continuation point and remove the temporary low/current execution alias immediately afterward

**Independent Test**: Boot in QEMU, activate the runtime root, reach the higher-half continuation marker, remove the low/current alias immediately after the first confirmed higher-half step, and continue emitting serial output

### Validation for User Story 1 ⚠️

- [X] T010 [P] [US1] Add Rust tests for continuation-plan and alias-removal invariants in `tests/host/paging.rs`
- [X] T011 [US1] Define higher-half continuation and alias-removal boot validation in `specs/005-runtime-execution-ownership/quickstart.md`

### Implementation for User Story 1

- [ ] T012 [P] [US1] Extend transition-alias mapping and teardown support in `src/memory/paging/address_space.rs`
- [X] T013 [P] [US1] Implement deterministic continuation-plan construction in `src/arch/x86_64/paging.rs`
- [X] T014 [US1] Add the continuation entry and post-switch jump stub in `asm/boot.s`
- [X] T015 [US1] Integrate the higher-half continuation flow and alias-removal markers in `src/main.rs`
- [X] T016 [US1] Document the continuation and alias-removal unsafe boundary in `src/arch/x86_64/paging.rs`

**Checkpoint**: User Story 1 should boot through the runtime root, continue in the higher half, and run without the temporary low/current alias

---

## Phase 4: User Story 2 - Own Execution Context (Priority: P2)

**Goal**: Replace firmware-owned descriptor and interrupt state with kernel-owned GDT, IDT, TSS, a breakpoint proof path, and a hardware timer IRQ path

**Independent Test**: Boot in QEMU, install kernel-owned GDT/IDT/TSS state, trigger a deliberate breakpoint exception, handle a hardware timer interrupt, and prove both paths stay under kernel control

### Validation for User Story 2 ⚠️

- [ ] T017 [P] [US2] Add Rust tests for descriptor ownership and interrupt proof metadata in `tests/host/paging.rs`
- [ ] T018 [US2] Define breakpoint and timer proof-path validation in `specs/005-runtime-execution-ownership/quickstart.md`

### Implementation for User Story 2

- [ ] T019 [P] [US2] Implement the kernel-owned GDT and TSS setup in `src/arch/x86_64/gdt.rs`
- [ ] T020 [P] [US2] Implement the kernel-owned IDT and vector installation in `src/arch/x86_64/idt.rs`
- [ ] T021 [P] [US2] Implement breakpoint and timer interrupt handlers plus acknowledgement flow in `src/arch/x86_64/interrupts.rs`
- [ ] T022 [P] [US2] Program the legacy PIC and PIT timer path in `src/arch/x86_64/timer.rs`
- [ ] T023 [US2] Add interrupt-entry stubs and return-sensitive assembly only where required in `asm/boot.s`
- [ ] T024 [US2] Integrate execution-context installation and proof markers in `src/main.rs`
- [ ] T025 [US2] Document descriptor, interrupt, and timer unsafe invariants in `src/arch/x86_64/interrupts.rs`

**Checkpoint**: User Stories 1 and 2 should boot with kernel-owned descriptor state and prove both breakpoint and timer handling without firmware-owned fallback

---

## Phase 5: User Story 3 - Enter A Stable Idle State (Priority: P3)

**Goal**: Replace the `cli`/`hlt` stopgap with an interrupt-enabled idle loop that wakes through the kernel-owned hardware timer path and remains stable

**Independent Test**: Boot in QEMU, enter the interrupt-driven idle path after execution-context ownership is installed, wake on a hardware timer interrupt, and remain stable without rebooting

### Validation for User Story 3 ⚠️

- [ ] T026 [P] [US3] Add Rust tests for idle-state and timer-wakeup state transitions in `tests/host/paging.rs`
- [ ] T027 [US3] Define interrupt-driven idle validation and reboot-loop checks in `specs/005-runtime-execution-ownership/quickstart.md`

### Implementation for User Story 3

- [ ] T028 [P] [US3] Replace the halt abstraction with an interrupt-driven idle entry point in `src/arch/x86_64/halt.rs`
- [ ] T029 [P] [US3] Add idle wake bookkeeping and resume markers in `src/arch/x86_64/interrupts.rs`
- [ ] T030 [US3] Integrate the stable idle path after runtime ownership setup in `src/main.rs`
- [ ] T031 [US3] Remove the permanent `cli` stopgap from the halt loop in `asm/boot.s`
- [ ] T032 [US3] Document idle-state ownership and timer-wakeup invariants in `src/arch/x86_64/halt.rs`

**Checkpoint**: All user stories should now be independently functional, with the kernel idling under interrupt-driven wakeup instead of a blanket interrupt-disable workaround

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Final consistency, validation, and documentation across all stories

- [ ] T033 [P] Update runtime-ownership implementation notes in `specs/005-runtime-execution-ownership/plan.md`
- [ ] T034 [P] Add final transcript and validation notes in `specs/005-runtime-execution-ownership/quickstart.md`
- [ ] T035 Run full validation coverage and record results in `specs/005-runtime-execution-ownership/quickstart.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Foundational completion
- **User Story 2 (Phase 4)**: Depends on Foundational completion and uses the continuation path from US1
- **User Story 3 (Phase 5)**: Depends on Foundational completion and on US2's owned timer interrupt path
- **Polish (Phase 6)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational - no dependency on later stories
- **User Story 2 (P2)**: Depends on US1 because descriptor ownership is installed after the higher-half continuation and alias teardown succeed
- **User Story 3 (P3)**: Depends on US2 because stable idle requires the kernel-owned timer interrupt path

### Within Each User Story

- Validation tasks should be completed before or alongside implementation
- Metadata and support modules before integration in `src/main.rs`
- Assembly and unsafe changes must stay isolated and justified
- Integration markers and quickstart updates complete the story

### Parallel Opportunities

- `T002` and `T003` can run in parallel during Setup
- `T005`, `T006`, and `T007` can run in parallel during Foundational work
- In US1, `T012` and `T013` can run in parallel before integration
- In US2, `T019`, `T020`, `T021`, and `T022` can run in parallel before integration
- In US3, `T028` and `T029` can run in parallel before final idle integration

---

## Parallel Example: User Story 2

```bash
# Launch validation preparation together:
Task: "Add Rust tests for descriptor ownership and interrupt proof metadata in tests/host/paging.rs"
Task: "Define breakpoint and timer proof-path validation in specs/005-runtime-execution-ownership/quickstart.md"

# Launch core implementation work together:
Task: "Implement the kernel-owned GDT and TSS setup in src/arch/x86_64/gdt.rs"
Task: "Implement the kernel-owned IDT and vector installation in src/arch/x86_64/idt.rs"
Task: "Implement breakpoint and timer interrupt handlers plus acknowledgement flow in src/arch/x86_64/interrupts.rs"
Task: "Program the legacy PIC and PIT timer path in src/arch/x86_64/timer.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: User Story 1
4. Stop and validate the higher-half continuation plus immediate alias teardown in QEMU

### Incremental Delivery

1. Complete Setup + Foundational
2. Deliver US1 as the first owned runtime execution milestone
3. Add US2 to take ownership of descriptor and interrupt state
4. Add US3 to replace the `cli`/`hlt` stopgap with stable interrupt-driven idle
5. Finish with polish and full validation

### Parallel Team Strategy

1. Complete Setup + Foundational together
2. Assign one engineer to continuation and alias teardown, one to descriptor and interrupt modules, and one to validation/docs as soon as dependencies allow
3. Merge at each story checkpoint and re-run the boot transcript validation before advancing

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] labels map tasks to specific user stories for traceability
- Each user story is independently completable and testable at its checkpoint
- Validation, assembly, and unsafe-boundary tasks are included because they are core to this feature
- Avoid carrying the temporary low/current execution alias or blanket `cli` workaround past the story that removes them
