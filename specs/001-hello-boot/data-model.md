# Data Model: Hello Boot

## BootDiskImage

- **Purpose**: Represents the single runtime artifact created by `make build`.
- **Fields**:
  - `path`: Repository-local output path, expected to be `bin/hello-boot.img`
  - `format`: Raw disk image
  - `boot_mode`: UEFI
  - `bootloader`: GRUB
  - `kernel_path_inside_image`: Staged kernel binary path referenced by `grub.cfg`
- **Validation Rules**:
  - Must exist after a successful build
  - Must be the only artifact `run.sh` needs in order to launch QEMU
  - Must remain inside the gitignored `bin/` directory

## KernelBinary

- **Purpose**: Freestanding Rust kernel executable loaded by GRUB from the boot
  image.
- **Fields**:
  - `entry_symbol`: Architecture bootstrap entry point
  - `format`: ELF
  - `arch`: x86_64
  - `runtime_model`: `no_std`
- **Validation Rules**:
  - Must be loadable by GRUB in the selected boot flow
  - Must transfer control into Rust after the minimal bootstrap path
  - Must expose the code path that renders `hello world` and enters the halt state

## FramebufferConsole

- **Purpose**: In-memory representation of the screen buffer passed through the
  boot flow for visible output.
- **Fields**:
  - `base_address`: Framebuffer memory base
  - `width`: Visible width in pixels
  - `height`: Visible height in pixels
  - `pitch`: Bytes per row
  - `pixel_format`: Expected framebuffer layout
- **Validation Rules**:
  - Must be initialized from boot metadata before rendering
  - Must support writing `hello world` exactly once
  - Must leave the rendered message intact after the kernel halts

## RunInvocation

- **Purpose**: Describes one local emulator launch attempt through `run.sh`.
- **Fields**:
  - `qemu_binary`: Expected QEMU executable path or command name
  - `firmware_path`: UEFI firmware file path
  - `image_path`: Boot disk image path
  - `exit_policy`: Fail fast on missing prerequisites; otherwise launch and hold
    the halted display state open
- **Validation Rules**:
  - Must reject missing image input before launching QEMU
  - Must use the latest built disk image directly
  - Must preserve the halted screen state until the emulator session is closed

## BootOutcome

- **Purpose**: Captures the observable result of one successful boot attempt.
- **Fields**:
  - `message_rendered`: Boolean flag for visible `hello world`
  - `render_count`: Expected to be exactly `1`
  - `halted`: Boolean flag indicating execution stopped
  - `screen_persists`: Boolean flag indicating the message stays visible
- **Validation Rules**:
  - `message_rendered` must be true
  - `render_count` must equal `1`
  - `halted` must be true after rendering
  - `screen_persists` must remain true until the user closes the QEMU window

## State Transitions

1. `BuildRequested` → `ImageCreated`
2. `ImageCreated` → `RunRequested`
3. `RunRequested` → `BootEntered`
4. `BootEntered` → `MessageRendered`
5. `MessageRendered` → `HaltedVisibleState`

Invalid transitions:

- `RunRequested` without `ImageCreated`
- `HaltedVisibleState` before `MessageRendered`
- Any path that renders the message more than once
