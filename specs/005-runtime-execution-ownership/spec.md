# Feature Specification: Runtime Execution Ownership

**Feature Branch**: `[005-runtime-execution-ownership]`  
**Created**: 2026-05-02  
**Status**: Draft  
**Input**: User description: "1. real higher-half continuation trampoline
2. remove the temporary low/current execution alias
3. bring up kernel-owned GDT/IDT/TSS
4. replace the cli/hlt stopgap with proper interrupt-safe idle behavior"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Continue In The Higher Half (Priority: P1)

As a kernel developer, I want the kernel to resume execution at a stable higher-half continuation point after the runtime paging switch so the boot flow no longer depends on a temporary low execution window.

**Why this priority**: The current runtime handoff works only because it preserves the live low execution window. That is a bootstrap workaround, not the intended kernel runtime state.

**Independent Test**: Can be fully tested by booting in QEMU, switching to the runtime root, reaching the higher-half continuation path, and proving that serial output still appears after the temporary low execution alias is removed.

**Acceptance Scenarios**:

1. **Given** the kernel has built a runtime paging root, **When** it activates that root, **Then** execution continues from a known higher-half continuation point instead of relying on the current low execution window.
2. **Given** the higher-half continuation path is active, **When** the kernel removes the temporary low execution alias, **Then** the system continues running and still emits the expected post-switch serial output.

---

### User Story 2 - Own Execution Context (Priority: P2)

As a kernel developer, I want the runtime kernel to own the execution context used after the paging handoff so interrupts, faults, and privileged transitions no longer depend on firmware-owned state.

**Why this priority**: Once the paging switch succeeds, the kernel still depends on inherited firmware execution state. Owning that state is necessary before enabling more runtime behavior.

**Independent Test**: Can be fully tested by booting in QEMU, installing kernel-owned execution context state, provoking benign post-switch interrupt or fault-safe paths, and verifying the system remains under kernel control.

**Acceptance Scenarios**:

1. **Given** the runtime root is active, **When** the kernel transitions into its owned execution context, **Then** the processor uses kernel-provided descriptor and interrupt state for subsequent runtime handling.
2. **Given** the kernel-owned execution context is installed, **When** a post-switch runtime event requires stack or handler state, **Then** the processor reaches the intended kernel-managed path without falling back to firmware-owned state.

---

### User Story 3 - Enter A Stable Idle State (Priority: P3)

As a kernel developer, I want the post-boot kernel to enter an interrupt-safe idle state without relying on a global interrupt-disable workaround so the system can remain running without reboot loops.

**Why this priority**: The current `cli`/`hlt` combination is a temporary guardrail. The kernel needs a stable idle behavior that remains correct once it owns interrupt and exception handling.

**Independent Test**: Can be fully tested by booting in QEMU, reaching the post-boot idle path after the higher-half continuation and execution-context ownership steps, and verifying the system remains stable without rebooting or losing control.

**Acceptance Scenarios**:

1. **Given** the kernel has completed the runtime handoff and installed kernel-owned execution context state, **When** it enters its idle path, **Then** the system remains stable without requiring a firmware-era interrupt-disable stopgap.
2. **Given** the system is idling under kernel control, **When** expected timer or interrupt activity occurs, **Then** the kernel remains in a recoverable runtime state rather than resetting or rebooting.

### Edge Cases

