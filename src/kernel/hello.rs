use crate::arch::x86_64::framebuffer::FramebufferConsole;
use crate::boot::multiboot::EfiStatus;
use rust_os::HELLO_WORLD_UTF16;

pub fn render(console: &mut FramebufferConsole) -> Result<(), EfiStatus> {
    console.write_utf16(&HELLO_WORLD_UTF16)
}
