# Session Report: 2026-05-01 Hello Boot

## Scope

This session implemented the first milestone of the project: `001 - Hello Boot`.
The goal was to create a GRUB-based x86_64 UEFI boot image that:

- builds into the gitignored `bin/` directory,
- launches through `run.sh` in QEMU,
- prints `hello world`,
- and halts while leaving the message visible.

## What Was Done

- Created the Rust project scaffold in [Cargo.toml](/Users/yotammadem/mademos/rust-os/Cargo.toml),
  [rust-toolchain.toml](/Users/yotammadem/mademos/rust-os/rust-toolchain.toml),
  and [.cargo/config.toml](/Users/yotammadem/mademos/rust-os/.cargo/config.toml).
- Added generated artifact boundaries in [.gitignore](/Users/yotammadem/mademos/rust-os/.gitignore)
  and `bin/.gitkeep`.
- Added the build pipeline in [Makefile](/Users/yotammadem/mademos/rust-os/Makefile).
- Added the runtime launcher in [run.sh](/Users/yotammadem/mademos/rust-os/run.sh).
- Added GRUB boot configuration in [grub/grub.cfg](/Users/yotammadem/mademos/rust-os/grub/grub.cfg).
- Added the minimal assembly halt helper in [asm/boot.s](/Users/yotammadem/mademos/rust-os/asm/boot.s).
- Added the Rust UEFI application entrypoint in [src/main.rs](/Users/yotammadem/mademos/rust-os/src/main.rs).
- Added boot/service table definitions in [src/boot/multiboot.rs](/Users/yotammadem/mademos/rust-os/src/boot/multiboot.rs).
- Added output and halt modules in [src/arch/x86_64/framebuffer.rs](/Users/yotammadem/mademos/rust-os/src/arch/x86_64/framebuffer.rs)
  and [src/arch/x86_64/halt.rs](/Users/yotammadem/mademos/rust-os/src/arch/x86_64/halt.rs).
- Added the hello-world kernel behavior in [src/kernel/hello.rs](/Users/yotammadem/mademos/rust-os/src/kernel/hello.rs).
- Added host-side validation tests in [tests/host.rs](/Users/yotammadem/mademos/rust-os/tests/host.rs)
  and `tests/host/*.rs`.
- Updated the feature quickstart and marked all implementation tasks complete in
  [specs/001-hello-boot/quickstart.md](/Users/yotammadem/mademos/rust-os/specs/001-hello-boot/quickstart.md)
  and [specs/001-hello-boot/tasks.md](/Users/yotammadem/mademos/rust-os/specs/001-hello-boot/tasks.md).

## Current Architecture

## Plain-English Explanation

This project currently works more like a small UEFI program than a traditional
fully self-managed kernel with direct screen hardware control.

The easiest way to think about the boot chain is:

1. QEMU starts a virtual machine.
2. That virtual machine starts UEFI firmware.
3. UEFI firmware starts GRUB.
4. GRUB loads our `HELLO.EFI` program.
5. UEFI calls our Rust entry function, `efi_main`.
6. Our Rust code asks UEFI to print `hello world`.
7. Our Rust code then enters an infinite halt loop.

So, in the current version:

- GRUB is the bootloader.
- `HELLO.EFI` is the program GRUB launches.
- `efi_main` is the first Rust function that actually runs.

### What `efi_main` Is

The function [src/main.rs](/Users/yotammadem/mademos/rust-os/src/main.rs)
contains:

```rust
pub extern "efiapi" fn efi_main(...)
```

This is the UEFI application entrypoint. It is the UEFI equivalent of `main()`
for a normal program.

`extern "efiapi"` means "use the calling convention that UEFI expects."

That is why this function is special:

- it is not called by our own Rust code,
- it is not called directly by GRUB as a normal Rust function,
- it is called by the UEFI loading environment after GRUB asks UEFI to load the
  `.efi` file.

### What `SystemTable` Is

