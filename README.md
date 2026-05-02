# rust-os

`rust-os` is a pure-Rust operating system project with a minimal boot chain based
on GRUB. The current baseline boots a UEFI image in QEMU, initializes an early
bitmap-based physical page allocator from the UEFI memory map, performs a simple
page-allocation smoke test, prints allocator-visible memory diagnostics to the
serial port, prints `hello world`, and halts.

## Status

- Current branch work: `002-bitmap-allocator`
- Current deliverable: staged x86_64 UEFI boot tree at `.build/efi/` plus an
  early physical page allocator in `src/memory/`
- Runtime flow: `make build` then `./run.sh`

## Milestones

1. **001 - Hello Boot**
   Deliver a GRUB-based x86_64 UEFI boot flow that launches through QEMU,
   prints `hello world`, and halts.
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

### 2026-05-02

- Switched boot output from UEFI text services to serial (`COM1`).
- Added boot-time allocator diagnostics and a simple allocated-page pointer smoke
  test before `hello world`.
- Replaced the duplicate raw-disk-image runtime path with a single staged EFI
  boot tree under `.build/efi/`.
- Added a Python end-to-end test that boots the staged EFI tree in QEMU and
  asserts `hello world` appears on the serial transcript.
- Detailed session reports:
  [docs/session-2026-05-02-bitmap-allocator-fix.md](/Users/yotammadem/mademos/rust-os/docs/session-2026-05-02-bitmap-allocator-fix.md)

## Project Layout

```text
Cargo.toml
Makefile
run.sh
.build/
tests/
├── host.rs
└── e2e_boot_serial.py
grub/
linker/
asm/
src/
├── memory/
specs/
```

## Build And Run

Build:

```bash
make build
```

Run:

```bash
./run.sh
```

`make build` stages the boot artifacts under `.build/efi/EFI/BOOT/`.
`./run.sh` boots that staged EFI tree directly through QEMU with serial output
attached to `stdio`.

Expected serial output now includes:

- allocator-visible available memory
- allocatable physical ranges
- `hello world`

## Tests

Run the Rust host/unit suite:

```bash
cargo test
```

Check the UEFI target still compiles:

```bash
cargo check --target x86_64-unknown-uefi
```

Run the Python end-to-end boot test:

```bash
python3 -m unittest tests.e2e_boot_serial
```

That e2e test:

- builds the EFI application
- stages `BOOTX64.EFI` and `HELLO.EFI` under `.build/efi/EFI/BOOT/`
- boots the staged tree in QEMU
- reads the serial transcript through a PTY
- asserts that `hello world` appears

## Requirements

- Rust toolchain with the `x86_64-unknown-uefi` target
- QEMU with `qemu-system-x86_64`
- GRUB host tooling via `x86_64-elf-grub-mkstandalone` or `grub-mkstandalone`
- UEFI firmware at `/usr/local/share/qemu/edk2-x86_64-code.fd`, or set
  `OVMF_CODE=/path/to/firmware`
- Python 3 for `tests/e2e_boot_serial.py`

For more detailed feature-specific validation notes, see
[specs/001-hello-boot/quickstart.md](/Users/yotammadem/mademos/rust-os/specs/001-hello-boot/quickstart.md)
and
[specs/002-bitmap-allocator/quickstart.md](/Users/yotammadem/mademos/rust-os/specs/002-bitmap-allocator/quickstart.md).
