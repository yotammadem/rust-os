---

description: "Task list for implementing the Hello Boot feature"
---

# Tasks: Hello Boot

**Input**: Design documents from `/specs/001-hello-boot/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Include validation tasks for every user story. Automated host-side Rust checks are included where practical, and QEMU boot validation is required for runtime milestones.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create the initial repository scaffolding and generated-output boundaries.

- [X] T001 Create the freestanding project layout in `./Cargo.toml`, `src/`, `asm/`, `linker/`, `grub/`, `tests/host/`, and `bin/.gitkeep`
- [X] T002 Initialize toolchain and ignore rules in `./rust-toolchain.toml` and `./.gitignore`
- [X] T003 [P] Add repository command surfaces in `./Makefile` and `./run.sh`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Establish the boot/runtime foundation that every user story relies on.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T004 Configure freestanding Rust build settings in `./Cargo.toml` and `.cargo/config.toml`
- [X] T005 [P] Create the linker and GRUB baseline assets in `linker/x86_64.ld` and `grub/grub.cfg`
- [X] T006 [P] Implement the x86_64 bootstrap entry and halt symbols in `asm/boot.s`
- [X] T007 [P] Create the shared kernel module structure in `src/main.rs`, `src/boot/mod.rs`, `src/arch/x86_64/mod.rs`, and `src/kernel/mod.rs`
- [X] T008 Implement boot metadata parsing and framebuffer descriptors in `src/boot/multiboot.rs` and `src/arch/x86_64/framebuffer.rs`
- [X] T009 Isolate halt-path and low-level unsafe boundaries in `src/arch/x86_64/halt.rs`
- [X] T010 Document the baseline build and run validation flow in `specs/001-hello-boot/quickstart.md`

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Build Bootable Image (Priority: P1) 🎯 MVP

**Goal**: Produce one bootable x86_64 UEFI disk image in `bin/` from a standard `make build` flow.

**Independent Test**: From a clean checkout, run `make build` and verify that `bin/hello-boot.img` is created as the single runtime image without any manual file moves.

### Validation for User Story 1 ⚠️

- [X] T011 [P] [US1] Add a host-side build smoke check for image path expectations in `tests/host/build_artifact.rs`
- [X] T012 [US1] Record the MVP build verification steps in `specs/001-hello-boot/quickstart.md`

### Implementation for User Story 1

- [X] T013 [P] [US1] Implement the kernel crate entry and panic behavior in `src/main.rs`
- [X] T014 [P] [US1] Implement the initial kernel control flow modules in `src/kernel/mod.rs` and `src/kernel/hello.rs`
- [X] T015 [US1] Wire the freestanding build pipeline to emit the kernel binary in `./Cargo.toml`, `.cargo/config.toml`, and `linker/x86_64.ld`
- [X] T016 [US1] Implement GRUB image staging and raw disk image creation in `./Makefile`
- [X] T017 [US1] Ensure generated artifacts stay under `bin/` by updating `./.gitignore` and `bin/.gitkeep`
- [X] T018 [US1] Update the build artifact contract in `specs/001-hello-boot/contracts/build-run-contract.md`

**Checkpoint**: User Story 1 should build `bin/hello-boot.img` reliably from `make build`

---

## Phase 4: User Story 2 - Launch Local Emulator Session (Priority: P2)

**Goal**: Launch the built disk image through a standard `run.sh` QEMU flow with clear error handling.

**Independent Test**: After a successful build, run `./run.sh` and verify that it boots `bin/hello-boot.img`; if the image is missing, verify that the script exits with a clear error instead of building.

### Validation for User Story 2 ⚠️

- [X] T019 [P] [US2] Add a host-side script contract check for missing-image failure behavior in `tests/host/run_contract.rs`
- [X] T020 [US2] Record the run-script success and failure scenarios in `specs/001-hello-boot/quickstart.md`

### Implementation for User Story 2

- [X] T021 [P] [US2] Implement QEMU and firmware path resolution in `./run.sh`
- [X] T022 [US2] Enforce fail-fast missing-image and missing-firmware errors in `./run.sh`
- [X] T023 [US2] Align the `make build` output naming and `./run.sh` input contract in `./Makefile` and `./run.sh`
- [X] T024 [US2] Update the run interface contract in `specs/001-hello-boot/contracts/build-run-contract.md`

**Checkpoint**: User Story 2 should start QEMU from `./run.sh` using the latest built image and fail clearly when prerequisites are absent

---

## Phase 5: User Story 3 - Show Hello World and Halt (Priority: P3)

**Goal**: Boot the image to a visible `hello world` screen and halt while preserving the message until the emulator is closed.

**Independent Test**: Build the image, run `./run.sh`, confirm that `hello world` appears exactly once on screen, and verify that execution halts while the message remains visible.

### Validation for User Story 3 ⚠️

- [X] T025 [P] [US3] Add a host-side rendering helper check in `tests/host/framebuffer_console.rs`
- [X] T026 [US3] Record the end-to-end boot validation procedure in `specs/001-hello-boot/quickstart.md`

### Implementation for User Story 3

- [X] T027 [P] [US3] Implement framebuffer write primitives and glyph rendering in `src/arch/x86_64/framebuffer.rs`
- [X] T028 [P] [US3] Implement the hello-world boot path in `src/kernel/hello.rs`
- [X] T029 [US3] Connect boot metadata, framebuffer setup, and kernel execution in `src/boot/multiboot.rs`, `src/boot/mod.rs`, and `src/main.rs`
- [X] T030 [US3] Finalize the visible halted state in `src/arch/x86_64/halt.rs` and `asm/boot.s`
- [X] T031 [US3] Update the observable boot contract in `specs/001-hello-boot/contracts/build-run-contract.md`

**Checkpoint**: All user stories should now be independently functional, including the visible halted hello-world boot result

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Tighten documentation, repeatability, and overall validation.

- [X] T032 [P] Refresh plan-aligned usage and prerequisites in `./AGENTS.md` and `specs/001-hello-boot/quickstart.md`
- [X] T033 Run the full validation sequence with `cargo fmt --check`, `cargo check`, `make build`, and `./run.sh`, then capture any required doc fixes in `specs/001-hello-boot/quickstart.md`
- [X] T034 [P] Reduce unnecessary unsafe or assembly surface discovered during implementation in `src/arch/x86_64/`, `src/boot/`, and `asm/boot.s`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Foundational completion
- **User Story 2 (Phase 4)**: Depends on User Story 1 producing the disk image contract
- **User Story 3 (Phase 5)**: Depends on Foundational completion and uses the build/run surfaces delivered by User Stories 1 and 2
- **Polish (Phase 6)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: First deliverable and MVP - no dependency on later stories
- **User Story 2 (P2)**: Depends on the build artifact contract from US1
- **User Story 3 (P3)**: Depends on the foundational boot/runtime plumbing and is best validated through the US1/US2 flow

### Within Each User Story

- Validation tasks land before or alongside implementation where practical
- Rust module scaffolding precedes integration wiring
- Assembly and unsafe changes stay isolated and justified
- Each story must end with a runnable, independently checkable increment

### Parallel Opportunities

- `T003`, `T005`, `T006`, and `T007` can run in parallel once Phase 1 starts or completes as applicable
- In US1, `T013` and `T014` can run in parallel before `T015` and `T016`
- In US2, `T019` and `T021` can run in parallel before final contract alignment
- In US3, `T027` and `T028` can run in parallel before boot-path integration in `T029`
- `T032` and `T034` can run in parallel during polish

---

## Parallel Example: User Story 1

```bash
# Launch User Story 1 validation and module work together:
Task: "Add a host-side build smoke check for image path expectations in tests/host/build_artifact.rs"
Task: "Implement the kernel crate entry and panic behavior in src/main.rs"
Task: "Implement the initial kernel control flow modules in src/kernel/mod.rs and src/kernel/hello.rs"
```

## Parallel Example: User Story 3

```bash
# Launch the rendering and boot-message work together:
Task: "Implement framebuffer write primitives and glyph rendering in src/arch/x86_64/framebuffer.rs"
Task: "Implement the hello-world boot path in src/kernel/hello.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: Run `make build` and verify `bin/hello-boot.img`
5. Commit the MVP build pipeline before moving to runtime behavior

### Incremental Delivery

1. Setup + Foundational establish the freestanding Rust/GRUB baseline
2. User Story 1 delivers the bootable disk image artifact
3. User Story 2 delivers the repeatable QEMU launch workflow
4. User Story 3 delivers the visible hello-world halt proof
5. Phase 6 tightens docs and trims unnecessary low-level surface area

### Parallel Team Strategy

With multiple developers:

1. One developer handles `Cargo.toml` / `.cargo/config.toml` while another prepares `grub/grub.cfg` and `linker/x86_64.ld`
2. After Foundational, one developer can own `Makefile` image creation while another owns `run.sh`
3. Once build and run paths are stable, framebuffer rendering and hello-world control flow can proceed in parallel

---

## Notes

- All tasks follow the required checkbox, ID, label, and file-path format
- User story tasks always carry `[US1]`, `[US2]`, or `[US3]`
- The suggested MVP scope is Phase 3 / User Story 1 only
- Validation is part of each user story because the constitution requires reproducible bring-up checks
