use rust_os::{HELLO_WORLD, HELLO_WORLD_UTF16, visible_message_units};

#[test]
fn visible_message_matches_spec() {
    assert_eq!(HELLO_WORLD, "hello world");
}

#[test]
fn utf16_message_has_expected_visible_length() {
    assert_eq!(visible_message_units(&HELLO_WORLD_UTF16), 13);
}
