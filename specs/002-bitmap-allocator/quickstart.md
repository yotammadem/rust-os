# Quickstart: Bitmap Allocator

## Prerequisites

- Rust toolchain installed locally with the `x86_64-unknown-uefi` target
- GRUB host tooling available locally via `x86_64-elf-grub-mkstandalone`
- QEMU installed with `qemu-system-x86_64`
- UEFI firmware file available at `/usr/local/share/qemu/edk2-x86_64-code.fd`
  or supplied through `OVMF_CODE=/path/to/firmware`

## Host Validation

```bash
cargo test
cargo check --target x86_64-unknown-uefi
```

Expected result:

- Host-side tests cover memory-region normalization, bitmap bookkeeping,
  contiguous page search, invalid free handling, and free/reuse behavior
- The kernel target still compiles in the freestanding UEFI configuration

## Boot Validation

```bash
make build
./run.sh
```

Expected result:

- The boot image rebuilds successfully at `bin/hello-boot.img`
- QEMU still boots the image through UEFI and GRUB
- Allocator initialization completes during boot without corrupting memory state
- The existing visible boot outcome remains intact after integration

## Clean Rebuild

```bash
make clean
cargo test
make build
./run.sh
```

## Failure Expectations

- If boot memory-map capture fails, allocator initialization fails
  deterministically instead of publishing partial state
- If no contiguous pages satisfy an allocation request, the allocator reports
  failure and leaves bookkeeping unchanged
- If a caller attempts an invalid free, the allocator reports failure and does
  not mutate tracked page ownership
