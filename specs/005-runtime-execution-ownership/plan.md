# Implementation Plan: Runtime Execution Ownership

**Branch**: `005-runtime-execution-ownership` | **Date**: 2026-05-02 | **Spec**: [specs/005-runtime-execution-ownership/spec.md](./spec.md)
**Input**: Feature specification from `/specs/005-runtime-execution-ownership/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

Replace the current bootstrap reachability workaround with a real higher-half
continuation trampoline, delete the temporary low/current execution alias as
soon as higher-half execution is proven, install kernel-owned GDT/IDT/TSS
state, and switch the post-boot idle path from `cli`/`hlt` to an
interrupt-driven `sti; hlt` flow proven by a kernel-handled hardware timer
interrupt and a deliberate breakpoint exception.

## Technical Context

**Language/Version**: Rust 2024 with `no_std` on the `x86_64-unknown-uefi` target  
**Primary Dependencies**: No third-party code; Rust toolchain crates plus the existing GRUB and UEFI/QEMU host tooling only  
**Storage**: N/A for persistent storage; continuation metadata, descriptor tables, interrupt tables, and bootstrap stacks live in RAM  
**Testing**: `cargo test`, `cargo check --target x86_64-unknown-uefi`, `make build`, and `./run.sh` with transcript inspection  
**Target Platform**: x86_64 UEFI kernel booted through GRUB in QEMU on macOS  
**Project Type**: Bare-metal operating system kernel bootstrap, paging handoff, and early interrupt-ownership work  
**Performance Goals**: Keep the continuation handoff bounded to a constant number of page-table and register transitions, keep alias teardown immediate after higher-half confirmation, and keep idle wakeup latency low enough to observe repeated timer-driven wakeups in QEMU without reset or stall  
**Constraints**: No new dependencies, single-processor boot only, 4 KiB pages only, no user mode yet, no APIC or SMP bring-up in this milestone, and any assembly or `unsafe` must stay narrowly contained in architecture-owned modules  
**Scale/Scope**: One kernel runtime root, one higher-half continuation trampoline, one temporary alias teardown path, one kernel-owned GDT/IDT/TSS installation path, one hardware timer interrupt source, one deliberate breakpoint proof path, and one stable interrupt-driven idle loop

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] No third-party dependencies are introduced; the plan stays within Rust toolchain crates, GRUB, and existing host tools.
- [x] Any GRUB integration change is explicitly described and justified; GRUB still only loads the EFI payload while the kernel takes over execution ownership after paging activation.
- [x] Any new assembly is the minimum required surface and includes a justification; assembly remains limited to the continuation-sensitive and interrupt-entry-sensitive boundaries that Rust cannot express safely on its own.
- [x] Any new or expanded `unsafe` boundary documents its invariants and safe wrapper plan; CR3-sensitive continuation, descriptor publication, interrupt return frames, and MMIO/port I/O stay isolated under architecture modules.
- [x] Validation covers all affected boot or kernel-critical paths with reproducible commands; host tests, freestanding target checks, image build, and QEMU transcript validation remain required.

Post-design re-check: Passed. The design keeps the zero-dependency policy,
limits assembly and `unsafe` to hardware-owned boundaries, and defines a
reproducible boot transcript for continuation, alias removal, owned interrupt
state, breakpoint handling, and interrupt-driven idle.

## Project Structure

### Documentation (this feature)

```text
specs/005-runtime-execution-ownership/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── runtime-execution-interface.md
└── tasks.md
```

### Source Code (repository root)

```text
Cargo.toml
Makefile
run.sh
AGENTS.md
src/
├── lib.rs
├── main.rs
├── boot/
│   ├── mod.rs
│   └── uefi.rs
├── arch/
│   ├── mod.rs
│   └── x86_64/
│       ├── debugcon.rs
│       ├── framebuffer.rs
│       ├── halt.rs
│       ├── mod.rs
│       ├── paging.rs
│       ├── serial.rs
│       ├── gdt.rs              # new
│       ├── idt.rs              # new
│       ├── interrupts.rs       # new
│       └── timer.rs            # new
├── kernel/
│   ├── hello.rs
│   └── mod.rs
└── memory/
    ├── bitmap.rs
    ├── map.rs
    ├── mod.rs
    └── paging/
        ├── address_space.rs
        ├── mapper.rs
        ├── mod.rs
        └── table.rs
