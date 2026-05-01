use core::ffi::c_void;

pub type EfiHandle = *mut c_void;
pub type EfiStatus = usize;

pub const EFI_ABORTED: EfiStatus = 0x8000_0000_0000_0015;

#[repr(C)]
pub struct TableHeader {
    pub signature: u64,
    pub revision: u32,
    pub header_size: u32,
    pub crc32: u32,
    pub reserved: u32,
}

#[repr(C)]
pub struct SimpleTextOutputMode {
    pub max_mode: i32,
    pub mode: i32,
    pub attribute: i32,
    pub cursor_column: i32,
    pub cursor_row: i32,
    pub cursor_visible: bool,
}

#[repr(C)]
pub struct SimpleTextOutputProtocol {
    pub reset: usize,
    pub output_string: unsafe extern "efiapi" fn(
        this: *mut SimpleTextOutputProtocol,
        string: *const u16,
    ) -> EfiStatus,
    pub test_string: usize,
    pub query_mode: usize,
    pub set_mode: usize,
    pub set_attribute: usize,
    pub clear_screen: unsafe extern "efiapi" fn(this: *mut SimpleTextOutputProtocol) -> EfiStatus,
    pub set_cursor_position: usize,
    pub enable_cursor: usize,
    pub mode: *mut SimpleTextOutputMode,
}

#[repr(C)]
pub struct SystemTable {
    pub hdr: TableHeader,
    pub firmware_vendor: *mut u16,
    pub firmware_revision: u32,
    pub console_in_handle: EfiHandle,
    pub con_in: *mut c_void,
    pub console_out_handle: EfiHandle,
    pub con_out: *mut SimpleTextOutputProtocol,
    pub standard_error_handle: EfiHandle,
    pub std_err: *mut SimpleTextOutputProtocol,
    pub runtime_services: *mut c_void,
    pub boot_services: *mut c_void,
    pub number_of_table_entries: usize,
    pub configuration_table: *mut c_void,
}
