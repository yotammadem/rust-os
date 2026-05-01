# Implementation Plan: Hello Boot

**Branch**: `001-hello-boot` | **Date**: 2026-05-01 | **Spec**: [specs/001-hello-boot/spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-hello-boot/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

Build a single bootable x86_64 UEFI disk image under `bin/`, boot it through
GRUB in QEMU, and run a freestanding Rust kernel that renders `hello world` once
to the screen and then halts while leaving the message visible. The technical
approach uses a `no_std` Rust kernel, a minimal architecture-specific assembly
bootstrap, a custom linker script, and a GRUB-managed EFI system partition image.

## Technical Context

**Language/Version**: Rust nightly with `no_std` freestanding target  
**Primary Dependencies**: No third-party code; Rust toolchain artifacts plus GRUB host/runtime components only  
**Storage**: Generated build artifacts only, under `bin/`  
**Testing**: `cargo fmt --check`, `cargo check`, optional host-side `cargo test` for pure helpers, `make build`, and manual `./run.sh` QEMU smoke validation  
**Target Platform**: x86_64 bare metal via UEFI GRUB, launched in QEMU on macOS  
**Project Type**: Bare-metal operating system kernel and boot image  
**Performance Goals**: Reach a visible `hello world` screen in one QEMU boot attempt and within roughly 5 seconds on a local dev machine  
**Constraints**: No third-party dependencies beyond GRUB; single bootable disk image `bin/hello-boot.img`; minimal assembly limited to bootstrap/halt path; `run.sh` must fail fast if the image or firmware is missing  
**Scale/Scope**: Single-core, single-message boot demo with no scheduler, heap, interrupts, persistence, or interactive input

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] No third-party dependencies are introduced; implementation uses only Rust toolchain components and GRUB.
- [x] Any GRUB integration change is explicitly described and justified; GRUB is the required UEFI bootloader and owns image boot handoff.
- [x] Any new assembly is the minimum required surface and includes a justification; one bootstrap file is planned for entry, stack setup, and final halt behavior.
- [x] Any new or expanded `unsafe` boundary documents its invariants and safe wrapper plan; framebuffer access and CPU halt loops will be isolated in narrow architecture modules.
- [x] Validation covers all affected boot or kernel-critical paths with reproducible commands; `make build` and `./run.sh` define the required validation flow.

Post-design re-check: Passed. The selected design remains inside the constitution with no justified exceptions required.

## Project Structure

### Documentation (this feature)

```text
specs/001-hello-boot/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── build-run-contract.md
└── tasks.md
```

### Source Code (repository root)

```text
Cargo.toml
Makefile
run.sh
grub/
└── grub.cfg
linker/
└── x86_64.ld
asm/
└── boot.s
src/
├── main.rs
├── boot/
│   ├── mod.rs
│   └── multiboot.rs
├── arch/
│   └── x86_64/
│       ├── mod.rs
│       ├── framebuffer.rs
│       └── halt.rs
└── kernel/
    ├── mod.rs
    └── hello.rs
bin/
tests/
└── host/
```

**Structure Decision**: Use a single freestanding Rust crate with a small amount
of architecture-specific support code. Keep GRUB configuration, linker state, and
assembly bootstrap in top-level support directories so boot-critical assets stay
obvious and isolated from the Rust kernel modules.

## Complexity Tracking

No constitution violations are expected. The only deliberate complexity is the
minimum boot assembly needed to enter Rust safely and halt the CPU after output.
