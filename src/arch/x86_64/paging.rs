use crate::memory::paging::KernelMappingTemplate;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActivationPlan {
    pub root_table_phys_addr: u64,
    pub higher_half_entry_addr: u64,
    pub higher_half_stack_top: u64,
    pub runtime_context_addr: u64,
    pub transition_alias_start: u64,
    pub transition_alias_page_count: usize,
}

impl ActivationPlan {
    pub fn from_template(
        root_table_phys_addr: u64,
        higher_half_entry_addr: u64,
        higher_half_stack_top: u64,
        runtime_context_addr: u64,
        template: &KernelMappingTemplate,
    ) -> Self {
        Self {
            root_table_phys_addr,
            higher_half_entry_addr,
            higher_half_stack_top,
            runtime_context_addr,
            transition_alias_start: template.transition_alias_start,
            transition_alias_page_count: template.transition_alias_page_count,
        }
    }
}

#[cfg(target_os = "uefi")]
unsafe extern "C" {
    fn cpu_activate_and_continue(
        root_table_phys_addr: u64,
        stack_top: u64,
        target_addr: u64,
        runtime_context_addr: u64,
    ) -> !;
}

pub unsafe fn load_page_table_root(root_table_phys_addr: u64) {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("mov cr3, {}", in(reg) root_table_phys_addr, options(nostack, preserves_flags));
    }
}

pub unsafe fn activate(plan: ActivationPlan) {
    unsafe { load_page_table_root(plan.root_table_phys_addr) };
}

#[cfg(target_os = "uefi")]
pub unsafe fn activate_and_continue(plan: ActivationPlan) -> ! {
    // Safety: the caller must ensure the continuation target, stack top, and runtime
    // context address are all reachable under the new root. The trampoline disables
    // interrupts, swaps CR3, switches to the higher-half stack, and jumps without
    // returning through the low bootstrap window.
    unsafe {
        cpu_activate_and_continue(
            plan.root_table_phys_addr,
            plan.higher_half_stack_top,
            plan.higher_half_entry_addr,
            plan.runtime_context_addr,
        )
    }
}

pub unsafe fn flush_runtime_mappings(root_table_phys_addr: u64) {
    // Safety: reloading CR3 is used only after mutating the active paging structures
    // so the processor drops stale translations for the runtime root we still own.
    unsafe { load_page_table_root(root_table_phys_addr) };
}

pub fn higher_half_alias_addr(low_addr: u64, transition_alias_start: u64) -> Option<u64> {
    low_addr
        .checked_sub(transition_alias_start)
        .map(|offset| crate::memory::KERNEL_VIRT_BASE + offset)
}

pub fn current_instruction_pointer() -> u64 {
    #[cfg(target_arch = "x86_64")]
    {
        let rip: u64;
        unsafe {
            core::arch::asm!("lea {}, [rip]", out(reg) rip, options(nomem, nostack, preserves_flags));
        }
        rip
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        0
    }
}

pub fn current_stack_pointer() -> u64 {
    #[cfg(target_arch = "x86_64")]
    {
        let rsp: u64;
        unsafe {
            core::arch::asm!("mov {}, rsp", out(reg) rsp, options(nomem, nostack, preserves_flags));
        }
        rsp
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        0
    }
}
