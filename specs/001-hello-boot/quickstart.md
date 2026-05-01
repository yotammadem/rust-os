# Quickstart: Hello Boot

## Prerequisites

- Rust stable toolchain installed locally with the `x86_64-unknown-uefi` target
- GRUB host tooling available locally via `x86_64-elf-grub-mkstandalone`
- QEMU installed with `qemu-system-x86_64`
- UEFI firmware file available at `/usr/local/share/qemu/edk2-x86_64-code.fd`
  or supplied through `OVMF_CODE=/path/to/firmware`

## Build

```bash
make build
```

Expected result:

- A single bootable disk image is created at `bin/hello-boot.img`
- The `bin/` directory remains generated output only

## Run

```bash
./run.sh
```

Expected result:

- `run.sh` checks that `bin/hello-boot.img` exists before launching
- QEMU boots the raw disk image using UEFI `pflash` firmware and GRUB
- The screen shows `hello world`
- The system halts and keeps the message visible until the QEMU session is closed

## Clean Rebuild

```bash
make clean
make build
./run.sh
```

## Failure Expectations

- If `bin/hello-boot.img` is missing, `run.sh` exits with a clear error and does
  not trigger a build automatically
- If the firmware path is missing, `run.sh` exits with a clear error describing
  how to provide `OVMF_CODE`
