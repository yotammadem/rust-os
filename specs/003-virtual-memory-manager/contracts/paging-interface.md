# Contract: Virtual Memory Manager Interface

## Kernel Paging Bootstrap Interface

### Required Behavior

- The kernel exposes an initialization path that consumes the existing boot
  memory snapshot and a ready physical page allocator.
- Initialization can build an allocator-backed model of the kernel's owned
  higher-half address space, including the temporary transition alias metadata
  needed for a future root-table switch.
- The architecture layer exposes a narrow activation surface
  (`src/arch/x86_64/paging.rs`) that owns page-table-root loading separately
  from the generic mapping logic.
- The current boot integration emits deterministic paging diagnostics during EFI
  startup while keeping the modeled paging structures isolated from live runtime
  memory until the higher-half handoff is implemented end-to-end.

### Failure Conditions

- The allocator cannot supply every required paging-structure page
- Required kernel execution regions cannot be represented in the higher-half
  layout
- The transition path cannot guarantee a valid instruction stream across the
  root switch

## Address Space Creation Interface

### Required Behavior

- The virtual memory manager exposes an operation that creates a fresh process
  address space with a distinct root paging structure.
- Every new process address space includes the shared kernel higher-half mapping.
- Process-private lower-half paging structures are allocated only as needed for
  that process.
- The current implementation will expose this behavior through paging and
  address-space modules under `src/memory/paging/`.

### Failure Conditions

- The allocator cannot provide the root table page or any required derived page
- Shared kernel mappings cannot be installed consistently into the new process
  root
- Private mappings requested during creation conflict with reserved ranges

## Mapping Update Interface

### Required Behavior

- The manager exposes a mapping operation that validates page alignment, rejects
  empty requests, walks the paging hierarchy, and allocates only the missing
  derived paging-structure pages required for the request.
- This mapping operation accepts a caller-supplied physical backing range; it
  does not implicitly allocate the mapped payload pages on behalf of the caller.
- Successful mapping publishes the final entries only after all required
  intermediate paging pages have been acquired.
- Failed mapping attempts leave the address space and allocator in a consistent
  state.

### Failure Conditions

- The request range is empty, unaligned, or exceeds the supported address-space
  layout
- The request conflicts with an existing incompatible mapping
- Paging-structure allocation fails before the request can be published safely

## Kernel Virtual Allocation Interface

### Required Behavior

- The manager exposes a kernel-owned allocation operation that obtains physical
  backing pages from the bitmap allocator, creates any required paging
  structures, maps the allocation into the kernel higher-half virtual space, and
  returns the resulting virtual range.
- This operation tracks both the backing physical pages and the paging-structure
  pages needed to support the mapping.
- The manager does not depend on a fixed preallocated VMM-only page-table pool;
  paging structures are allocated on demand and tracked explicitly.

### Failure Conditions

- The allocator cannot provide either the backing physical pages or the required
  paging-structure pages
- The requested kernel virtual range is invalid, unavailable, or conflicts with
  an existing mapping
- The operation fails after partial allocation and cannot publish the new
  mapping safely, in which case all newly acquired pages must be returned or
  retained in a defined consistent state

## Teardown Interface

### Required Behavior

- The manager exposes an address-space destroy operation that releases all
  private paging-structure pages owned by the target address space back to the
  bitmap allocator.
- Any VMM-owned kernel virtual allocations created through the kernel allocation
  interface must be reclaimable through the same allocator-backed ownership
  records.
- Teardown preserves shared kernel higher-half paging pages and only reclaims
  process-private ownership.

### Failure Conditions

- Ownership records are incomplete or inconsistent
- A destroy request targets an address space that still appears active in the
  architecture-owned execution context

## Observable Validation Contract

- `cargo test` covers page-table allocation, kernel-template reuse, process
  address-space independence, mapping conflict handling, rollback on failure, and
  paging-page reclamation
- `cargo check --target x86_64-unknown-uefi` confirms the new paging modules and
  bootstrap path remain valid in the freestanding target
- `make build` and `./run.sh` confirm the kernel still boots through the current
  GRUB + UEFI path and prints the paging diagnostic surface plus the existing
  serial hello-world line
