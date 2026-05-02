#![no_std]

#[cfg(not(target_os = "uefi"))]
extern crate std;

pub mod arch;
pub mod boot;
pub mod kernel;
pub mod memory;

pub const BOOT_ARTIFACT_PATH: &str = ".build/efi";
pub const DEFAULT_OVMF_CODE: &str = "/usr/local/share/qemu/edk2-x86_64-code.fd";
pub const DIRECT_MAP_SMOKE_PREFIX: &str = "direct-map smoke:";
pub const HELLO_WORLD: &str = "hello world";
pub const HELLO_WORLD_SERIAL: &str = "hello world\r\n";
pub const KERNEL_BOOT_PHYS_BASE: u64 = 0x0010_0000;
pub const PAGING_DIAGNOSTIC_PREFIX: &str = "paging root:";
pub const RUN_MISSING_ARTIFACT_ERROR: &str = "missing staged EFI tree: run `make build` first";
pub const RUN_MISSING_FIRMWARE_ERROR: &str =
    "missing UEFI firmware: set OVMF_CODE=/path/to/firmware";
