#!/bin/zsh
set -euo pipefail

IMAGE="bin/hello-boot.img"
OVMF_CODE="${OVMF_CODE:-/usr/local/share/qemu/edk2-x86_64-code.fd}"

if [[ ! -f "$IMAGE" ]]; then
  echo "missing boot image: run \`make build\` first" >&2
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

exec qemu-system-x86_64 \
  -machine q35,accel=hvf \
  -m 256M \
  -serial stdio \
  -drive if=pflash,format=raw,readonly=on,file="$OVMF_CODE" \
  -drive format=raw,file="$IMAGE"
