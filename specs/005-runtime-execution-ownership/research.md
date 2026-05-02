# Research: Runtime Execution Ownership

## Decision 1: Use an explicit higher-half continuation trampoline

- **Decision**: Replace the current "map whatever UEFI is executing right now"
  workaround with an explicit continuation trampoline owned by
  `src/arch/x86_64/paging.rs` that loads the runtime root and resumes at a known
  higher-half address.
- **Rationale**: The current workaround proves reachability, but it does not
  establish the intended runtime ownership model. A deterministic higher-half
  continuation is the only way to remove the low alias immediately and know
  which code and stack invariants the handoff actually depends on.
- **Alternatives considered**:
  - Keep mapping the current instruction and stack windows: Rejected because it
    bakes firmware-era placement assumptions into the runtime kernel.
  - Switch directly with no continuation contract: Rejected because it makes the
    first post-`CR3` instruction implicit and hard to validate.

## Decision 2: Remove the temporary alias immediately after the first confirmed higher-half step

- **Decision**: The kernel will remove the temporary low/current execution alias
  immediately after the first confirmed higher-half continuation step and flush
  the affected translations before continuing into later runtime setup.
- **Rationale**: This matches the clarified spec and prevents accidental hidden
  dependence on the alias during descriptor-table or idle-state work.
- **Alternatives considered**:
  - Keep the alias until GDT/IDT/TSS installation: Rejected because it hides
    whether owned execution context truly depends only on higher-half state.
  - Keep the alias until idle is proven: Rejected because it defers the most
    important ownership boundary and widens the failure surface.

## Decision 3: Install a minimal kernel-owned execution context

- **Decision**: Bring up a minimal kernel-owned execution context consisting of
  a flat kernel GDT, one TSS, one IDT, a hardware timer interrupt handler, and
  a deliberate breakpoint handler as the synchronous exception proof path.
- **Rationale**: The feature needs enough owned state to prove the kernel no
  longer depends on firmware-owned descriptor and interrupt state, but it does
  not need a full production interrupt architecture yet.
- **Alternatives considered**:
  - Install only the timer interrupt path: Rejected because it proves wakeup but
    not owned synchronous exception handling.
  - Build a broad early exception matrix now: Rejected because it increases the
    implementation surface before user mode, scheduler, and broader fault policy
    exist.

## Decision 4: Use the legacy PIC + PIT path for the first timer wakeup

- **Decision**: Use the legacy 8259 PIC plus PIT timer path for the first
  hardware interrupt-driven idle wakeup under QEMU.
- **Rationale**: It is the smallest available timer source that does not require
  APIC bring-up, SMP coordination, or more advanced platform discovery than
  this milestone needs.
- **Alternatives considered**:
  - Use the local APIC timer: Rejected because it pulls LAPIC initialization
    forward before the kernel owns the simpler legacy path.
  - Use only software-generated interrupts: Rejected because the clarified spec
    requires a real hardware timer interrupt to prove idle wakeup.

## Decision 5: Keep assembly limited to entry stubs and return-sensitive boundaries

- **Decision**: Keep new assembly limited to the pieces Rust cannot safely
  express on its own, such as a tightly controlled continuation entry stub,
  interrupt/trap entry stubs, and any `iretq`-sensitive return path.
- **Rationale**: Continuation and interrupt entry both require exact control of
  register state and stack layout. That is legitimate architecture-owned
  assembly, but anything above the entry boundary should return to Rust
  immediately.
- **Alternatives considered**:
  - Implement the entire continuation and interrupt flow in assembly: Rejected
    because it would unnecessarily broaden the low-level surface.
  - Force a Rust-only design: Rejected because entry-frame and return semantics
    are not safely expressible without architecture stubs.

## Decision 6: Validate runtime ownership through distinct transcript markers

- **Decision**: Extend the boot transcript so it proves five distinct milestones:
  higher-half entry, low-alias removal, kernel-owned descriptor installation,
  deliberate breakpoint handling, and timer-driven idle wakeup.
- **Rationale**: The current transcript already proved the CR3 switch and
  direct-map access. This feature needs equally visible proof for the new
  ownership milestones so regressions are easy to localize.
- **Alternatives considered**:
  - Rely only on "hello world" and no reboot: Rejected because it would not say
    whether failures occur in continuation, alias teardown, descriptor
    installation, or the first interrupt.
  - Rely only on host tests: Rejected because host tests cannot prove the real
    runtime ownership transitions.
