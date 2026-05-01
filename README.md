# rust-os

`rust-os` is a pure-Rust operating system project with a minimal boot chain based
on GRUB. The current baseline boots a UEFI image in QEMU, prints `hello world`,
and halts.

## Status

- Current branch work: `001-hello-boot`
- Current deliverable: bootable x86_64 UEFI disk image at `bin/hello-boot.img`
- Runtime flow: `make build` then `./run.sh`

## Milestones

1. **001 - Hello Boot**
   Deliver a GRUB-based x86_64 UEFI boot image that builds into `bin/`,
   launches through QEMU, prints `hello world`, and halts.

## Changelog

### 2026-05-01

- Added the first working boot milestone, `001 - Hello Boot`.
- Introduced the Rust UEFI application, GRUB boot image assembly, QEMU run flow,
  and host-side validation tests.
- Added planning and implementation artifacts for the first feature under
  `specs/001-hello-boot/`.
- Detailed session report: [docs/session-2026-05-01-hello-boot.md](/Users/yotammadem/mademos/rust-os/docs/session-2026-05-01-hello-boot.md)

## Project Layout

```text
Cargo.toml
Makefile
run.sh
grub/
linker/
asm/
src/
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
[specs/001-hello-boot/quickstart.md](/Users/yotammadem/mademos/rust-os/specs/001-hello-boot/quickstart.md).
