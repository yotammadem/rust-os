# Data Model: Bitmap Allocator

## BootMemoryMapSnapshot

- **Purpose**: Immutable snapshot of boot-provided memory information captured
  before allocator initialization.
- **Fields**:
  - `regions`: Slice of raw or normalized memory regions discovered at boot
  - `descriptor_count`: Number of entries captured from firmware memory metadata
  - `descriptor_size`: Stride used to walk the boot memory descriptors
  - `page_size`: Fixed page size for allocator normalization, expected to be
    `4096`
  - `highest_usable_address`: Upper bound used to size the managed page bitmap
- **Validation Rules**:
  - Must be captured before allocator metadata pages are reserved
  - Must preserve enough information to classify usable versus non-usable ranges
  - Must not depend on heap allocation

## MemoryRegion

- **Purpose**: Page-aligned region record used by allocator initialization.
- **Fields**:
  - `start_phys_addr`: Inclusive physical start address
  - `length_bytes`: Total byte length of the region
  - `start_page_index`: First page index covered by the region
  - `page_count`: Number of 4 KiB pages in the aligned region
  - `kind`: One of `Usable`, `Reserved`, `Kernel`, `Boot`, or
    `AllocatorMetadata`
- **Validation Rules**:
  - Region bounds must be page-aligned after normalization
  - `page_count` must be zero only for discarded sub-page fragments
  - Overlapping normalized regions are invalid after classification

## PageSpan

- **Purpose**: Canonical representation of one allocated or freed physical page
  range.
- **Fields**:
  - `start_page_index`: First managed page in the span
  - `page_count`: Number of contiguous pages in the span
  - `start_phys_addr`: Physical address derived from `start_page_index`
  - `end_phys_addr_exclusive`: Exclusive upper address bound
- **Validation Rules**:
  - `page_count` must be greater than zero
  - All pages in the span must lie inside allocator-managed memory
  - Multi-page spans are always contiguous

## BitmapAllocatorState

- **Purpose**: Runtime allocator state that answers allocation and free requests.
- **Fields**:
  - `managed_start_page`: Lowest page index represented in the bitmap
  - `managed_page_count`: Total number of tracked pages
  - `bitmap_phys_addr`: Physical base address of the bitmap storage
  - `bitmap_len_bytes`: Total bitmap storage length
  - `free_page_count`: Running count of currently free pages
  - `last_search_page`: Optional cursor for continuing first-fit scans
- **Validation Rules**:
  - Bitmap bits for reserved, kernel, boot, and metadata pages start as used
  - `free_page_count` must match the number of clear bits in managed usable pages
  - The allocator must never return pages that back its own bitmap

## AllocationResult

- **Purpose**: Deterministic outcome of allocation or free requests.
- **Variants**:
  - `Allocated(PageSpan)`
  - `OutOfMemory`
  - `InvalidRequest`
  - `InvalidFree`
- **Validation Rules**:
  - Failed results must leave bitmap state unchanged
  - `InvalidFree` applies to unowned, overlapping, or out-of-range spans
  - `InvalidRequest` covers zero-page requests or impossible range sizes

## State Transitions

1. `BootMemoryCaptured` → `RegionsNormalized`
2. `RegionsNormalized` → `BitmapReserved`
3. `BitmapReserved` → `AllocatorReady`
4. `AllocatorReady` → `PagesAllocated`
5. `PagesAllocated` → `PagesFreed`
6. `PagesFreed` → `PagesAllocated`

Invalid transitions:

- `PagesAllocated` before `AllocatorReady`
- `PagesFreed` for a span that is not currently owned by the allocator
- Any transition that marks allocator metadata pages free
