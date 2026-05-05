# rust-os

`rust-os` is a pure-Rust x86_64 OS bring-up project built around a custom UEFI
loader and a freestanding higher-half kernel.

The current system boots `BOOTX64.EFI` directly from the EFI system partition,
prints progress over COM1 serial, collects boot information from UEFI, chooses a
simple early physical-memory layout, loads `KERNEL.BIN` from disk, parses it as
ELF64, and places each `PT_LOAD` segment into physical memory before halting.

## Current State

- Boot target: direct UEFI boot of `EFI/BOOT/BOOTX64.EFI`
- Loader target: `x86_64-unknown-uefi`
- Kernel target: `x86_64-unknown-none`
- Output path: `bin/hello-boot.img`
- Runtime flow: `make build` then `./run.sh`

At runtime the loader currently reports:

- loader image start/end from `LoadedImageProtocol`
- copied UEFI memory-map metadata
- a simple early allocation layout:
  - kernel-usable region
  - boot-info region
  - page-table region
- kernel ELF entry point
- physical and virtual placement of every loaded `PT_LOAD` segment

## Boot Flow

1. Firmware starts `EFI/BOOT/BOOTX64.EFI`.
2. The loader initializes serial output.
3. The loader gathers boot info from UEFI.
4. The loader derives a simple early physical-memory layout from conventional
   memory above `2 MiB`.
5. The loader opens `EFI/BOOT/KERNEL.BIN`.
6. The loader reads the ELF headers, parses the loadable segments, and copies
   each `PT_LOAD` segment into physical memory.
7. The loader prints the resolved load plan and halts.

Paging is not enabled yet, and control is not yet transferred to the kernel.

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

## Next Steps

- Build initial x86_64 page tables in the reserved page-table region.
- Identity-map only the loader-side memory needed during the transition.
- Map the loaded kernel ELF segments at their higher-half virtual addresses.
- Reserve and map an initial higher-half kernel stack.
- Extend the boot handoff with the minimal state the kernel needs.
- Enable paging, switch to the kernel stack, and jump to the kernel entry point.
