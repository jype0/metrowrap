.set noat
.set noreorder

# This file only defines rodata symbols, no function
.section .rodata
.align 2
dlabel OnlyRodata
    .asciz "This is only rodata, no code"
.size OnlyRodata, . - OnlyRodata
