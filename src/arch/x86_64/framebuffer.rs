use crate::boot::uefi::{EfiStatus, SimpleTextOutputProtocol, SystemTable};

pub struct FramebufferConsole {
    con_out: *mut SimpleTextOutputProtocol,
}

impl FramebufferConsole {
    pub unsafe fn from_system_table(system_table: *mut SystemTable) -> Option<Self> {
        if system_table.is_null() {
            return None;
        }

        let con_out = unsafe { (*system_table).con_out };
        if con_out.is_null() {
            return None;
        }

        Some(Self { con_out })
    }

    pub fn write_utf16(&mut self, message: &[u16]) -> Result<(), EfiStatus> {
        let status = unsafe { ((*self.con_out).output_string)(self.con_out, message.as_ptr()) };
        if status == 0 { Ok(()) } else { Err(status) }
    }
}
