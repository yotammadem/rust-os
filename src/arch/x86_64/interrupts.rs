use core::arch::{asm, global_asm};
use core::fmt::{self, Write};

use super::{debugcon::DebugCon, halt, serial::SerialPort};
use crate::kernel::runtime;

const PAGE_FAULT_VECTOR: usize = 14;
const DOUBLE_FAULT_VECTOR: usize = 8;
const INVALID_OPCODE_VECTOR: usize = 6;
const INTERRUPT_GATE_PRESENT: u8 = 0x8e;
const KERNEL_CODE_SELECTOR: u16 = 1 << 3;
const KERNEL_DATA_SELECTOR: u16 = 2 << 3;

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    type_attributes: u8,
    offset_middle: u16,
    offset_high: u32,
    reserved: u32,
}

impl IdtEntry {
    const fn missing() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            type_attributes: 0,
            offset_middle: 0,
            offset_high: 0,
            reserved: 0,
        }
    }

    fn new(handler: u64, selector: u16) -> Self {
        Self {
            offset_low: handler as u16,
            selector,
            ist: 0,
            type_attributes: INTERRUPT_GATE_PRESENT,
            offset_middle: (handler >> 16) as u16,
            offset_high: (handler >> 32) as u32,
            reserved: 0,
        }
    }
}

#[repr(C, packed)]
struct Idtr {
    limit: u16,
    base: u64,
}

#[repr(C, packed)]
struct Gdtr {
    limit: u16,
    base: u64,
}

#[repr(C)]
struct ExceptionStackFrame {
    rip: u64,
    cs: u64,
    rflags: u64,
}

static mut IDT: [IdtEntry; 256] = [IdtEntry::missing(); 256];
static mut GDT: [u64; 3] = [
    0,
    0x00af9a000000ffff,
    0x00cf92000000ffff,
];

global_asm!(
    r#"
    .global rust_os_page_fault_stub
rust_os_page_fault_stub:
    mov rdi, [rsp]
    lea rsi, [rsp + 8]
    and rsp, -16
    call rust_os_page_fault_handler
    ud2

    .global rust_os_invalid_opcode_stub
rust_os_invalid_opcode_stub:
    xor edi, edi
    mov rsi, rsp
    and rsp, -16
    call rust_os_invalid_opcode_handler
    ud2

    .global rust_os_double_fault_stub
rust_os_double_fault_stub:
    mov rdi, [rsp]
    lea rsi, [rsp + 8]
    and rsp, -16
    call rust_os_double_fault_handler
    ud2
"#
);

unsafe extern "C" {
    fn rust_os_page_fault_stub();
    fn rust_os_invalid_opcode_stub();
    fn rust_os_double_fault_stub();
}

pub unsafe fn install_minimal_fault_handlers() {
    unsafe { install_kernel_gdt() };

    unsafe {
        let page_fault_stub = runtime::image_addr_to_runtime_virt(
            rust_os_page_fault_stub as *const () as usize as u64,
        )
        .expect("page-fault stub must live inside the runtime image window");
        let invalid_opcode_stub = runtime::image_addr_to_runtime_virt(
            rust_os_invalid_opcode_stub as *const () as usize as u64,
        )
        .expect("invalid-opcode stub must live inside the runtime image window");
        let double_fault_stub = runtime::image_addr_to_runtime_virt(
            rust_os_double_fault_stub as *const () as usize as u64,
        )
        .expect("double-fault stub must live inside the runtime image window");
        let idt_base = runtime::image_addr_to_runtime_virt(core::ptr::addr_of!(IDT) as u64)
            .expect("IDT storage must live inside the runtime image window");

        IDT[PAGE_FAULT_VECTOR] = IdtEntry::new(page_fault_stub, KERNEL_CODE_SELECTOR);
        IDT[INVALID_OPCODE_VECTOR] = IdtEntry::new(
            invalid_opcode_stub,
            KERNEL_CODE_SELECTOR,
        );
        IDT[DOUBLE_FAULT_VECTOR] = IdtEntry::new(double_fault_stub, KERNEL_CODE_SELECTOR);

        let idtr = Idtr {
            limit: (core::mem::size_of::<[IdtEntry; 256]>() - 1) as u16,
            base: idt_base,
        };

        asm!("lidt [{}]", in(reg) &idtr, options(readonly, nostack, preserves_flags));
    }
}

#[unsafe(no_mangle)]
extern "sysv64" fn rust_os_page_fault_handler(
    error_code: u64,
    frame: *const ExceptionStackFrame,
) -> ! {
    let mut debugcon = DebugCon::new();
    let mut serial = unsafe { SerialPort::com1() };
    let frame = unsafe { &*frame };

    write_fault_header(&mut debugcon, "page-fault");
    write_fault_header(&mut serial, "page-fault");
    write_fault_value(&mut debugcon, "cr2", read_cr2());
    write_fault_value(&mut serial, "cr2", read_cr2());
    write_fault_value(&mut debugcon, "rip", frame.rip);
    write_fault_value(&mut serial, "rip", frame.rip);
    write_fault_value(&mut debugcon, "error", error_code);
    write_fault_value(&mut serial, "error", error_code);
    write_fault_value(&mut debugcon, "rsp", current_stack_pointer());
    write_fault_value(&mut serial, "rsp", current_stack_pointer());

    halt::halt_forever()
}

