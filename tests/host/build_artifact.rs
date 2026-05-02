use rust_os::{BOOT_ARTIFACT_PATH, HELLO_WORLD_SERIAL};

#[test]
fn boot_artifact_path_points_to_staged_efi_tree() {
    assert_eq!(BOOT_ARTIFACT_PATH, ".build/efi");
}

#[test]
fn hello_message_is_serial_line_terminated() {
    assert!(HELLO_WORLD_SERIAL.ends_with("\r\n"));
}
