#[cfg(any(target_os = "uefi", target_os = "none"))]
core::arch::global_asm!(include_str!("../../../asm/boot.s"));

#[cfg(any(target_os = "uefi", target_os = "none"))]
unsafe extern "C" {
    fn cpu_halt() -> !;
}

#[cfg(any(target_os = "uefi", target_os = "none"))]
pub fn halt_forever() -> ! {
    unsafe { cpu_halt() }
}

#[cfg(not(any(target_os = "uefi", target_os = "none")))]
pub fn halt_forever() -> ! {
    loop {
        core::hint::spin_loop();
    }
}
