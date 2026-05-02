use crate::memory::paging::KernelMappingTemplate;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActivationPlan {
    pub root_table_phys_addr: u64,
    pub higher_half_entry_addr: u64,
    pub transition_alias_start: u64,
    pub transition_alias_page_count: usize,
}

impl ActivationPlan {
    pub fn from_template(root_table_phys_addr: u64, higher_half_entry_addr: u64, template: &KernelMappingTemplate) -> Self {
        Self {
            root_table_phys_addr,
            higher_half_entry_addr,
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

pub unsafe fn activate(plan: ActivationPlan) {
    unsafe { load_page_table_root(plan.root_table_phys_addr) };
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