#[unsafe(no_mangle)]
extern "sysv64" fn rust_os_invalid_opcode_handler(
    error_code: u64,
    frame: *const ExceptionStackFrame,
) -> ! {
    let mut debugcon = DebugCon::new();
    let mut serial = unsafe { SerialPort::com1() };
    let frame = unsafe { &*frame };

    write_fault_header(&mut debugcon, "invalid-opcode");
    write_fault_header(&mut serial, "invalid-opcode");
    write_fault_value(&mut debugcon, "rip", frame.rip);
    write_fault_value(&mut serial, "rip", frame.rip);
    write_fault_value(&mut debugcon, "error", error_code);
    write_fault_value(&mut serial, "error", error_code);
    write_fault_value(&mut debugcon, "rsp", current_stack_pointer());
    write_fault_value(&mut serial, "rsp", current_stack_pointer());

    halt::halt_forever()
}

#[unsafe(no_mangle)]
extern "sysv64" fn rust_os_double_fault_handler(
    error_code: u64,
    frame: *const ExceptionStackFrame,
) -> ! {
    let mut debugcon = DebugCon::new();
    let mut serial = unsafe { SerialPort::com1() };
    let frame = unsafe { &*frame };

    write_fault_header(&mut debugcon, "double-fault");
    write_fault_header(&mut serial, "double-fault");
    write_fault_value(&mut debugcon, "rip", frame.rip);
    write_fault_value(&mut serial, "rip", frame.rip);
    write_fault_value(&mut debugcon, "error", error_code);
    write_fault_value(&mut serial, "error", error_code);
    write_fault_value(&mut debugcon, "rsp", current_stack_pointer());
    write_fault_value(&mut serial, "rsp", current_stack_pointer());

    halt::halt_forever()
}

fn write_fault_header<W: Write>(writer: &mut W, label: &str) {
    let _ = writer.write_str("fault: ");
    let _ = writer.write_str(label);
    let _ = writer.write_str("\r\n");
}

fn write_fault_value<W: Write>(writer: &mut W, label: &str, value: u64) {
    let _ = writer.write_str("  ");
    let _ = writer.write_str(label);
    let _ = writer.write_str(": 0x");
    let _ = write_hex_u64(writer, value);
    let _ = writer.write_str("\r\n");
}

fn write_hex_u64<W: Write>(writer: &mut W, value: u64) -> fmt::Result {
    let mut digits = [0u8; 16];
    let mut nibbles = 0usize;
    let mut current = value;

    loop {
        let digit = (current & 0xf) as usize;
        digits[15 - nibbles] = b"0123456789abcdef"[digit];
        nibbles += 1;
        current >>= 4;
        if current == 0 {
            break;
        }
    }

    let bytes = &digits[(16 - nibbles)..];
    writer.write_str(unsafe { core::str::from_utf8_unchecked(bytes) })
}

unsafe fn install_kernel_gdt() {
    let gdt_base = runtime::image_addr_to_runtime_virt(core::ptr::addr_of!(GDT) as u64)
        .expect("GDT storage must live inside the runtime image window");
    let gdtr = Gdtr {
        limit: (core::mem::size_of::<[u64; 3]>() - 1) as u16,
        base: gdt_base,
    };

    unsafe {
        asm!(
            "lgdt [{gdtr}]",
            "mov ax, {data_sel:x}",
            "mov ds, ax",
            "mov es, ax",
            "mov ss, ax",
            "push {code_sel}",
            "lea rax, [rip + 2f]",
            "push rax",
            "retfq",
            "2:",
            gdtr = in(reg) &gdtr,
            code_sel = in(reg) u64::from(KERNEL_CODE_SELECTOR),
            data_sel = in(reg) KERNEL_DATA_SELECTOR,
            out("rax") _,
            options(preserves_flags)
        );
    }
}

fn current_stack_pointer() -> u64 {
    let rsp: u64;
    unsafe {
        asm!("mov {}, rsp", out(reg) rsp, options(nomem, nostack, preserves_flags));
    }
    rsp
}

fn read_cr2() -> u64 {
    let cr2: u64;
    unsafe {
        asm!("mov {}, cr2", out(reg) cr2, options(nomem, nostack, preserves_flags));
    }
    cr2
}

pub unsafe fn trigger_fault_at(addr: u64) -> ! {
    unsafe {
        asm!("mov rax, [{0}]", "ud2", in(reg) addr, options(noreturn));
    }
}

pub unsafe fn trigger_invalid_opcode() -> ! {
    unsafe {
        asm!("ud2", options(noreturn));
    }
}