UEFI passes a pointer to a big structure called the system table into
`efi_main`.

That system table is like a bundle of firmware-provided services. It gives the
program access to things such as:

- console output,
- boot services,
- firmware metadata.

In this project, we currently use it only for console output.

### How `hello world` Is Printed

The name `FramebufferConsole` is misleading in the current code. It does **not**
write character pixels into a graphics framebuffer yet.

What actually happens is:

1. `efi_main` receives a pointer to the UEFI `SystemTable`.
2. `FramebufferConsole::from_system_table(...)` pulls out `con_out`.
3. `con_out` is a pointer to a UEFI console protocol struct.
4. That protocol struct contains function pointers, including
   `output_string`.
5. Our code calls that `output_string` function pointer with a UTF-16 version of
   `hello world`.
6. UEFI firmware draws the text on screen for us.

So the characters are not currently drawn by our own glyph or pixel code.
Instead, the firmware is doing the rendering.

### Important Distinction: Protocol Pointer vs Function Pointer

`SystemTable.con_out` is **not** itself a function pointer.

It is a pointer to a struct. That struct contains function pointers.

The model is:

- `SystemTable.con_out` -> pointer to `SimpleTextOutputProtocol`
- `SimpleTextOutputProtocol.output_string` -> function pointer
- our code calls that function pointer

That is similar to calling a method through a table of callbacks.

### What Is Not Implemented Yet

A real framebuffer text renderer would do more work itself:

- find the graphics framebuffer memory,
- know the screen width, height, and pixel format,
- store bitmap glyphs for letters,
- write pixels directly into video memory.

This session did **not** implement that yet.

Right now, the project proves:

- GRUB can boot the image,
- UEFI can load our Rust EFI program,
- our Rust code runs,
- our Rust code can display `hello world`,
- and then halt.

## Boot Flow

1. `make build` compiles a `x86_64-unknown-uefi` Rust binary.
2. The build generates a GRUB standalone UEFI loader with the Rust EFI payload
   embedded into GRUB's memdisk.
3. The build assembles a raw disk image at `bin/hello-boot.img`.
4. `run.sh` launches QEMU with the local EDK2 firmware as a `pflash` drive.
5. GRUB starts and chainloads the embedded `HELLO.EFI` payload.
6. The Rust EFI application writes `hello world` through the UEFI text output
   protocol and then enters the halt loop from `asm/boot.s`.

## Source Structure

```text
src/
├── lib.rs
├── main.rs
├── boot/
│   ├── mod.rs
│   └── multiboot.rs
├── arch/
│   ├── mod.rs
│   └── x86_64/
│       ├── mod.rs
│       ├── framebuffer.rs
│       └── halt.rs
└── kernel/
    ├── mod.rs
    └── hello.rs
```

## Responsibility Split

- `src/main.rs`: UEFI entrypoint and top-level control flow.
- `src/boot/multiboot.rs`: UEFI-compatible table and console protocol types.
- `src/arch/x86_64/framebuffer.rs`: console output abstraction over UEFI text output.
- `src/arch/x86_64/halt.rs`: Rust-facing halt wrapper.
- `asm/boot.s`: the minimal assembly infinite halt loop.
- `src/kernel/hello.rs`: the user-visible behavior for this milestone.
- `Makefile`: compiles, stages, and assembles the boot image.
- `run.sh`: validates image/firmware presence and launches QEMU.

## Validation Performed

- `cargo test` passed, including 6 host-side checks.
- `cargo check --target x86_64-unknown-uefi` passed.
- `make build` succeeded and produced `bin/hello-boot.img`.
- `./run.sh` was smoke-tested in QEMU and reached a boot that printed
  `hello world`.

## Current Limitations

- Output currently uses the UEFI text output protocol rather than a graphical
  framebuffer implementation.
- The architecture is still a boot milestone, not a general-purpose kernel.
- The worktree still contains uncommitted implementation changes at the time this
  report was written.