tests/
├── host.rs
├── e2e_boot_serial.py
└── host/
    ├── allocator.rs
    ├── boot_memory_map.rs
    ├── build_artifact.rs
    ├── framebuffer_console.rs
    ├── paging.rs
    ├── run_contract.rs
    └── serial_console.rs
asm/
├── boot.s
linker/
grub/
```

**Structure Decision**: Keep higher-half address-space construction in
`src/memory/paging/`, but move all continuation, descriptor-table ownership,
interrupt entry, PIC/PIT programming, and idle wakeup behavior into
`src/arch/x86_64/`. Boot orchestration and transcript markers remain in
`src/main.rs`, while transcript-based proof stays in `tests/e2e_boot_serial.py`
and new host-side invariants live under `tests/host/`.

## Phase 0: Research Summary

- Use an explicit higher-half continuation trampoline rather than preserving the
  live low execution window.
- Tear down the transition alias immediately after the first confirmed
  higher-half instruction and flush the alias translations before continuing.
- Install a minimal kernel-owned execution context: flat kernel GDT, one TSS,
  one bootstrap IST-capable stack policy, and an IDT that proves ownership via
  a timer IRQ and a breakpoint exception.
- Use the legacy PIC + PIT path for the first hardware timer wakeup because it
  is the smallest interrupt source already available under QEMU without APIC
  bring-up.
- Validate owned runtime state through transcript markers that distinguish
  higher-half entry, alias teardown, descriptor installation, breakpoint
  handling, timer wakeup, and steady idle.

## Phase 1: Design Artifacts

- `research.md`: Captures the continuation, alias-teardown, descriptor-state,
  timer-source, and idle-path decisions with rejected alternatives.
- `data-model.md`: Defines the continuation window, alias-removal contract,
  kernel execution context, interrupt proof events, and idle lifecycle states.
- `contracts/runtime-execution-interface.md`: Defines observable runtime
  behavior for continuation, alias teardown, interrupt ownership, breakpoint
  handling, and timer-driven idle wakeups.
- `quickstart.md`: Documents host validation, boot validation, expected
  transcript markers, and failure expectations for this feature.

## Implementation Notes

- `src/main.rs` will stop mapping the current low instruction and stack windows
  as a general workaround and instead hand an explicit continuation target and
  bootstrap stack contract into the architecture layer.
- `src/arch/x86_64/paging.rs` will grow from a raw CR3 loader into the owned
  continuation entry point that activates the runtime root and transfers
  execution to the higher half.
- `src/arch/x86_64/gdt.rs`, `idt.rs`, and `interrupts.rs` will own descriptor
  publication, handler registration, and proof paths for the timer IRQ and the
  breakpoint exception.
- `src/arch/x86_64/timer.rs` will program the legacy timer source and deliver an
  interrupt path compatible with the early kernel-owned IDT.
- `asm/boot.s` may remain the home for any minimal naked stubs that cannot be
  expressed safely in Rust, but new assembly should only exist where `iretq`,
  segment reload, or precise entry-frame layout require it.

## Complexity Tracking

No constitution violations are expected. The main complexity is that this
feature touches the two most fragile ownership boundaries in the current boot
path at once: the first instruction after `mov cr3`, and the first interrupt
after the kernel stops relying on firmware state. The design therefore keeps the
proof surface intentionally narrow: one continuation jump, one alias-removal
moment, one breakpoint proof, one timer IRQ proof, and one stable idle loop.
