# Data Model: Runtime Execution Ownership

## HigherHalfContinuationPlan

- **Purpose**: Defines the exact execution contract used for the first
  post-`CR3` instruction under the runtime root.
- **Fields**:
  - `root_table_phys_addr`: Physical address of the runtime top-level page table
  - `continuation_virt_addr`: Higher-half virtual address of the first owned
    continuation instruction
  - `bootstrap_stack_top`: Higher-half stack pointer used for the continuation
  - `transition_alias_start`: Temporary low/current execution alias start
  - `transition_alias_page_count`: Temporary alias size in 4 KiB pages
- **Validation Rules**:
  - `continuation_virt_addr` must be mapped in the runtime root before `CR3`
    activation
  - `bootstrap_stack_top` must point into mapped writable higher-half memory
  - The alias must cover only the transition-sensitive execution footprint and
    nothing needed after the first confirmed higher-half step

## TemporaryExecutionAlias

- **Purpose**: Represents the short-lived mapping retained only to survive the
  paging-root transition.
- **Fields**:
  - `virt_start`: Low/current virtual start of the alias
  - `phys_start`: Physical backing start
  - `page_count`: Number of mapped pages
  - `removal_trigger`: The exact event that permits teardown
  - `flush_strategy`: Translation flush method used after unmapping
- **Validation Rules**:
  - `removal_trigger` must be "first confirmed higher-half continuation step"
  - The alias must be unmapped before descriptor-table ownership and idle-path
    validation continue
  - Alias teardown must not invalidate the active higher-half code or stack

## KernelExecutionContext

- **Purpose**: Describes the kernel-owned descriptor and interrupt state used
  after the runtime handoff.
- **Fields**:
  - `gdt_descriptor`: Pointer and limit for the kernel-owned GDT
  - `kernel_code_selector`: Active kernel code segment selector
  - `kernel_data_selector`: Active kernel data segment selector
  - `tss_selector`: Selector for the kernel-owned TSS
  - `tss_rsp0`: Ring-0 stack pointer recorded in the TSS
  - `idt_descriptor`: Pointer and limit for the kernel-owned IDT
  - `breakpoint_vector`: Vector used for deliberate synchronous exception proof
  - `timer_vector`: Vector used for hardware timer IRQ proof
- **Validation Rules**:
  - GDT and IDT descriptors must reference mapped higher-half memory
  - TSS fields must point to mapped kernel stacks
  - Breakpoint and timer vectors must both resolve to kernel-owned handlers

## InterruptProofEvent

- **Purpose**: Captures the two required proof events that demonstrate owned
  runtime interrupt and exception handling.
- **Fields**:
  - `event_kind`: `Breakpoint` or `HardwareTimer`
  - `vector`: Interrupt or exception vector number
  - `handler_marker`: Serial/debugcon marker emitted on successful handling
  - `acknowledgement_required`: Whether external interrupt acknowledgement is
    required before return
- **Validation Rules**:
  - `Breakpoint` must be triggerable by the kernel on demand after IDT install
  - `HardwareTimer` must arrive from the programmed timer source while the CPU
    is in the idle path
  - Both events must complete without falling back to firmware-owned state

## KernelIdleState

- **Purpose**: Represents the steady post-boot idle behavior after runtime
  ownership is complete.
- **Fields**:
  - `interrupts_enabled`: Whether the idle loop executes with interrupts enabled
  - `halt_instruction`: Whether the processor uses `hlt` while idle
  - `wake_source`: Hardware source expected to wake the CPU
  - `resume_marker`: Transcript marker emitted after a successful wakeup
- **Validation Rules**:
  - `interrupts_enabled` must be true during steady-state idle
  - `wake_source` must be the kernel-handled hardware timer interrupt
  - The kernel must resume through a kernel-owned handler and remain stable

## State Transitions

1. `RuntimeRootPrepared` -> `ContinuationReady`
2. `ContinuationReady` -> `HigherHalfActive`
3. `HigherHalfActive` -> `AliasRemoved`
4. `AliasRemoved` -> `ExecutionContextInstalled`
5. `ExecutionContextInstalled` -> `BreakpointHandled`
6. `ExecutionContextInstalled` -> `IdleArmed`
7. `IdleArmed` -> `TimerWakeHandled`
8. `TimerWakeHandled` -> `StableIdle`

Invalid transitions:

- `HigherHalfActive` without a mapped higher-half continuation stack
- `ExecutionContextInstalled` while the temporary execution alias is still live
- `StableIdle` without at least one successful timer wake handled through the
  kernel-owned IDT
- `BreakpointHandled` if the trap path depends on firmware-owned descriptor or
  interrupt state
