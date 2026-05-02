use rust_os::{HELLO_WORLD, HELLO_WORLD_SERIAL};

#[test]
fn visible_message_matches_spec() {
    assert_eq!(HELLO_WORLD, "hello world");
}

#[test]
fn serial_message_has_expected_line_ending() {
    assert_eq!(HELLO_WORLD_SERIAL, "hello world\r\n");
}
