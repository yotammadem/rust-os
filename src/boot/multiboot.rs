use core::ffi::c_void;

pub type EfiHandle = *mut c_void;
pub type EfiStatus = usize;
pub type EfiPhysicalAddress = u64;
pub type EfiVirtualAddress = u64;
pub type EfiMemoryType = u32;
pub type EfiAllocateType = u32;
pub type EfiTpl = usize;
pub type EfiEvent = *mut c_void;
pub type EfiGuid = Guid;

pub const EFI_SUCCESS: EfiStatus = 0;
pub const EFI_ABORTED: EfiStatus = 0x8000_0000_0000_0015;
pub const EFI_BUFFER_TOO_SMALL: EfiStatus = 0x8000_0000_0000_0005;
pub const EFI_LOADER_DATA: EfiMemoryType = 4;

pub const LOADED_IMAGE_PROTOCOL_GUID: Guid = Guid::new(
    0x5B1B31A1,
    0x9562,
    0x11D2,
    [0x8E, 0x3F, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B],
);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Guid {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

impl Guid {
    pub const fn new(data1: u32, data2: u16, data3: u16, data4: [u8; 8]) -> Self {
        Self {
            data1,
            data2,
            data3,
            data4,
        }
    }
}

#[repr(C)]
pub struct TableHeader {
    pub signature: u64,
    pub revision: u32,
    pub header_size: u32,
    pub crc32: u32,
    pub reserved: u32,
}

#[repr(C)]
pub struct BootServices {
    pub hdr: TableHeader,
    pub raise_tpl: usize,
    pub restore_tpl: usize,
    pub allocate_pages: usize,
    pub free_pages: usize,
    pub get_memory_map: unsafe extern "efiapi" fn(
        memory_map_size: *mut usize,
        memory_map: *mut MemoryDescriptor,
        map_key: *mut usize,
        descriptor_size: *mut usize,
        descriptor_version: *mut u32,
    ) -> EfiStatus,
    pub allocate_pool: unsafe extern "efiapi" fn(
        pool_type: EfiMemoryType,
        size: usize,
        buffer: *mut *mut c_void,
    ) -> EfiStatus,
    pub free_pool: unsafe extern "efiapi" fn(buffer: *mut c_void) -> EfiStatus,
    pub create_event: usize,
    pub set_timer: usize,
    pub wait_for_event: usize,
    pub signal_event: usize,
    pub close_event: usize,
    pub check_event: usize,
    pub install_protocol_interface: usize,
    pub reinstall_protocol_interface: usize,
    pub uninstall_protocol_interface: usize,
    pub handle_protocol: unsafe extern "efiapi" fn(
        handle: EfiHandle,
        protocol: *const Guid,
        interface: *mut *mut c_void,
    ) -> EfiStatus,
    pub reserved: *mut c_void,
    pub register_protocol_notify: usize,
    pub locate_handle: usize,
    pub locate_device_path: usize,
    pub install_configuration_table: usize,
    pub load_image: usize,
    pub start_image: usize,
    pub exit: usize,
    pub unload_image: usize,
    pub exit_boot_services: usize,
    pub get_next_monotonic_count: usize,
    pub stall: usize,
    pub set_watchdog_timer: usize,
    pub connect_controller: usize,
    pub disconnect_controller: usize,
    pub open_protocol: usize,
    pub close_protocol: usize,
    pub open_protocol_information: usize,
    pub protocols_per_handle: usize,
    pub locate_handle_buffer: usize,
    pub locate_protocol: usize,
    pub install_multiple_protocol_interfaces: usize,
    pub uninstall_multiple_protocol_interfaces: usize,
    pub calculate_crc32: usize,
    pub copy_mem: usize,
    pub set_mem: usize,
    pub create_event_ex: usize,
}

#[repr(C)]
pub struct MemoryDescriptor {
    pub typ: u32,
    pub pad: u32,
    pub physical_start: EfiPhysicalAddress,
    pub virtual_start: EfiVirtualAddress,
    pub number_of_pages: u64,
    pub attribute: u64,
}

#[repr(C)]
pub struct LoadedImageProtocol {
    pub revision: u32,
    pub parent_handle: EfiHandle,
    pub system_table: *mut SystemTable,
    pub device_handle: EfiHandle,
    pub file_path: *mut c_void,
    pub reserved: *mut c_void,
    pub load_options_size: u32,
    pub load_options: *mut c_void,
    pub image_base: *mut c_void,
    pub image_size: u64,
    pub image_code_type: EfiMemoryType,
    pub image_data_type: EfiMemoryType,
    pub unload: usize,
}

#[repr(C)]
pub struct SystemTable {
    pub hdr: TableHeader,
    pub firmware_vendor: *mut u16,
    pub firmware_revision: u32,
    pub console_in_handle: EfiHandle,
    pub con_in: *mut c_void,
    pub console_out_handle: EfiHandle,
    pub con_out: *mut c_void,
    pub standard_error_handle: EfiHandle,
    pub std_err: *mut c_void,
    pub runtime_services: *mut c_void,
    pub boot_services: *mut BootServices,
    pub number_of_table_entries: usize,
    pub configuration_table: *mut c_void,
}
