# Implementation Plan: Live VM Handoff

**Branch**: `004-live-vm-handoff` | **Date**: 2026-05-02 | **Spec**: [specs/004-live-vm-handoff/spec.md](./spec.md)
**Input**: Feature specification from `/specs/004-live-vm-handoff/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

Extend the existing virtual-memory groundwork by converting the software paging
model into hardware-consumable x86_64 page tables in RAM, adding a kernel
direct-map window for allocator-managed physical memory, switching into a
kernel-owned CR3 root, and continuing execution in the higher half before
removing the temporary low bootstrap alias. The previous `003` feature remains
the substrate for ownership tracking, rollback, and process-space modeling; this
feature turns that substrate into the live runtime path.

## Technical Context

**Language/Version**: Rust 2024 with `no_std` on the `x86_64-unknown-uefi` target  
**Primary Dependencies**: No third-party code; Rust toolchain crates plus existing GRUB host/runtime components only  
**Storage**: N/A for persistent storage; paging structures, direct-map bookkeeping, and allocator ownership all live in RAM  
**Testing**: `cargo test`, `cargo check --target x86_64-unknown-uefi`, `make build`, and `./run.sh`  
**Target Platform**: x86_64 UEFI kernel booted through GRUB in QEMU on macOS  
**Project Type**: Bare-metal operating system kernel memory-management and bootstrap-handoff subsystem  
**Performance Goals**: Build the initial live page-table hierarchy in time proportional to mapped ranges and paging pages, keep direct-map translation constant-time, and keep rollback deterministic on all failure paths  
**Constraints**: No new dependencies, no new standalone assembly files, 4 KiB pages only, a non-overlapping higher-half kernel image region plus full allocator-managed direct-map region, no retained low alias after the handoff, and `unsafe` confined to page-table publication, direct-map memory access, and CR3/continuation code  
**Scale/Scope**: One live kernel address space with higher-half image mappings and direct-map RAM coverage, allocator-backed smoke validation after the switch, and process address spaces cloned from the active kernel template; no scheduler, user mode, demand paging, huge pages, or SMP synchronization in this milestone

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] No third-party dependencies are introduced; the plan stays within Rust toolchain crates, GRUB, and the existing allocator.
- [x] Any GRUB integration change is explicitly described and justified; GRUB remains the bootloader while the kernel takes ownership of runtime paging after boot.
- [x] Any new assembly is the minimum required surface and includes a justification; no new standalone assembly files are planned, and inline assembly remains limited to CR3/transition-sensitive architecture code if Rust-only control flow proves insufficient.
- [x] Any new or expanded `unsafe` boundary documents its invariants and safe wrapper plan; page-table writes, direct-map memory access, and CR3 handoff stay isolated under memory and arch modules.
- [x] Validation covers all affected boot or kernel-critical paths with reproducible commands; host tests, freestanding target checks, image build, and QEMU bring-up remain required.

Post-design re-check: Passed. The design keeps dependency policy intact,
contains low-level escape hatches, and defines a reproducible proof path for the
live handoff and post-switch direct-map access.

## Project Structure

### Documentation (this feature)

```text
specs/004-live-vm-handoff/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── runtime-paging-interface.md
└── tasks.md
```

### Source Code (repository root)

```text
Cargo.toml
Makefile
run.sh
src/
├── lib.rs
├── main.rs
├── boot/
│   ├── mod.rs
│   └── uefi.rs
├── arch/
│   ├── mod.rs
│   └── x86_64/
│       ├── framebuffer.rs
│       ├── halt.rs
│       ├── mod.rs
│       ├── paging.rs
│       └── serial.rs
├── kernel/
│   ├── hello.rs
│   └── mod.rs
└── memory/
    ├── bitmap.rs
    ├── map.rs
    ├── mod.rs
    └── paging/
        ├── address_space.rs
        ├── mapper.rs
        ├── mod.rs
        └── table.rs
tests/
├── host.rs
├── e2e_boot_serial.py
└── host/
    ├── allocator.rs
    ├── boot_memory_map.rs
    ├── paging.rs
    └── run_contract.rs
asm/
linker/
grub/
```

**Structure Decision**: Keep generic address-space layout, direct-map
translation helpers, ownership tracking, and page-table publication logic in
`src/memory/paging/`, while keeping CR3 loading and higher-half continuation in
`src/arch/x86_64/paging.rs`. Boot-time orchestration and post-switch smoke
validation live in `src/main.rs`, and transcript-based proof remains in
`tests/e2e_boot_serial.py`.

## Implementation Notes

- The runtime root now installs a shared direct-map window at
  `KERNEL_DIRECT_MAP_BASE` and records the allocator-managed physical limit in
  the cloned kernel template used for process address spaces.
- Host-side validation now asserts direct-map round trips, inherited direct-map
  visibility in process roots, rollback stability, and the stable transcript
  markers `paging root:` and `direct-map smoke:`.
- `unsafe` remains concentrated in three places: paging-frame writes in
  `src/memory/paging/address_space.rs`, CR3 activation in
  `src/arch/x86_64/paging.rs`, and direct-map smoke access in `src/main.rs`.

## Complexity Tracking

No constitution violations are expected. The main implementation complexity is a
small bootstrap window where the old execution path and the new higher-half path
must both remain valid long enough to load CR3, resume at the higher-half
continuation point, and then drop the low alias without losing serial output or
stack reachability.
