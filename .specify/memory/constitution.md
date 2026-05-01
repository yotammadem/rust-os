<!--
Sync Impact Report
Version change: template -> 1.0.0
Modified principles:
- Principle 1 -> I. Rust-First Kernel
- Principle 2 -> II. Zero-Dependency Rule
- Principle 3 -> III. Assembly as a Last Resort
- Principle 4 -> IV. Unsafe Boundaries Must Be Contained
- Principle 5 -> V. Reproducible Bring-Up Validation
Added sections:
- Technical Boundaries
- Delivery Workflow
Removed sections:
- None
Templates requiring updates:
- ✅ updated .specify/templates/plan-template.md
- ✅ updated .specify/templates/spec-template.md
- ✅ updated .specify/templates/tasks-template.md
- ⚠ pending .specify/templates/commands/*.md (directory not present in this repository)
Follow-up TODOs:
- None
-->
# rust-os Constitution

## Core Principles

### I. Rust-First Kernel
All production kernel and platform code MUST be written in Rust. The codebase MUST
remain `no_std` unless a constitution amendment explicitly approves a different
runtime model. Safe Rust is the default; when low-level work requires `unsafe`,
the unsafe boundary MUST be kept as small as possible and wrapped in a safe,
well-named interface. The rationale is straightforward: language-level guarantees
are one of the main reasons to build this operating system in Rust at all.

### II. Zero-Dependency Rule
The project MUST not introduce third-party dependencies. GRUB is the only approved
external bootloader component. Toolchain-provided crates that are part of the Rust
distribution, such as `core` and `alloc`, are allowed; additional crates from
`crates.io`, git sources, or vendored third-party code are prohibited unless this
constitution is amended first. This keeps the trusted computing base small and
forces the project to understand its own low-level behavior.

### III. Assembly as a Last Resort
Assembly MUST only be used where the target architecture, boot protocol, context
switching, interrupt entry, or other hardware-defined interfaces make Rust
insufficient on its own. Each assembly file or inline assembly block MUST document
why Rust could not express the operation, what contract the assembly obeys, and
which Rust module owns it. Assembly MUST be isolated behind the smallest stable
surface practical so the rest of the kernel remains readable, portable, and
reviewable.

### IV. Unsafe Boundaries Must Be Contained
Unsafe code MUST be limited to hardware access, memory management primitives, boot
handoff, synchronization internals, and other operations whose invariants cannot
be expressed in safe Rust alone. Every unsafe block MUST be paired with a short
explanation of the invariant it relies on and SHOULD be encapsulated so callers use
a safe API whenever possible. This principle complements the assembly rule: if
low-level escape hatches are unavoidable, they must still be narrow and auditable.

### V. Reproducible Bring-Up Validation
Changes that affect boot, memory layout, interrupts, scheduling, or other critical
kernel paths MUST include a reproducible validation path before merge. Validation
MUST name the commands or emulator flow used to verify the change, and regressions
MUST be caught with automated Rust tests whenever host-side testing is practical.
Where full automation is not possible, the pull request MUST describe the manual
boot or hardware validation that was performed. Kernel work is only acceptable when
others can re-run the same checks and reach the same result.

## Technical Boundaries

- The default target is a Rust operating system kernel with a minimal boot chain
  built around GRUB.
- The repository MUST prefer Rust modules, linker configuration, and build scripts
  over handwritten assembly whenever both can solve the same problem.
- Feature designs MUST call out any new assembly, unsafe expansion, architecture-
  specific code, linker changes, or boot protocol changes as explicit review items.
- New abstractions MUST justify their existence in terms of correctness,
  maintainability, or hardware requirements; speculative infrastructure is not
  allowed.

## Delivery Workflow

- Every feature spec MUST list architecture assumptions, dependency impact,
  assembly impact, unsafe impact, and a validation plan.
- Every implementation plan MUST fail its Constitution Check if it introduces a new
  external dependency, expands assembly without justification, or leaves validation
  steps undefined.
- Every task list MUST include work items for validation and for documenting any
  assembly or unsafe boundary that changes.
- Code review MUST treat constitution violations as blocking defects, not style
  preferences.

## Governance

This constitution overrides conflicting local habits, feature plans, and task
lists. Amendments require a documented rationale, an explicit description of the
rules being changed, and updates to dependent templates before the amendment is
considered complete. Versioning follows semantic versioning for governance:
MAJOR for incompatible principle changes or removals, MINOR for new principles or
materially expanded rules, and PATCH for clarifications that do not change
behavioral expectations. Compliance review is mandatory for every plan and pull
request; reviewers MUST verify dependency policy, assembly minimization, unsafe
containment, and validation evidence against this document.

**Version**: 1.0.0 | **Ratified**: 2026-05-01 | **Last Amended**: 2026-05-01
