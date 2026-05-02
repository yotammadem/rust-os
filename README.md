# rust-os

`rust-os` is a pure-Rust x86_64 operating system project with a minimal GRUB
boot chain. The current implemented baseline boots a UEFI image in QEMU,
captures the boot memory map, initializes a bitmap-backed physical page
allocator, builds allocator-owned x86_64 page tables, switches into a
kernel-owned higher-half runtime root, proves direct-map access through the new
CR3, prints `hello world` to serial, and halts.

## Status

- Active branch: `main`
- Latest implemented milestone: `004-live-vm-handoff`
- Next planned milestone: `005-runtime-execution-ownership` (spec drafted, not
  implemented)
- Runtime flow: `make build` then `./run.sh`

## Milestones

1. **001 - Hello Boot**
   Deliver a GRUB-based x86_64 UEFI boot flow that launches through QEMU,
   prints `hello world`, and halts.
2. **002 - Bitmap Allocator**
   Initialize a 4 KiB physical page allocator from boot-provided UEFI memory
   data, reserve in-memory bitmap metadata, and expose contiguous allocation and
   free operations.
3. **003 - Virtual Memory Manager**
   Build allocator-backed page-table hierarchies, model a higher-half kernel
   address space, and support process address spaces that share kernel mappings
   while keeping private mappings isolated.
4. **004 - Live VM Handoff**
   Convert the paging model into live hardware page tables, install a
   kernel-owned CR3 root, keep a temporary transition alias during the switch,
   and prove post-switch direct-map access from the active runtime root.
5. **005 - Runtime Execution Ownership**
   Planned next step: higher-half continuation without the temporary low alias,
   kernel-owned GDT/IDT/TSS, and an interrupt-safe idle path.

## Current Boot Transcript

Successful boots emit serial markers that prove the runtime handoff:

- `boot-step:` progress markers through EFI entry, allocator setup, paging
  preparation, CR3 activation, and post-switch validation
- `paging root:` diagnostics for the runtime paging root and managed physical
  memory window
- `direct-map smoke:` proof that allocator-owned physical memory is reachable
  through the kernel direct-map region after the handoff
- `hello world`

## Project Layout

```text
Cargo.toml
Makefile
run.sh
.build/
asm/
grub/
linker/
src/
├── arch/x86_64/
├── boot/
├── kernel/
└── memory/
    └── paging/
tests/
├── e2e_boot_serial.py
├── host.rs
└── host/
specs/
├── 001-hello-boot/
├── 002-bitmap-allocator/
├── 003-virtual-memory-manager/
├── 004-live-vm-handoff/
└── 005-runtime-execution-ownership/
docs/
```

## Build And Run

Build the staged EFI tree:

```bash
make build
```

Run it in QEMU:

```bash
./run.sh
```

`make build` stages the boot artifacts under `.build/efi/EFI/BOOT/`.
`./run.sh` boots that staged EFI tree directly through QEMU with serial output
attached to `stdio`.

Optional runtime flags:

- `OVMF_CODE=/path/to/firmware ./run.sh` to override the UEFI firmware image
- `QEMU_DEBUG_FLAGS=1 ./run.sh` to run QEMU with `-no-reboot`, `-no-shutdown`,
  and extra debug tracing
- `QEMU_DEBUGCON_LOG=/tmp/debugcon.log ./run.sh` to capture early debug console
  output

## Tests

Run the host-side Rust test suite:

```bash
cargo test
```

Check the freestanding UEFI target still compiles:

```bash
cargo check --target x86_64-unknown-uefi
```

Run the end-to-end boot test:

```bash
python3 -m unittest tests.e2e_boot_serial
```

The e2e test rebuilds the EFI artifacts, boots QEMU, reads the serial
transcript through a PTY, and asserts that `paging root:`, `direct-map smoke:`,
and `hello world` all appear.

## Requirements

- Rust toolchain with the `x86_64-unknown-uefi` target
- QEMU with `qemu-system-x86_64`
- GRUB host tooling via `grub-mkstandalone` or
  `x86_64-elf-grub-mkstandalone`
- UEFI firmware at `/usr/local/share/qemu/edk2-x86_64-code.fd`, or set
  `OVMF_CODE=/path/to/firmware`
- Python 3 for `tests/e2e_boot_serial.py`

## Specs And Session Notes

Feature artifacts live under:

- [specs/001-hello-boot/](/Users/yotammadem/mademos/rust-os/specs/001-hello-boot/)
- [specs/002-bitmap-allocator/](/Users/yotammadem/mademos/rust-os/specs/002-bitmap-allocator/)
- [specs/003-virtual-memory-manager/](/Users/yotammadem/mademos/rust-os/specs/003-virtual-memory-manager/)
- [specs/004-live-vm-handoff/](/Users/yotammadem/mademos/rust-os/specs/004-live-vm-handoff/)
- [specs/005-runtime-execution-ownership/](/Users/yotammadem/mademos/rust-os/specs/005-runtime-execution-ownership/)

Session reports:

- [docs/session-2026-05-01-hello-boot.md](/Users/yotammadem/mademos/rust-os/docs/session-2026-05-01-hello-boot.md)
- [docs/session-2026-05-01-bitmap-allocator.md](/Users/yotammadem/mademos/rust-os/docs/session-2026-05-01-bitmap-allocator.md)
- [docs/session-2026-05-02-bitmap-allocator-fix.md](/Users/yotammadem/mademos/rust-os/docs/session-2026-05-02-bitmap-allocator-fix.md)
