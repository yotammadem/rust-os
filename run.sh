#!/bin/zsh
set -euo pipefail

EFI_TREE=".build/efi"
GRUB_EFI="$EFI_TREE/EFI/BOOT/BOOTX64.EFI"
APP_EFI="$EFI_TREE/EFI/BOOT/HELLO.EFI"
OVMF_CODE="${OVMF_CODE:-/usr/local/share/qemu/edk2-x86_64-code.fd}"
QEMU_DEBUGCON_LOG="${QEMU_DEBUGCON_LOG:-}"
QEMU_DEBUG_FLAGS="${QEMU_DEBUG_FLAGS:-0}"

if [[ ! -f "$GRUB_EFI" || ! -f "$APP_EFI" ]]; then
  echo "missing staged EFI tree: run \`make build\` first" >&2
  exit 1
fi

if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
  echo "missing qemu-system-x86_64 on PATH" >&2
  exit 1
fi

if [[ ! -f "$OVMF_CODE" ]]; then
  echo "missing UEFI firmware: set OVMF_CODE=/path/to/firmware" >&2
  exit 1
fi

qemu_args=(
  -machine q35,accel=hvf
  -m 256M
  -serial stdio
  -drive if=pflash,format=raw,readonly=on,file="$OVMF_CODE"
  -drive format=raw,file=fat:rw:"$EFI_TREE"
)

if [[ -n "$QEMU_DEBUGCON_LOG" ]]; then
  qemu_args+=(
    -debugcon "file:$QEMU_DEBUGCON_LOG"
    -global isa-debugcon.iobase=0x402
  )
fi

if [[ "$QEMU_DEBUG_FLAGS" == "1" ]]; then
  qemu_args+=(
    -no-reboot
    -no-shutdown
    -d int,guest_errors,cpu_reset
  )
fi

exec qemu-system-x86_64 "${qemu_args[@]}"
