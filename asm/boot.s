.text
.global cpu_halt

cpu_halt:
    cli
1:
    hlt
    jmp 1b
