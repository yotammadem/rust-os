.text
.global cpu_halt

cpu_halt:
1:
    hlt
    jmp 1b
