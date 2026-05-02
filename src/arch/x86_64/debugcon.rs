use core::arch::asm;
use core::fmt;

const DEBUGCON_PORT: u16 = 0x402;

pub struct DebugCon;

impl DebugCon {
    pub const fn new() -> Self {
        Self
    }

    fn write_byte(&mut self, byte: u8) {
        unsafe {
            asm!(
                "out dx, al",
                in("dx") DEBUGCON_PORT,
                in("al") byte,
                options(nomem, nostack, preserves_flags)
            );
        }
    }
}

impl fmt::Write for DebugCon {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
        Ok(())
    }
}