- What happens if the higher-half continuation target is not reachable after the runtime root is activated?
- How does the kernel recover if removing the low execution alias would unmap the current stack or immediate continuation code?
- What happens if kernel-owned execution context state is only partially installed when a runtime event arrives?
- How does the system behave if the idle path is entered before the kernel has fully taken ownership of interrupt and exception handling?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST resume execution at a deterministic higher-half continuation point immediately after activating the runtime paging root.
- **FR-002**: The system MUST preserve the code, stack, and immediate runtime data needed to survive the paging-root switch until the higher-half continuation path is active.
- **FR-003**: The system MUST remove the temporary low/current execution alias after higher-half execution is established successfully.
- **FR-004**: The system MUST reject or abort the handoff if the higher-half continuation point, required stack state, or immediate runtime data would become unreachable during the transition.
- **FR-005**: The runtime kernel MUST install and use kernel-owned execution-context state for post-switch runtime handling instead of relying on inherited firmware-owned state.
- **FR-006**: The system MUST ensure that post-switch interrupt, exception, and privileged runtime handling reaches kernel-owned control paths once that execution-context ownership step is complete.
- **FR-007**: The post-boot kernel MUST provide an idle behavior that remains stable without depending on a blanket interrupt-disable workaround as its long-term runtime behavior.
- **FR-008**: The system MUST remain under kernel control after entering idle and MUST NOT reset or reboot as a result of expected post-boot runtime events.
- **FR-009**: The system MUST continue to provide observable serial transcript proof after the paging handoff, after low-alias removal, and after entering the post-boot idle state.
- **FR-010**: Failed continuation setup, alias removal, execution-context installation, or idle-state entry MUST fail in a bounded way without silently falling back to firmware-owned runtime behavior.

### Implementation Constraints *(mandatory for this project)*

- **IC-001**: The feature MUST be implemented in Rust and remain compatible with the project's `no_std` runtime assumptions unless explicitly approved otherwise.
- **IC-002**: The feature MUST NOT introduce third-party dependencies; if external code seems necessary, the spec MUST treat that as blocked pending a constitution amendment.
- **IC-003**: Any required assembly MUST be identified explicitly, with a reason that explains why Rust alone is insufficient.
- **IC-004**: Any new or expanded `unsafe` boundary MUST identify the invariant it relies on and the module that will contain it.
- **IC-005**: The spec MUST define how the feature will be validated, including automated checks where practical and manual boot or emulator checks where needed.

### Key Entities *(include if feature involves data)*

- **HigherHalfContinuationWindow**: The executable and stack-reachable runtime window that remains valid during the moment of paging-root activation and leads into the higher-half continuation point.
- **TemporaryExecutionAlias**: The short-lived low/current execution mapping retained only long enough to survive the handoff and removed once higher-half execution is proven stable.
- **KernelExecutionContext**: The kernel-owned descriptor, interrupt, exception, and stack-transition state used after the runtime handoff.
- **KernelIdleState**: The stable post-boot processor state entered after the runtime handoff and execution-context ownership steps are complete.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In repeated QEMU boots, the kernel reaches the higher-half continuation path and emits the expected post-switch serial transcript without depending on the preserved current low execution window.
- **SC-002**: In repeated QEMU boots, removing the temporary low/current execution alias does not prevent the kernel from continuing to run and emit the expected post-removal transcript markers.
- **SC-003**: In repeated validation runs, the system remains under kernel-owned execution control after the runtime handoff and does not fall back to firmware-owned runtime handling.
- **SC-004**: After entering the post-boot idle path, the system remains stable without rebooting or resetting during the validation window.

## Assumptions

- The current live runtime paging handoff and direct-map proof from feature `004-live-vm-handoff` remain the substrate for this follow-on feature.
- Single-processor boot remains the scope for this feature; multiprocessor coordination is still out of scope.
- User mode, scheduler bring-up, and demand paging remain out of scope; this feature only establishes the kernel-owned runtime foundation needed before those later milestones.
- Serial transcript output remains the primary externally visible proof path for continuation, alias removal, execution-context ownership, and stable idle behavior.

## Low-Level Impact *(mandatory)*

- **Architecture Impact**: Affects CR3 handoff flow, higher-half continuation, low alias removal, descriptor and interrupt ownership, and the post-boot idle path on x86_64.
- **Dependency Impact**: No new dependencies.
- **Assembly Impact**: Expected in the continuation trampoline and possibly in execution-context or idle-transition touchpoints where Rust alone cannot express the required processor-state transitions safely.
- **Unsafe Impact**: Expected around continuation publication, descriptor and interrupt state installation, privileged register updates, and any stack-sensitive runtime transition code.
- **Validation Plan**: Validate with `cargo test`, `cargo check --target x86_64-unknown-uefi`, `make build`, and repeated `./run.sh` transcript checks that prove higher-half continuation, low-alias removal, kernel-owned execution-context installation, and stable idle behavior without reboot loops.
