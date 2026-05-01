use rust_os::{BOOT_IMAGE_PATH, HELLO_WORLD_UTF16, utf16_is_nul_terminated};

#[test]
fn boot_image_path_points_to_bin_image() {
    assert_eq!(BOOT_IMAGE_PATH, "bin/hello-boot.img");
}

#[test]
fn hello_message_buffer_is_nul_terminated() {
    assert!(utf16_is_nul_terminated(&HELLO_WORLD_UTF16));
}
