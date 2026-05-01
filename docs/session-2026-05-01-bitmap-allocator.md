# Session Report: 2026-05-01 Bitmap Allocator

## Scope

This session implemented the second milestone of the project:
`002 - Bitmap Allocator`.

The goal was to add an early bitmap-based physical page allocator that:

- captures boot-provided memory information from the current UEFI boot path,
- normalizes that memory into 4 KiB physical pages,
- reserves its own bitmap metadata from usable pages,
- allocates single or contiguous multiple pages,
- and frees valid page spans safely without corrupting allocator state.

## What Was Done

- Replaced the misnamed boot protocol module with [src/boot/uefi.rs](/Users/yotammadem/mademos/rust-os/src/boot/uefi.rs).
- Added UEFI memory-map capture and descriptor definitions in
  [src/boot/uefi.rs](/Users/yotammadem/mademos/rust-os/src/boot/uefi.rs).
- Added the memory subsystem in [src/memory/mod.rs](/Users/yotammadem/mademos/rust-os/src/memory/mod.rs),
  [src/memory/map.rs](/Users/yotammadem/mademos/rust-os/src/memory/map.rs), and
  [src/memory/bitmap.rs](/Users/yotammadem/mademos/rust-os/src/memory/bitmap.rs).
- Integrated allocator initialization into [src/main.rs](/Users/yotammadem/mademos/rust-os/src/main.rs)
  before the existing hello-world path runs.
- Added host-side tests in [tests/host/boot_memory_map.rs](/Users/yotammadem/mademos/rust-os/tests/host/boot_memory_map.rs)
  and [tests/host/allocator.rs](/Users/yotammadem/mademos/rust-os/tests/host/allocator.rs).
- Updated project and feature documentation in
  [README.md](/Users/yotammadem/mademos/rust-os/README.md),
  [specs/002-bitmap-allocator/quickstart.md](/Users/yotammadem/mademos/rust-os/specs/002-bitmap-allocator/quickstart.md),
  [specs/002-bitmap-allocator/contracts/allocator-interface.md](/Users/yotammadem/mademos/rust-os/specs/002-bitmap-allocator/contracts/allocator-interface.md),
  and [specs/002-bitmap-allocator/tasks.md](/Users/yotammadem/mademos/rust-os/specs/002-bitmap-allocator/tasks.md).

## Plain-English Explanation

This is the first real memory manager in the project.

The allocator works like this:

1. UEFI gives the program a memory map at boot.
2. That memory map tells us which physical memory ranges are usable and which
   are reserved.
3. The kernel converts those ranges into 4 KiB pages.
4. The allocator chooses a small usable page range to store its own bitmap.
5. It marks those pages as unavailable so they cannot be allocated later.
6. It then uses bitmap bits to track which pages are free and which are used.

### What the Bitmap Means

The allocator does not store a large struct for every page. It stores bits.

In the current implementation:

- one bitset says whether a page is allocatable at all,
- one bitset says whether that allocatable page is currently in use.

So a page is:

- free if it is allocatable and not used,
- used if it is allocatable and marked used,
- unavailable if it is not allocatable in the first place.

### Why the Allocator Uses the UEFI Memory Map

GRUB is still the bootloader, but the running program is a UEFI application.
That means the memory information already available to the kernel comes from
UEFI.

So this milestone intentionally reuses the current boot path instead of turning
the work into a boot-protocol migration.

### How the Allocator Bootstraps Itself

The allocator cannot ask a heap allocator for memory, because it is itself the
first allocator.

So it bootstraps itself by:

1. finding a usable region,
2. reserving enough pages for bitmap storage,
3. treating those pages as allocator-owned metadata,
4. and writing the bitmap directly there.

That direct write is why there is a narrow `unsafe` boundary in
[src/memory/bitmap.rs](/Users/yotammadem/mademos/rust-os/src/memory/bitmap.rs):
the code has to treat a physical page range as raw writable bytes during very
early boot.

## Current Architecture

```text
src/
├── lib.rs
├── main.rs
├── boot/
│   ├── mod.rs
│   └── uefi.rs
├── arch/
│   ├── mod.rs
│   └── x86_64/
│       ├── mod.rs
│       ├── framebuffer.rs
│       └── halt.rs
├── memory/
│   ├── mod.rs
│   ├── map.rs
│   └── bitmap.rs
└── kernel/
    ├── mod.rs
    └── hello.rs
```

## Responsibility Split

- `src/boot/uefi.rs`: UEFI table definitions, memory descriptors, and boot memory snapshot capture.
- `src/memory/map.rs`: shared page, region, snapshot, and allocation result types.
- `src/memory/bitmap.rs`: allocator initialization, metadata reservation, allocation, and free logic.
- `src/main.rs`: boot-time orchestration that captures the memory map, initializes the allocator, then continues into the visible boot path.
- `tests/host/boot_memory_map.rs`: page-alignment and normalization checks.
- `tests/host/allocator.rs`: initialization, allocation, invalid-request, free, and reuse checks.

## Boot Flow After This Session

1. QEMU starts UEFI firmware.
2. UEFI firmware starts GRUB.
3. GRUB chainloads the EFI payload.
4. `efi_main` runs.
5. The kernel captures the UEFI memory map.
6. The bitmap allocator initializes from that boot memory information.
7. The kernel continues into the existing hello-world output path.
8. The system halts.

## Validation Performed

- `cargo test` passed, including 18 host-side checks.
- `cargo check --target x86_64-unknown-uefi` passed.
- `make build` succeeded and produced `bin/hello-boot.img`.
- A bounded `./run.sh` smoke test reached the GRUB handoff into the EFI
  payload. The automated capture cannot directly prove on-screen UEFI text
  output because that output does not go through the serial log.

## Current Limitations

- The allocator is still a single early-boot physical page allocator only.
- There is still no heap allocator, virtual memory manager, or SMP-safe access model.
- Output still uses the UEFI text output protocol rather than a true framebuffer renderer.
