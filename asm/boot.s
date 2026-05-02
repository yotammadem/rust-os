.text
.global cpu_halt
.global cpu_activate_and_continue

cpu_halt:
    cli
1:
    hlt
    jmp 1b

cpu_activate_and_continue:
    cli
    mov rax, rdi
    mov rsp, rsi
    mov rdi, rcx
    mov cr3, rax
    jmp rdx
