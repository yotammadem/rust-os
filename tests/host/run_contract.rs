use rust_os::{
    DEFAULT_OVMF_CODE, PAGING_DIAGNOSTIC_PREFIX, RUN_MISSING_ARTIFACT_ERROR,
    RUN_MISSING_FIRMWARE_ERROR,
};

#[test]
fn default_firmware_path_matches_plan() {
    assert_eq!(
        DEFAULT_OVMF_CODE,
        "/usr/local/share/qemu/edk2-x86_64-code.fd"
    );
}

#[test]
fn run_errors_are_clear() {
    assert!(RUN_MISSING_ARTIFACT_ERROR.contains("make build"));
    assert!(RUN_MISSING_FIRMWARE_ERROR.contains("OVMF_CODE"));
}

#[test]
fn paging_diagnostic_prefix_is_stable() {
    assert_eq!(PAGING_DIAGNOSTIC_PREFIX, "paging root:");
}
