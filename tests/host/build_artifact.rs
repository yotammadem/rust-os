use rust_os::{BOOT_IMAGE_PATH, HELLO_WORLD_SERIAL};

#[test]
fn boot_image_path_points_to_bin_image() {
    assert_eq!(BOOT_IMAGE_PATH, "bin/hello-boot.img");
}

#[test]
fn hello_message_ends_with_serial_newline() {
    assert!(HELLO_WORLD_SERIAL.ends_with(b"\r\n"));
}
