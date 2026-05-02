# Contract: Runtime Paging Interface

## Live Kernel Handoff Interface

### Required Behavior

- The kernel exposes a handoff path that consumes the boot memory snapshot and a
  ready bitmap allocator, constructs a processor-consumable runtime paging
  hierarchy, and activates it by loading a new CR3 root.
- The handoff path defines a temporary low bootstrap alias that keeps the active
  instruction stream and stack reachable across the root switch.
- The handoff path resumes execution at a known higher-half continuation point
  and emits the first success message only after the new translation state is
  active.
- The handoff path removes the temporary low bootstrap alias after higher-half
  execution is established.

### Failure Conditions

- The allocator cannot supply every page required for the runtime paging
  hierarchy
- The kernel cannot cover the active execution path or stack with the bootstrap
  alias
- The higher-half continuation point is not reachable after the root switch
- Removing the temporary low alias would invalidate required runtime mappings

## Direct-Map Interface

### Required Behavior

- The runtime paging layout defines a direct-map virtual window covering all
  allocator-managed physical RAM in scope for this milestone.
- The system exposes helpers to translate physical addresses into direct-map
  virtual addresses and to recover physical addresses from that same window.
- Newly allocated paging pages and direct-map smoke-test pages are initialized
  through valid kernel virtual addresses inside the active direct-map window.
- The direct map remains present in the shared kernel template inherited by new
  process address spaces.

### Failure Conditions

- A requested physical address falls outside the supported direct-map coverage
- The direct-map range overlaps another reserved kernel virtual range
- A direct-map smoke-test page cannot be written, read back, or released
  consistently after allocation

## Process Address-Space Interface

### Required Behavior

- The system creates a fresh process address space with a distinct root paging
  structure derived from the active kernel runtime template.
- Each process root inherits the shared kernel higher-half and direct-map
  regions while allocating private lower-half paging structures only as needed.
- Destroying a process address space releases only its reclaimable private
  paging structures and preserves the shared live kernel mappings.

### Failure Conditions

- The allocator cannot provide the process root or required private derived
  pages
- The shared kernel runtime template cannot be installed consistently into the
  new process root
- A requested private mapping conflicts with reserved or shared runtime ranges

## Observable Validation Contract

- `cargo test` covers direct-map translation helpers, rollback on paging-page
  allocation failure, bootstrap alias construction rules, process-space
  isolation, and destroy-time reclamation behavior
- `cargo check --target x86_64-unknown-uefi` confirms the runtime paging and
  handoff code remains valid for the freestanding target
- `make build` rebuilds and stages the EFI payload with the live handoff code
- `./run.sh` produces a QEMU transcript showing that `hello world` is printed
  after the root switch and that the direct-map smoke path succeeds
