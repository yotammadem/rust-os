# Research: Live VM Handoff

## Decision 1: Reuse the existing 4-level x86_64 paging model and make it hardware-consumable

- **Decision**: Keep the existing four-level 4 KiB paging structure model from
  the prior feature, but convert allocator-backed table pages into real
  processor-consumable page tables written into RAM.
- **Rationale**: The previous feature already established the ownership and
  rollback model. Reusing that shape preserves host-test coverage and narrows
  the new work to the live publication path instead of changing the address-space
  model itself.
- **Alternatives considered**:
  - Redesign the paging subsystem around a different abstraction: Rejected
    because it would discard the useful groundwork from `003`.
  - Introduce huge pages now: Rejected because it complicates the first live
    bootstrap path and provides no required value for this milestone.

## Decision 2: Add a full direct-map window for allocator-managed RAM

- **Decision**: Reserve a higher-half direct-map virtual region that covers all
  physical RAM managed by the bitmap allocator and provide helpers for
  physical-to-direct-map and direct-map-to-physical translation.
- **Rationale**: The kernel needs a stable way to initialize page-table pages
  and later allocator-backed pages after the root switch. A direct map avoids
  temporary per-page mappings and simplifies later allocator and VMM work.
- **Alternatives considered**:
  - Map only selected physical pages on demand: Rejected because it adds mapping
    churn and complexity to low-level memory initialization.
  - Continue writing through raw physical addresses: Rejected because that is
    not reliable once the kernel fully owns its virtual address space.

## Decision 3: Use a temporary low bootstrap alias only for the active execution path

- **Decision**: Keep a temporary low-address alias only for the currently
  executing bootstrap code, stack, and any immediately required data needed to
  survive the CR3 switch into the higher half.
- **Rationale**: The CPU must continue fetching instructions and touching the
  active stack across the root change. Mapping the full low address space is not
  required, and retaining the alias after the handoff would violate the feature
  requirement.
- **Alternatives considered**:
  - Keep a permanent identity map: Rejected because the spec explicitly forbids
    it after the switch.
  - Switch directly without any low alias: Rejected because the current
    instruction stream would not remain valid across the handoff.

## Decision 4: Keep CR3 loading and continuation logic in the architecture layer

- **Decision**: Confine CR3 loading, any required TLB-sensitive transition
  logic, and the higher-half continuation entry path to `src/arch/x86_64/paging.rs`.
- **Rationale**: This keeps architecture-sensitive behavior away from the generic
  paging and allocator logic and matches the constitution's containment rule for
  inline assembly and unsafe boundaries.
- **Alternatives considered**:
  - Drive the whole switch from generic memory code: Rejected because the
    architecture-specific invariants would leak into the generic layer.
  - Introduce a standalone assembly trampoline file: Rejected unless inline
    assembly and Rust control flow are insufficient, because a new file would
    broaden the low-level surface.

## Decision 5: Validate success by post-switch behavior, not just by table construction

- **Decision**: Treat successful post-switch serial output and a direct-map
  physical-page smoke test as the primary runtime proof that the live kernel
  address space is active.
- **Rationale**: Table construction alone only proves preparation. A CR3 switch
  is only meaningful if the kernel can continue running and touch allocator-owned
  physical memory through the new virtual layout afterward.
- **Alternatives considered**:
  - Prove success only with host tests: Rejected because host tests cannot prove
    the real root switch.
  - Prove success only by printing diagnostics before the switch: Rejected
    because that does not distinguish the old and new translation states.

## Decision 6: Keep process roots derived from the live kernel template

- **Decision**: Continue building process address spaces from a shared kernel
  template, but define that template from the live higher-half and direct-map
  layout rather than from the earlier software-only model.
- **Rationale**: Process roots should inherit the exact kernel mappings the
  running kernel uses, not a separate planning artifact.
- **Alternatives considered**:
  - Delay process-root support to a later feature: Rejected because the current
    spec still requires it, and the groundwork already exists.
  - Clone the entire live kernel tree per process: Rejected because it wastes
    memory and obscures shared ownership.
