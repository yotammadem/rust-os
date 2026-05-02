# Quickstart: Live VM Handoff

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

- Host-side tests cover direct-map translation, live paging-page initialization
  invariants, rollback on partial failure, bootstrap alias reachability rules,
  and process address-space isolation/reclaim behavior
- The freestanding UEFI target still compiles after the live handoff path is
  introduced

## Boot Validation

```bash
make build
./run.sh
```

Expected result:

- The EFI payload rebuilds successfully with the live handoff code linked in
- QEMU boots through GRUB and reaches the kernel
- The kernel loads a kernel-owned CR3 root, continues at the higher-half
  continuation point, and prints `hello world` after the switch
- The kernel allocates a physical page after the switch, reaches it through the
  direct-map virtual range, verifies the test pattern round-trip, and releases
  the page successfully

## Clean Rebuild

```bash
make clean
cargo test
make build
./run.sh
```

## Failure Expectations

- If the allocator cannot provide every required paging page, the handoff
  aborts without leaking paging ownership
- If the bootstrap alias does not cover the active execution path or stack, the
  kernel aborts the switch instead of attempting a partial handoff
- If the direct-map translation helpers receive an out-of-range physical
  address, the request is rejected without mutating runtime mappings
- If the direct-map smoke test cannot verify a write/read round-trip, the
  kernel treats the handoff as failed rather than continuing on an invalid
  memory-access path
