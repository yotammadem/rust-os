use crate::memory::paging::KernelMappingTemplate;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActivationPlan {
    pub root_table_phys_addr: u64,
    pub higher_half_entry_addr: u64,
    pub higher_half_stack_pointer: u64,
    pub transition_alias_start: u64,
    pub transition_alias_page_count: usize,
}

impl ActivationPlan {
    pub fn from_template(
        root_table_phys_addr: u64,
        higher_half_entry_addr: u64,
        higher_half_stack_pointer: u64,
        template: &KernelMappingTemplate,
    ) -> Self {
        Self {
            root_table_phys_addr,
            higher_half_entry_addr,
            higher_half_stack_pointer,
            transition_alias_start: template.transition_alias_start,
            transition_alias_page_count: template.transition_alias_page_count,
        }
    }
}

pub unsafe fn load_page_table_root(root_table_phys_addr: u64) {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("mov cr3, {}", in(reg) root_table_phys_addr, options(nostack, preserves_flags));
    }
}

pub unsafe fn activate(plan: ActivationPlan) -> ! {
    unsafe {
        // Switch into the runtime address space and enter the higher-half continuation
        // through a normal UEFI/x64 call boundary so Rust code sees the expected ABI.
        core::arch::asm!(
            "cli", // Disable interrupts during the transition
            "mov cr3, {root}", // Install the runtime page-table root
            "mov rsp, {stack}", // Switch to the higher-half stack alias before calling Rust code
            "sub rsp, 40", // Reserve the required Win64 shadow space
            "mov rax, {entry}", // Stage the continuation target
            "call rax", // Transfer control into the higher-half continuation
            "ud2", // The continuation is not expected to return
            root = in(reg) plan.root_table_phys_addr,
            stack = in(reg) plan.higher_half_stack_pointer,
            entry = in(reg) plan.higher_half_entry_addr,
            options(noreturn)
        );
    }
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
