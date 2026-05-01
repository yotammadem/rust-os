# rust-os

`rust-os` is a pure-Rust operating system project with a minimal boot chain based
on GRUB. The current baseline boots a UEFI image in QEMU, prints `hello world`,
halts, and initializes an early bitmap-based physical page allocator from the
UEFI memory map.

## Status

- Current branch work: `002-bitmap-allocator`
- Current deliverable: bootable x86_64 UEFI disk image at `bin/hello-boot.img`
  plus an early physical page allocator in `src/memory/`
- Runtime flow: `make build` then `./run.sh`

## Milestones

1. **001 - Hello Boot**
   Deliver a GRUB-based x86_64 UEFI boot image that builds into `bin/`,
   launches through QEMU, prints `hello world`, and halts.
2. **002 - Bitmap Allocator**
   Initialize a 4 KiB physical page allocator from boot-provided UEFI memory
   data, reserve in-memory bitmap metadata, and expose contiguous allocation and
   free operations.

## Changelog

### 2026-05-01

- Added the first working boot milestone, `001 - Hello Boot`.
- Introduced the Rust UEFI application, GRUB boot image assembly, QEMU run flow,
  and host-side validation tests.
- Added planning and implementation artifacts for the first feature under
  `specs/001-hello-boot/`.
- Detailed session report: [docs/session-2026-05-01-hello-boot.md](/Users/yotammadem/mademos/rust-os/docs/session-2026-05-01-hello-boot.md)

### 2026-05-01 (Session 2)

- Added the second milestone, `002 - Bitmap Allocator`, under
  `specs/002-bitmap-allocator/`.
- Added a UEFI-memory-map-backed bitmap allocator in `src/memory/` and wired its
  initialization into the existing boot path.
- Detailed session report: [docs/session-2026-05-01-bitmap-allocator.md](/Users/yotammadem/mademos/rust-os/docs/session-2026-05-01-bitmap-allocator.md)

## Project Layout

```text
Cargo.toml
Makefile
run.sh
grub/
linker/
asm/
src/
├── memory/
tests/
specs/
```

## Quickstart

Build:

```bash
make build
```

Run:

```bash
./run.sh
```

For the exact verified host prerequisites and behavior, see
[specs/001-hello-boot/quickstart.md](/Users/yotammadem/mademos/rust-os/specs/001-hello-boot/quickstart.md)
and
[specs/002-bitmap-allocator/quickstart.md](/Users/yotammadem/mademos/rust-os/specs/002-bitmap-allocator/quickstart.md).
