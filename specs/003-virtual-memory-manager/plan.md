# Implementation Plan: Virtual Memory Manager

**Branch**: `003-virtual-memory-manager` | **Date**: 2026-05-02 | **Spec**: [specs/003-virtual-memory-manager/spec.md](./spec.md)
**Input**: Feature specification from `/specs/003-virtual-memory-manager/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

Introduce the kernel's first owned virtual-memory subsystem by building x86_64
page-table hierarchies from physical pages supplied by the existing bitmap
allocator, preparing a higher-half kernel address space, and creating fresh
process address spaces that share kernel mappings while keeping private mappings
independent. The design keeps paging-structure ownership explicit so bootstrap,
rollback, and teardown all have deterministic outcomes.

## Technical Context

**Language/Version**: Rust 2024 with `no_std` on the `x86_64-unknown-uefi` target  
**Primary Dependencies**: No third-party code; Rust toolchain crates plus existing GRUB host/runtime components only  
**Storage**: N/A for persistent storage; paging structures and bookkeeping live in allocator-owned physical pages during boot and runtime  
**Testing**: `cargo test`, `cargo check --target x86_64-unknown-uefi`, `make build`, and `./run.sh`  
**Target Platform**: x86_64 UEFI kernel booted through GRUB in QEMU on macOS  
**Project Type**: Bare-metal operating system kernel memory-management subsystem  
**Performance Goals**: Build the initial kernel paging hierarchy in time proportional to required mapped ranges and page-table pages, keep per-mapping expansion bounded to the required hierarchy walk, and make failure/rollback deterministic  
**Constraints**: No new dependencies, no new standalone assembly files, 4 KiB pages only, higher-half kernel execution with no identity mapping retained after the transition, allocator ownership must stay consistent across partial failures, and `unsafe` must stay confined to paging-entry writes plus control-register transition code  
**Scale/Scope**: One kernel address space template, fresh per-process root page tables with shared kernel higher-half mappings, no scheduler or context switching yet, no user-mode execution yet, and no demand paging, swapping, huge pages, or SMP synchronization in this milestone

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] No third-party dependencies are introduced; the plan stays within Rust toolchain crates, GRUB, and the existing in-repo allocator.
- [x] Any GRUB integration change is explicitly described and justified; GRUB remains the bootloader while the kernel takes ownership of paging structures after boot.
- [x] Any new assembly is the minimum required surface and includes a justification; no new standalone assembly files are planned, and any required control-register writes stay in a narrow architecture-owned path.
- [x] Any new or expanded `unsafe` boundary documents its invariants and safe wrapper plan; paging-entry mutation and activation of a new page-table root will be wrapped behind memory and architecture modules with explicit ownership rules.
- [x] Validation covers all affected boot or kernel-critical paths with reproducible commands; host-side tests, freestanding target checks, image build, and QEMU boot remain required.

Post-design re-check: Passed. The selected design reuses the existing physical
allocator, keeps new low-level escape hatches narrow, avoids new dependencies,
and defines a reproducible validation path for both pure paging logic and the
boot transition into higher-half execution.

## Project Structure

### Documentation (this feature)

```text
specs/003-virtual-memory-manager/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── paging-interface.md
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

**Structure Decision**: Keep architecture-specific register activation and
address constants under `src/arch/x86_64/`, while the reusable paging data
structures, mapping logic, and address-space ownership model live under a new
`src/memory/paging/` subtree. This preserves the current separation between boot
handoff, architecture hooks, and memory management while making per-process
address-space creation testable on the host.

## Complexity Tracking

No constitution violations are expected. The main planned complexity is a narrow
bootstrap transition that briefly requires both the current execution path and
the new higher-half continuation to remain reachable long enough to switch page
table roots and then drop the low-address alias from the active address space.
