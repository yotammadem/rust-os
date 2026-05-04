use crate::arch::x86_64::serial::SerialPort;
use crate::boot::multiboot::EfiStatus;
use rust_os::HELLO_WORLD_SERIAL;

pub fn render(serial: &mut SerialPort) -> Result<(), EfiStatus> {
    serial.write_bytes(HELLO_WORLD_SERIAL);
    Ok(())
}
