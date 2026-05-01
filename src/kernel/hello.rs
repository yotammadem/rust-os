use crate::HELLO_WORLD_UTF16;
use crate::arch::x86_64::framebuffer::FramebufferConsole;
use crate::boot::uefi::EfiStatus;

pub fn render(console: &mut FramebufferConsole) -> Result<(), EfiStatus> {
    console.write_utf16(&HELLO_WORLD_UTF16)
}
