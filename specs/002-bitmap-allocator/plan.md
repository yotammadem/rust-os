# Implementation Plan: Bitmap Allocator

**Branch**: `002-bitmap-allocator` | **Date**: 2026-05-01 | **Spec**: [specs/002-bitmap-allocator/spec.md](./spec.md)
**Input**: Feature specification from `/specs/002-bitmap-allocator/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

Add the first physical page allocator to the kernel by capturing the boot-time
UEFI memory map available in the GRUB-launched environment, normalizing it into
page-aligned usable and reserved regions, reserving space for allocator-owned
bitmap metadata, and exposing contiguous page allocation and free operations
with deterministic failure behavior.

## Technical Context

**Language/Version**: Rust 2024 with `no_std` on the `x86_64-unknown-uefi` target  
**Primary Dependencies**: No third-party code; Rust toolchain crates plus GRUB host/runtime components only  
**Storage**: N/A for persistent storage; allocator metadata lives in reserved physical pages during boot  
**Testing**: `cargo test`, `cargo check --target x86_64-unknown-uefi`, `make build`, and `./run.sh`  
**Target Platform**: x86_64 UEFI kernel booted through GRUB in QEMU on macOS  
**Project Type**: Bare-metal operating system kernel memory-management subsystem  
**Performance Goals**: Initialize deterministically during early boot, keep allocator setup linear in reported memory regions, and keep first-fit page scans acceptable for the small single-core bring-up scope  
**Constraints**: No new dependencies, no new assembly, contiguous multi-page allocations only, fixed 4 KiB page size, invalid frees must fail without state changes, and `unsafe` must stay confined to boot memory parsing plus raw bitmap storage access  
**Scale/Scope**: One early-boot physical allocator instance, one address space, no heap allocator, no virtual memory manager, and no SMP or locking in this milestone

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] No third-party dependencies are introduced; the design stays within Rust toolchain crates and GRUB.
- [x] Any GRUB integration change is explicitly described and justified; GRUB remains the bootloader while allocator input comes from the UEFI boot environment it launches.
- [x] Any new assembly is the minimum required surface and includes a justification; this feature plans no new assembly.
- [x] Any new or expanded `unsafe` boundary documents its invariants and safe wrapper plan; raw UEFI memory-map access and in-place bitmap storage will be wrapped behind allocator initialization APIs.
- [x] Validation covers all affected boot or kernel-critical paths with reproducible commands; host-side allocator tests and the existing `make build` plus `./run.sh` boot path are required.

Post-design re-check: Passed. The selected design keeps the existing boot chain,
adds no dependencies, avoids new assembly, and defines a reproducible validation
path for both pure allocator logic and boot-time initialization.

## Project Structure

### Documentation (this feature)

```text
specs/002-bitmap-allocator/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── allocator-interface.md
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
│       └── mod.rs
├── kernel/
│   ├── hello.rs
│   └── mod.rs
└── memory/
    ├── bitmap.rs
    ├── map.rs
    └── mod.rs
tests/
├── host.rs
└── host/
    ├── allocator.rs
    ├── boot_memory_map.rs
    └── run_contract.rs
asm/
linker/
grub/
```

**Structure Decision**: Add a dedicated `src/memory/` subtree for normalized
memory regions and bitmap allocation logic, and move the boot protocol
definitions to `src/boot/uefi.rs` so the allocator integrates with accurately
named UEFI handoff code instead of expanding the misleading `multiboot` module.

## Complexity Tracking

No constitution violations are expected. The main planned complexity is a narrow
`unsafe` initialization path that snapshots the UEFI memory map and reserves
physical pages for the bitmap before exposing a safe allocator interface.
