# rust-os

`rust-os` is a pure-Rust x86_64 OS bring-up project built around a custom UEFI
loader and a freestanding higher-half kernel.

The current system boots `BOOTX64.EFI` directly from the EFI system partition,
prints progress over COM1 serial, collects boot information from UEFI, chooses a
simple early physical-memory layout, loads `KERNEL.BIN` from disk, parses it as
ELF64, and places each `PT_LOAD` segment into physical memory. It then builds
a 4-level x86_64 page table, enables paging, and transfers control to the
higher-half kernel entry point.

## Current State

- Boot target: direct UEFI boot of `EFI/BOOT/BOOTX64.EFI`
- Loader target: `x86_64-unknown-uefi`
- Kernel target: `x86_64-unknown-none`
- Output path: `bin/hello-boot.img`
- Runtime flow: `make build` then `./run.sh`

At runtime the loader and kernel report:

- loader image start/end from `LoadedImageProtocol`
- copied UEFI memory-map metadata
- a simple early allocation layout:
  - kernel-usable region
  - kernel stack region (higher-half mapped)
  - boot-info region (higher-half mapped)
  - page-table region
- kernel ELF entry point
- physical and virtual placement of every loaded `PT_LOAD` segment
- successful transition to the kernel with a higher-half stack and `BootInfo` pointer

## Boot Flow

1. Firmware starts `EFI/BOOT/BOOTX64.EFI`.
2. The loader initializes serial output.
3. The loader gathers boot info from UEFI.
4. The loader derives a simple early physical-memory layout from conventional
   memory above `2 MiB`.
5. The loader opens `EFI/BOOT/KERNEL.BIN`.
6. The loader reads the ELF headers, parses the loadable segments, and copies
   each `PT_LOAD` segment into physical memory.
7. The loader builds x86_64 page tables:
   - Identity-maps loader transition regions and boot data.
   - Maps kernel segments and stack into the higher-half (`0xffffffff80000000`).
   - Maps the `BootInfo` handoff structure into the higher-half (`0xffff_ffff_8009_0000`).
8. The loader enables paging, switches to the kernel stack, and jumps to the entry point.
9. The kernel starts, prints its banner, and verifies its higher-half environment.

Paging is now enabled, and control is successfully transferred to the kernel.

## Project Layout

```text
Cargo.toml
Makefile
run.sh
asm/
kernel/
linker/
loader/
specs/
src/
tests/
```

High-level split:

- `loader/`: UEFI entrypoint and early boot logic
- `kernel/`: freestanding higher-half kernel binary
- `src/`: shared low-level code and shared boot/handoff types
- `linker/`: separate loader and kernel linker scripts

## Quickstart

Build:

```bash
make build
```

Run:

```bash
./run.sh
```
