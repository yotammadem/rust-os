# Quickstart: Virtual Memory Manager

## Prerequisites

- Rust toolchain installed locally with the `x86_64-unknown-uefi` target
- GRUB host tooling available locally via `grub-mkstandalone` or
  `x86_64-elf-grub-mkstandalone`
- QEMU installed with `qemu-system-x86_64`
- UEFI firmware file available at `/usr/local/share/qemu/edk2-x86_64-code.fd`
  or supplied through `OVMF_CODE=/path/to/firmware`

## Host Validation

```bash
cargo test
cargo check --target x86_64-unknown-uefi
```

Expected result:

- Host-side tests cover kernel-template construction, process address-space
  creation, mapping conflict detection, rollback on partial allocation failure,
  and teardown/reclaim behavior
- The freestanding UEFI target still compiles after the new paging modules and
  bootstrap path are introduced

## Boot Validation

```bash
make build
./run.sh
```

Expected result:

- The EFI payload rebuilds successfully with the new paging code linked in
- QEMU still boots through GRUB and transfers control to the kernel
- The kernel prints the paging diagnostic prefix, higher-half entry target, and
  transition-alias metadata on serial before the existing `hello world` line
- Host-side paging construction, process-space creation, rollback, and reclaim
  behavior remain covered by `cargo test`

## Clean Rebuild

```bash
make clean
cargo test
make build
./run.sh
```

## Failure Expectations

- If the allocator cannot provide every page needed for a paging operation, the
  operation fails and returns all newly acquired paging pages
- If a mapping request is empty, unaligned, or conflicts with an existing
  incompatible mapping, the request is rejected without mutating the target
  address space
- If the bootstrap transition cannot keep the executing code reachable across the
  root-table switch, the kernel aborts the transition instead of continuing in a
  partially mapped state
- Destroying a process address space reclaims only its private paging pages and
  must not free shared kernel higher-half structures

## Verified Results

Verified on 2026-05-02 with the current implementation:

- `cargo test` passed with 25 host tests, including paging index, kernel
  template, rollback, kernel allocation, process isolation, and destroy-path
  coverage.
- `cargo check --target x86_64-unknown-uefi` passed.
- `make build` passed and staged the EFI tree under `.build/efi`.
- A short `./run.sh` transcript confirmed `paging root:`, the higher-half entry
  line, the transition alias line, and `hello world` on serial.
