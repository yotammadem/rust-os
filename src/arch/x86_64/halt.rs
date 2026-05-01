#[cfg(target_os = "uefi")]
core::arch::global_asm!(include_str!("../../../asm/boot.s"));

#[cfg(target_os = "uefi")]
unsafe extern "C" {
    fn cpu_halt() -> !;
}

#[cfg(target_os = "uefi")]
pub fn halt_forever() -> ! {
    unsafe { cpu_halt() }
}

#[cfg(not(target_os = "uefi"))]
pub fn halt_forever() -> ! {
    loop {
        core::hint::spin_loop();
    }
}
