# Contract: Build and Run Interface

## Build Interface

### Command

```bash
make build
```

### Required Behavior

- Produces one bootable disk image file at `bin/hello-boot.img`
- Refreshes the generated output deterministically on repeated builds
- Does not require manual file moves or renaming before execution

### Failure Conditions

- Missing required local toolchain components
- Missing GRUB host tooling needed to assemble the boot image
- Any boot image assembly step that fails to create the final disk image

## Run Interface

### Command

```bash
./run.sh
```

### Required Behavior

- Requires an existing `bin/hello-boot.img`
- Launches `qemu-system-x86_64` against that image
- Uses UEFI firmware from `OVMF_CODE` if supplied, otherwise falls back to the
  local default firmware path chosen in the plan
- Leaves the halted `hello world` screen visible until the user closes the
  emulator session

### Failure Conditions

- `bin/hello-boot.img` is missing
- `qemu-system-x86_64` is unavailable
- UEFI firmware file cannot be found

## Observable Boot Contract

- Boot path is x86_64 + UEFI + GRUB
- `hello world` appears exactly once on screen
- Execution halts after rendering and does not continue into another visible state
