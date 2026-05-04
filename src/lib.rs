#![no_std]

pub const BOOT_IMAGE_PATH: &str = "bin/hello-boot.img";
pub const DEFAULT_OVMF_CODE: &str = "/usr/local/share/qemu/edk2-x86_64-code.fd";
pub const HELLO_WORLD: &str = "hello world";
pub const HELLO_WORLD_SERIAL: &[u8] = b"hello world\r\n";
pub const RUN_MISSING_IMAGE_ERROR: &str = "missing boot image: run `make build` first";
pub const RUN_MISSING_FIRMWARE_ERROR: &str =
    "missing UEFI firmware: set OVMF_CODE=/path/to/firmware";
