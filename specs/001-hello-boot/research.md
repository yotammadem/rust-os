# Research: Hello Boot

## Decision 1: Build one raw UEFI boot disk image

- **Decision**: Produce a single raw disk image file at `bin/hello-boot.img` that
  contains a GPT/FAT EFI system partition with GRUB, `grub.cfg`, and the kernel
  binary.
- **Rationale**: The clarified spec requires one runtime artifact that `run.sh`
  can boot directly. A raw disk image satisfies that contract cleanly and maps
  well to QEMU's `-drive file=...` flow.
- **Alternatives considered**:
  - ISO image: Rejected because the feature explicitly converged on one bootable
    disk image rather than optical-media semantics.
  - Separate kernel plus staging tree: Rejected because `run.sh` would then need
    extra assembly logic to locate or package artifacts at runtime.

## Decision 2: Use GRUB on UEFI to load a freestanding Rust ELF kernel

- **Decision**: Build a freestanding x86_64 Rust kernel ELF and let GRUB load it
  from the EFI system partition during UEFI boot.
- **Rationale**: This keeps GRUB as the actual bootloader, preserves the operating
  system orientation of the project, and avoids introducing a Rust bootloader
  crate or alternate boot stack that would violate the constitution.
- **Alternatives considered**:
  - Direct UEFI application as the final OS binary: Rejected because GRUB would be
    reduced to a launcher rather than the real boot path.
  - Limine or bootloader crates: Rejected because the constitution disallows new
    third-party dependencies beyond GRUB.

## Decision 3: Minimize assembly to one bootstrap translation unit

- **Decision**: Confine handwritten assembly to a single x86_64 bootstrap file for
  boot entry, required CPU mode or stack setup, and the final halt loop label if
  Rust cannot express it alone.
- **Rationale**: This satisfies the "minimum assembly" rule while giving a clear
  ownership boundary for the code Rust cannot safely or portably initialize by
  itself.
- **Alternatives considered**:
  - Multiple assembly files for boot, screen, and halt logic: Rejected because it
    expands the non-Rust surface unnecessarily.
  - Fully inline assembly spread across Rust modules: Rejected because it weakens
    auditability and makes unsafe boundaries harder to reason about.

## Decision 4: Render `hello world` through a GRUB-provided framebuffer

- **Decision**: Use boot metadata from GRUB to locate the framebuffer and render
  the message from Rust with a tiny built-in glyph routine.
- **Rationale**: UEFI systems cannot reliably assume VGA text mode is present.
  Framebuffer output is the most portable way to guarantee that the message is
  visible on screen under a GRUB + UEFI flow.
- **Alternatives considered**:
  - VGA text memory at `0xb8000`: Rejected because it is not a dependable UEFI-era
    display path.
  - Serial-only output: Rejected because the specification requires the message to
    appear on screen.

## Decision 5: Use macOS-native image assembly tools plus GRUB utilities

- **Decision**: Build the disk image with macOS-native tooling where possible and
  rely on GRUB utilities only for the GRUB EFI payload and configuration staging.
- **Rationale**: This avoids adding third-party image-manipulation dependencies
  while staying compatible with the current Mac environment.
- **Alternatives considered**:
  - `mtools`, `xorriso`, or guest filesystem utilities: Rejected as extra host
    dependencies not required by the feature itself.
  - Manual hand-built FAT structures in Rust: Rejected as unnecessary complexity
    for the first boot milestone.

## Decision 6: Make `run.sh` fail fast and allow firmware override

- **Decision**: `run.sh` will require an existing `bin/hello-boot.img`, look for
  `/usr/local/share/qemu/edk2-x86_64-code.fd` by default on this machine, and
  allow an `OVMF_CODE` environment override before failing with a clear error.
- **Rationale**: The clarification session established that build and run
  responsibilities must stay separate. An override keeps the script usable on
  systems where the firmware path differs.
- **Alternatives considered**:
  - Auto-build when the image is missing: Rejected by explicit clarification.
  - Hardcode one firmware path with no override: Rejected because it would make
    the run flow brittle across machines.
