# Quickstart: Runtime Execution Ownership

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

- Host-side tests cover higher-half continuation-plan invariants, immediate
  alias-removal ordering, descriptor-table ownership bookkeeping, breakpoint
  proof-path setup, and timer/idle state transitions
- The freestanding UEFI target still compiles after the continuation and early
  interrupt-ownership work is added

## Boot Validation

```bash
make build
./run.sh
```

Expected result:

- The EFI payload rebuilds successfully with the higher-half continuation and
  owned execution-context code linked in
- QEMU boots through GRUB and reaches the kernel
- The serial transcript still includes the earlier paging diagnostics and
  direct-map proof markers
- The transcript includes distinct markers for:
  - `boot-step: 8 higher-half-entry`
  - `boot-step: 9 transition-alias-removed`
  - kernel-owned descriptor and interrupt state installed
  - deliberate breakpoint handler entered and returned
  - hardware timer interrupt woke the idle CPU
- The kernel remains stable after the first idle wakeup without using a
  permanent `cli`/`hlt` stopgap

## Debug Boot Validation

```bash
QEMU_DEBUG_FLAGS=1 QEMU_DEBUGCON_LOG=/tmp/rust-os-debugcon.log ./run.sh
```

Expected result:

- QEMU keeps the VM from immediately rebooting on failures
- The debugcon log mirrors the early boot and post-handoff markers
- If a regression occurs, the last emitted continuation, alias, breakpoint, or
  timer marker localizes the failing ownership boundary quickly

## Clean Rebuild

```bash
make clean
cargo test
make build
./run.sh
```

## Failure Expectations

- If the higher-half continuation target or stack is not mapped correctly, the
  kernel aborts the handoff instead of continuing on the temporary alias
- If alias removal would invalidate the active higher-half execution path, the
  kernel treats the handoff as failed and emits a bounded failure marker
- If the kernel-owned GDT, IDT, or TSS cannot be installed consistently, the
  kernel does not continue into breakpoint or idle validation
- If the breakpoint proof path or timer wake path escapes kernel-owned state,
  the kernel treats runtime ownership as failed rather than silently continuing
- If the idle path cannot wake and return through the kernel-owned timer
  handler, the system must fail loudly instead of reverting to `cli` as the
  steady-state behavior

## Validation Notes

- Planned validation for this feature: `cargo test`
- Planned validation for this feature: `cargo check --target x86_64-unknown-uefi`
- Planned validation for this feature: `make build`
- Planned validation for this feature: `./run.sh`
- Planned validation for this feature: `QEMU_DEBUG_FLAGS=1 QEMU_DEBUGCON_LOG=/tmp/rust-os-debugcon.log ./run.sh`
- Current status on 2026-05-02: host tests and freestanding compilation pass, but QEMU
  still resets after `boot-step: 7 pre-activate` before reaching the first
  higher-half continuation marker
