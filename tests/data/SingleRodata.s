.set noat
.set noreorder

# Test function with single rodata symbol
glabel SingleRodata
    lui     $v0, %hi(single_data)
    addiu   $v0, $v0, %lo(single_data)
    jr      $ra
    nop

.section .rodata
.align 2
dlabel single_data
    .word 0xDEADBEEF
    .word 0xCAFEBABE
.size single_data, . - single_data
