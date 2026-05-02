# Contract: Runtime Execution Interface

## Higher-Half Continuation Contract

### Required Behavior

- The kernel exposes a runtime handoff path that activates the prepared paging
  root and resumes at a deterministic higher-half continuation address.
- The continuation path uses an explicit higher-half stack contract rather than
  preserving the current firmware-loaded execution window as a long-term
  workaround.
- The continuation path emits an observable success marker only after the first
  owned higher-half step is executing under the runtime root.

### Failure Conditions

- The continuation address is not mapped under the runtime root
- The higher-half continuation stack is not writable or not reachable
- The runtime handoff would require continued execution from the low/current
  alias after the first higher-half step

## Temporary Alias Removal Contract

### Required Behavior

- The kernel retains a temporary low/current execution alias only long enough to
  survive the root switch.
- The alias is removed immediately after the first confirmed higher-half
  continuation step.
- Alias removal flushes the affected translations before later runtime setup
  continues.

### Failure Conditions

- Removing the alias would unmap the active higher-half instruction stream
- Removing the alias would unmap the active higher-half stack
- The kernel continues into descriptor or idle setup while the alias is still
  live

## Kernel-Owned Execution Context Contract

### Required Behavior

- The kernel installs and activates its own GDT, IDT, and TSS after the
  higher-half continuation succeeds.
- The installed execution context includes at minimum a hardware timer interrupt
  path and a deliberate breakpoint exception path.
- A deliberate breakpoint exception after installation reaches a kernel-owned
  handler and returns under kernel-owned state.

### Failure Conditions

- Descriptor tables reference unmapped or low-alias-backed memory
- The kernel cannot activate the TSS or IDT under the runtime root
- A breakpoint proof path reaches firmware-owned state or resets the system

## Interrupt-Driven Idle Contract

### Required Behavior

- The post-boot idle path executes with interrupts enabled and uses `hlt` as
  the steady-state idle instruction.
- A real hardware timer interrupt wakes the CPU from idle through a
  kernel-owned handler path.
- The kernel acknowledges the timer interrupt correctly and returns to a stable
  kernel-owned runtime state after wakeup.

### Failure Conditions

- The idle path requires a blanket `cli` workaround to remain stable
- A timer interrupt during idle resets, reboots, or leaves kernel control
- The timer interrupt fires but never reaches or returns from the kernel-owned
  handler path

## Observable Validation Contract

- `cargo test` covers host-side invariants for continuation metadata, alias
  teardown ordering, descriptor ownership bookkeeping, and interrupt-proof state
- `cargo check --target x86_64-unknown-uefi` confirms the freestanding runtime
  ownership path builds cleanly
- `make build` rebuilds and stages the EFI payload with higher-half continuation
  and early interrupt ownership support
- `./run.sh` produces a QEMU transcript that proves:
  - higher-half continuation occurred
  - the low/current alias was removed
  - a kernel-owned breakpoint handler ran
  - a hardware timer interrupt woke the idle CPU through a kernel-owned handler

## Unsafe And Assembly Boundaries

- `src/arch/x86_64/paging.rs` owns the CR3-sensitive continuation boundary and
  any stack/register invariants required for the first higher-half instruction;
  the trampoline must disable interrupts, switch to a higher-half stack, and
  jump without returning through the low bootstrap path
- `src/arch/x86_64/gdt.rs` and `src/arch/x86_64/idt.rs` own descriptor
  publication, selector loading, and TSS activation invariants
- `src/arch/x86_64/interrupts.rs` and `asm/boot.s` own any unavoidable entry
  stubs, trap frames, `iretq`-sensitive transitions, and external interrupt
  acknowledgement boundaries
- `src/arch/x86_64/timer.rs` owns the PIC/PIT programming invariants used for
  the first hardware timer wakeup
