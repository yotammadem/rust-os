#![no_std]

pub const BOOT_IMAGE_PATH: &str = "bin/hello-boot.img";
pub const DEFAULT_OVMF_CODE: &str = "/usr/local/share/qemu/edk2-x86_64-code.fd";
pub const HELLO_WORLD: &str = "hello world";
pub const HELLO_WORLD_UTF16: [u16; 14] = [
    'h' as u16,
    'e' as u16,
    'l' as u16,
    'l' as u16,
    'o' as u16,
    ' ' as u16,
    'w' as u16,
    'o' as u16,
    'r' as u16,
    'l' as u16,
    'd' as u16,
    '\r' as u16,
    '\n' as u16,
    0,
];
pub const RUN_MISSING_IMAGE_ERROR: &str = "missing boot image: run `make build` first";
pub const RUN_MISSING_FIRMWARE_ERROR: &str =
    "missing UEFI firmware: set OVMF_CODE=/path/to/firmware";

pub const fn utf16_is_nul_terminated(slice: &[u16]) -> bool {
    !slice.is_empty() && slice[slice.len() - 1] == 0
}

pub const fn visible_message_units(slice: &[u16]) -> usize {
    let mut idx = 0;
    while idx < slice.len() {
        if slice[idx] == 0 {
            break;
        }
        idx += 1;
    }
    idx
}
