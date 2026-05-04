use rust_os::{BOOT_IMAGE_PATH, KERNEL_IMAGE_PATH, LOADER_EFI_PATH, LOADER_SERIAL_BANNER};

#[test]
fn boot_image_path_points_to_bin_image() {
    assert_eq!(BOOT_IMAGE_PATH, "bin/hello-boot.img");
}

#[test]
fn efi_artifact_paths_match_image_layout() {
    assert_eq!(LOADER_EFI_PATH, "EFI/BOOT/LOADER.EFI");
    assert_eq!(KERNEL_IMAGE_PATH, "EFI/BOOT/KERNEL.BIN");
}

#[test]
fn loader_banner_ends_with_serial_newline() {
    assert!(LOADER_SERIAL_BANNER.ends_with(b"\r\n"));
}
