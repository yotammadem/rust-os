use rust_os::{HELLO_WORLD, KERNEL_SERIAL_BANNER, LOADER_SERIAL_BANNER};

#[test]
fn visible_message_matches_spec() {
    assert_eq!(HELLO_WORLD, "hello world");
}

#[test]
fn serial_messages_have_expected_wire_format() {
    assert_eq!(LOADER_SERIAL_BANNER, b"loader boot info\r\n");
    assert_eq!(KERNEL_SERIAL_BANNER, b"kernel stub\r\n");
}
