use rust_os::{HELLO_WORLD, HELLO_WORLD_SERIAL};

#[test]
fn visible_message_matches_spec() {
    assert_eq!(HELLO_WORLD, "hello world");
}

#[test]
fn serial_message_has_expected_wire_format() {
    assert_eq!(HELLO_WORLD_SERIAL, b"hello world\r\n");
}
