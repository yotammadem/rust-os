use core::arch::asm;

const COM1_BASE: u16 = 0x3F8;
const DATA: u16 = 0;
const INTERRUPT_ENABLE: u16 = 1;
const FIFO_CONTROL: u16 = 2;
const LINE_CONTROL: u16 = 3;
const MODEM_CONTROL: u16 = 4;
const LINE_STATUS: u16 = 5;

const LINE_STATUS_TRANSMIT_READY: u8 = 1 << 5;
const LINE_CONTROL_DLAB: u8 = 1 << 7;
const LINE_CONTROL_8N1: u8 = 0x03;

pub struct SerialPort {
    base_port: u16,
}

impl SerialPort {
    pub unsafe fn com1() -> Self {
        let mut serial = Self {
            base_port: COM1_BASE,
        };
        unsafe { serial.initialize() };
        serial
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.write_byte(byte);
        }
    }

    unsafe fn initialize(&mut self) {
        // Disable interrupts while programming the UART.
        unsafe { self.write_register(INTERRUPT_ENABLE, 0x00) };
        // Enable divisor latch access to configure baud rate.
        unsafe { self.write_register(LINE_CONTROL, LINE_CONTROL_DLAB) };
        // Divisor 3 => 38400 baud with a 115200 Hz base clock.
        unsafe { self.write_register(DATA, 0x03) };
        unsafe { self.write_register(INTERRUPT_ENABLE, 0x00) };
        // Restore 8 data bits, no parity, 1 stop bit.
        unsafe { self.write_register(LINE_CONTROL, LINE_CONTROL_8N1) };
        // Enable FIFO, clear queues, 14-byte threshold.
        unsafe { self.write_register(FIFO_CONTROL, 0xC7) };
        // Assert DTR/RTS and enable OUT2 for a conventional initialized state.
        unsafe { self.write_register(MODEM_CONTROL, 0x0B) };
    }

    fn write_byte(&mut self, byte: u8) {
        while (unsafe { self.read_register(LINE_STATUS) } & LINE_STATUS_TRANSMIT_READY) == 0 {}
        unsafe { self.write_register(DATA, byte) };
    }

    unsafe fn read_register(&self, offset: u16) -> u8 {
        let value: u8;
        unsafe {
            asm!(
                "in al, dx",
                in("dx") self.base_port + offset,
                out("al") value,
                options(nomem, nostack, preserves_flags)
            );
        }
        value
    }

    unsafe fn write_register(&self, offset: u16, value: u8) {
        unsafe {
            asm!(
                "out dx, al",
                in("dx") self.base_port + offset,
                in("al") value,
                options(nomem, nostack, preserves_flags)
            );
        }
    }
}
