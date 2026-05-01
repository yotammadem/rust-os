# Research: Bitmap Allocator

## Decision 1: Source boot memory from the UEFI memory map in the GRUB boot flow

- **Decision**: Capture the UEFI memory map from the boot services table during
  early kernel entry, then normalize it into the allocator's `MemoryRegion`
  model before any page allocation is allowed.
- **Rationale**: The current kernel is a UEFI application launched by GRUB, so
  the UEFI memory map is the real authoritative boot-time memory source already
  available in this architecture. Reusing that path avoids a disruptive boot
  protocol switch while still satisfying the requirement to build allocator state
  from boot-provided memory information.
- **Alternatives considered**:
  - Switch the kernel to GRUB Multiboot2 memory tags now: Rejected because it
    would expand the feature into a boot-protocol migration instead of an
    allocator milestone.
  - Hardcode a memory layout for QEMU: Rejected because it would violate the
    spec's requirement to derive availability from real boot data.

## Decision 2: Standardize on 4 KiB physical pages

- **Decision**: Track and allocate memory in 4 KiB page frames.
- **Rationale**: 4 KiB is the x86_64 baseline page size, matches early kernel
  expectations, and keeps bitmap math simple and testable for the first
  allocator milestone.
- **Alternatives considered**:
  - Make page size configurable at runtime: Rejected because no existing part of
    the kernel needs that flexibility yet.
  - Use large pages in the allocator core: Rejected because it adds policy and
    fragmentation questions before basic page ownership exists.

## Decision 3: Reserve allocator bitmap storage from usable physical pages

- **Decision**: During initialization, choose a usable page-aligned physical
  range large enough to hold the bitmap metadata, mark those pages unavailable,
  and build the bitmap in place there.
- **Rationale**: The allocator must not depend on a heap that does not exist yet,
  and bitmap state must represent the same physical memory it governs. Reserving
  physical pages during init makes the ownership model explicit and testable.
- **Alternatives considered**:
  - Store the bitmap in a fixed static Rust array: Rejected because the maximum
    managed memory size would become artificially capped by compile-time storage.
  - Leave bitmap placement to a later bootstrap allocator: Rejected because this
    feature is the first allocator and has to bootstrap itself.

## Decision 4: Expose contiguous allocation with explicit failure results

- **Decision**: The allocator API will support single-page and contiguous
  multi-page allocation, reject zero-page requests, and return explicit failure
  for out-of-memory or invalid-free conditions without mutating state.
- **Rationale**: The clarification session already established contiguity and
  no-op-free rejection semantics. Making zero-page requests fail keeps the API
  unambiguous and simplifies tests for first-fit search and rollback behavior.
- **Alternatives considered**:
  - Non-contiguous allocation sets: Rejected by clarification and because they
    complicate the first allocator API.
  - Silent ignore on invalid free: Rejected by clarification because it can hide
    ownership bugs in later kernel code.

## Decision 5: Keep the allocator logic pure Rust and assembly-free

- **Decision**: Implement the allocator, memory-map normalization, and page-span
  bookkeeping entirely in Rust with no new assembly.
- **Rationale**: This feature only needs parsing, address arithmetic, and bitmap
  updates, all of which Rust can express directly. That fits the constitution's
  Rust-first and assembly-minimization rules.
- **Alternatives considered**:
  - Add assembly helpers for page scanning or bootstrap setup: Rejected as
    unnecessary complexity with no hardware requirement.

## Decision 6: Validate in two layers: host logic and boot integration

- **Decision**: Validate pure allocator behavior with host-side Rust tests, then
  confirm boot integration by rebuilding and booting the existing image through
  `make build` and `./run.sh`.
- **Rationale**: The bitmap search, normalization, and free/reuse logic are
  deterministic and should be covered by automated tests. Boot validation still
  matters because the allocator depends on early UEFI memory-map capture and must
  not regress the current boot flow.
- **Alternatives considered**:
  - Boot-only manual validation: Rejected because allocator invariants are easier
    and faster to prove with host-side tests.
  - Host-only validation: Rejected because it would miss boot-time memory-map and
    metadata-reservation integration errors.
