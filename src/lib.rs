#![no_std]

#[cfg(any(target_os = "uefi", target_os = "none"))]
pub mod arch;
pub mod boot;

pub const BOOT_IMAGE_PATH: &str = "bin/hello-boot.img";
pub const DEFAULT_OVMF_CODE: &str = "/usr/local/share/qemu/edk2-x86_64-code.fd";
pub const HELLO_WORLD: &str = "hello world";
pub const LOADER_EFI_PATH: &str = "EFI/BOOT/LOADER.EFI";
pub const KERNEL_IMAGE_PATH: &str = "EFI/BOOT/KERNEL.BIN";
pub const LOADER_SERIAL_BANNER: &[u8] = b"loader boot info\r\n";
pub const KERNEL_SERIAL_BANNER: &[u8] = b"kernel stub\r\n";
pub const RUN_MISSING_IMAGE_ERROR: &str = "missing boot image: run `make build` first";
pub const RUN_MISSING_FIRMWARE_ERROR: &str =
    "missing UEFI firmware: set OVMF_CODE=/path/to/firmware";
