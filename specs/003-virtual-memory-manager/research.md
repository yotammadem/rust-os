# Research: Virtual Memory Manager

## Decision 1: Use the x86_64 four-level 4 KiB paging model

- **Decision**: Represent each address space as a 4-level x86_64 paging
  hierarchy rooted at one top-level table page, with all derived structures
  allocated in 4 KiB physical pages from the existing bitmap allocator.
- **Rationale**: The current kernel already assumes 4 KiB physical pages and
  targets x86_64 UEFI. Matching the architecture's standard 4-level layout keeps
  address translation predictable and avoids premature huge-page policy.
- **Alternatives considered**:
  - Model paging generically without x86_64-specific levels: Rejected because
    the current milestone is architecture-specific and must manipulate real page
    table entries.
  - Introduce huge-page support now: Rejected because it complicates allocation,
    teardown, and conflict rules before the first complete paging subsystem
    exists.

## Decision 2: Build one shared kernel higher-half mapping template

- **Decision**: Construct the kernel address space first, then reuse its
  higher-half kernel mappings as the shared kernel portion of every new process
  address space while allocating private lower-half structures per process.
- **Rationale**: The spec requires shared kernel mappings and isolated private
  process mappings. A reusable kernel template makes that split explicit and
  avoids mutating one process when another grows.
- **Alternatives considered**:
  - Clone the entire kernel hierarchy into every process: Rejected because it
    duplicates paging pages unnecessarily and makes kernel-map updates harder to
    reason about.
  - Keep one global address space only: Rejected because the feature must create
    fresh process roots independently of the kernel root.

## Decision 3: Bootstrap with a temporary low-address transition alias, then remove it

- **Decision**: When activating the kernel-owned paging hierarchy, include the
  currently executing bootstrap code in both its current low address and the
  target higher-half address long enough to load the new root and branch into the
  higher-half continuation, then remove the low-address alias so no identity
  mapping remains after the transition completes.
- **Rationale**: Loading a new page-table root while already executing requires
  the next instruction fetch to remain valid. A temporary alias satisfies the
  hardware constraint without violating the clarified requirement that no
  identity mapping is retained after virtual memory is enabled.
- **Alternatives considered**:
  - Keep a permanent identity mapping for safety: Rejected by clarification.
  - Switch directly to a page-table root that maps only the higher-half address:
    Rejected because the current instruction pointer would fault before control
    reaches the higher-half continuation.

## Decision 4: Allocate paging pages through the bitmap allocator with rollback guards

- **Decision**: All new page-directory and page-table pages come from
  `BitmapAllocator`, and multi-step address-space creation or mapping operations
  use an ownership record that can roll back newly acquired pages on failure.
- **Rationale**: The bitmap allocator is the single source of physical memory
  truth in this codebase. Explicit rollback is required to satisfy the spec's
  clean-failure and no-leak requirements.
- **Alternatives considered**:
  - Add a separate bootstrap allocator for paging pages: Rejected because it
    would split ownership and bypass the current allocator contract.
  - Leak partially allocated paging pages on failure and clean them later:
    Rejected because it violates deterministic failure behavior.

## Decision 5: Avoid a fixed VMM-private paging pool

- **Decision**: Do not reserve a large private buffer for future page
  directories or page tables. Instead, allocate paging-structure pages on demand
  and record ownership per address space or shared kernel template.
- **Rationale**: A fixed pool simplifies the first bring-up a little, but it
  introduces waste and an artificial upper limit on address-space growth. The
  existing bitmap allocator already provides deterministic page ownership, so
  explicit tracking is the cleaner long-term boundary.
- **Alternatives considered**:
  - Preallocate a fixed page-table buffer at bootstrap: Rejected because it
    wastes memory and makes future scaling depend on an arbitrary cap.
  - Keep a tiny permanent reserve for all later paging work: Rejected because it
    obscures ownership and is unnecessary once allocator-backed tracking exists.

## Decision 6: Track address-space-owned paging pages explicitly for teardown

- **Decision**: Each address space records which physical pages it owns for root
  and derived paging structures so teardown can walk only that ownership set and
  return pages to the allocator safely.
- **Rationale**: The system must reclaim paging pages without scanning unrelated
  physical memory or risking double free across shared kernel mappings.
- **Alternatives considered**:
  - Reconstruct ownership only by re-walking page tables at destroy time:
    Rejected because shared kernel mappings make ownership inference harder and
    more error-prone.
  - Treat all kernel-mapped paging pages as permanently reserved: Rejected
    because process-private structures must be reclaimable.

## Decision 7: Separate explicit mappings from VMM-owned kernel allocations

- **Decision**: Keep two API modes: one operation maps caller-supplied physical
  pages into an address space, and a separate kernel allocation operation both
  allocates physical backing pages and maps them into the higher-half kernel
  space.
- **Rationale**: This keeps the boundary clear between generic mapping and
  kernel-owned virtual allocations. It also avoids overloading one mapping API
  with hidden backing-page allocation behavior.
- **Alternatives considered**:
  - Make every mapping operation allocate backing pages implicitly: Rejected
    because process and bootstrap mappings often need explicit physical targets.
  - Force the caller to always allocate and map separately, even for kernel
    higher-half memory: Rejected because the VMM itself needs a convenient path
    for allocator-backed kernel virtual allocations.

## Decision 8: Keep control-register and page-table writes behind narrow unsafe boundaries

- **Decision**: Confine raw page-table entry mutation to the memory paging
  modules and confine page-table-root activation to an x86_64-owned path that
  encapsulates any required inline assembly or privileged register access.
- **Rationale**: This matches the constitution's unsafe-containment rule and
  keeps the majority of address-space construction testable as safe Rust.
- **Alternatives considered**:
  - Spread `unsafe` writes through kernel bring-up code: Rejected because it
    weakens auditability and makes rollback invariants harder to review.
  - Encode page-table activation in a standalone assembly file: Rejected unless
    Rust inline assembly proves insufficient, because no broader assembly surface
    is needed for this milestone.

## Decision 9: Validate in three layers: host paging logic, freestanding build, boot transition

- **Decision**: Validate page-table construction, mapping conflict handling,
  rollback, and teardown with host-side Rust tests; validate target integration
  with `cargo check --target x86_64-unknown-uefi`; then validate bring-up with
  `make build` and `./run.sh`.
- **Rationale**: Most correctness rules are deterministic and should be covered
  in host tests, while the higher-half activation path still requires real boot
  integration coverage.
- **Alternatives considered**:
  - Boot-only validation: Rejected because it is too slow and opaque for mapping
    edge cases.
  - Host-only validation: Rejected because it would not prove that the kernel
    still boots through the actual GRUB + UEFI path after paging ownership moves
    into the kernel.
