# Contract: Bitmap Allocator Interface

## Boot Initialization Interface

### Required Behavior

- The kernel exposes an initialization path that accepts boot-provided memory
  information from the UEFI launch environment.
- Initialization normalizes memory ranges into 4 KiB pages, reserves bitmap
  metadata pages, and publishes one ready allocator instance.
- Initialization must fail deterministically if no usable pages remain after
  reserving kernel, boot, and allocator-owned memory.

### Failure Conditions

- Boot memory metadata cannot be read safely
- No page-aligned usable memory remains for allocator management
- Bitmap metadata cannot be placed without overlapping reserved or kernel pages

## Allocation Interface

### Required Behavior

- The allocator exposes a single-page allocation operation.
- The allocator exposes a contiguous multi-page allocation operation.
- Successful allocation returns one `PageSpan` and marks every page in that span
  used before control returns to the caller.
- A zero-page or oversized request returns failure without mutating allocator
  state.
- The current implementation exposes this behavior through `BitmapAllocator`
  methods in `src/memory/bitmap.rs`.

### Failure Conditions

- No free page exists for a single-page request
- No contiguous free run satisfies the requested page count
- The request is invalid for allocator bounds or page-count rules

## Free Interface

### Required Behavior

- The allocator exposes a free operation for a previously returned `PageSpan`.
- Successful free marks the full span available again and returns a released-span
  result.
- Invalid free requests return failure and leave allocator state unchanged.

### Failure Conditions

- The span was never allocated by the allocator
- The span overlaps allocator metadata, kernel-owned, or otherwise unmanaged
  pages
- The span bounds do not match page alignment or tracked allocator limits

## Observable Validation Contract

- `cargo test` covers memory-region normalization, contiguous search behavior,
  allocation bookkeeping, invalid request handling, and free/reuse semantics
- `cargo check --target x86_64-unknown-uefi` confirms the feature remains valid
  in the freestanding target
- `make build` and `./run.sh` confirm the allocator integrates into the current
  GRUB + UEFI boot path without regressing the visible boot outcome
