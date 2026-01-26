.set noat
.set noreorder

# Test function with multiple rodata symbols
glabel MultiRodata
    lui     $v0, %hi(rodata_string1)
    addiu   $v0, $v0, %lo(rodata_string1)
    lui     $v1, %hi(rodata_array)
    addiu   $v1, $v1, %lo(rodata_array)
    lui     $a0, %hi(rodata_string2)
    addiu   $a0, $a0, %lo(rodata_string2)
    jr      $ra
    nop
.size MultiRodata, . - MultiRodata

.section .rodata
.align 2
dlabel rodata_string1
    .asciz "First string"
.size rodata_string1, . - rodata_string1

.section .rodata
.align 2
dlabel rodata_array
    .word 0x12345678
    .word 0x9ABCDEF0
    .word 0x11111111
    .word 0x22222222
.size rodata_array, . - rodata_array

.section .rodata
.align 2
dlabel rodata_string2
    .asciz "Second string data"
.size rodata_string2, . - rodata_string2
