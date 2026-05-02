use core::arch::asm;
use core::fmt;

const COM1_BASE: u16 = 0x3f8;
const DATA_REGISTER: u16 = COM1_BASE;
const INTERRUPT_ENABLE_REGISTER: u16 = COM1_BASE + 1;
const FIFO_CONTROL_REGISTER: u16 = COM1_BASE + 2;
const LINE_CONTROL_REGISTER: u16 = COM1_BASE + 3;
const MODEM_CONTROL_REGISTER: u16 = COM1_BASE + 4;
const LINE_STATUS_REGISTER: u16 = COM1_BASE + 5;
const TRANSMIT_HOLDING_EMPTY: u8 = 1 << 5;

pub struct SerialPort {
    base: u16,
}

impl SerialPort {
    pub const unsafe fn com1() -> Self {
        Self { base: COM1_BASE }
    }

    pub unsafe fn initialize(&mut self) {
        let _ = self.base;
        unsafe {
            outb(INTERRUPT_ENABLE_REGISTER, 0x00);
            outb(LINE_CONTROL_REGISTER, 0x80);
            outb(DATA_REGISTER, 0x03);
            outb(INTERRUPT_ENABLE_REGISTER, 0x00);
            outb(LINE_CONTROL_REGISTER, 0x03);
            outb(FIFO_CONTROL_REGISTER, 0xc7);
            outb(MODEM_CONTROL_REGISTER, 0x0b);
        }
    }

    fn write_byte(&mut self, byte: u8) {
        while !self.transmit_ready() {
            core::hint::spin_loop();
        }

        unsafe { outb(DATA_REGISTER, byte) };
    }

    fn transmit_ready(&self) -> bool {
        unsafe { inb(LINE_STATUS_REGISTER) & TRANSMIT_HOLDING_EMPTY != 0 }
    }
}

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
        Ok(())
    }
}

unsafe fn outb(port: u16, value: u8) {
    unsafe {
        asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nomem, nostack, preserves_flags)
        );
    }
}

unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    unsafe {
        asm!(
            "in al, dx",
            out("al") value,
            in("dx") port,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}
