.set noat
.set noreorder

# Test function with local rodata symbols
glabel LocalSymbols
    lui     $v0, %hi(local_data$1)
    addiu   $v0, $v0, %lo(local_data$1)
    lui     $v1, %hi(global_data)
    addiu   $v1, $v1, %lo(global_data)
    jr      $ra
    nop

.section .rodata
.align 2
dlabel local_data$1
    .word 0x11111111
.size local_data$1, . - local_data$1

.section .rodata
.align 2
dlabel global_data
    .word 0x22222222
.size global_data, . - global_data
