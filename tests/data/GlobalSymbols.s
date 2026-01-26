.set noat
.set noreorder

# Test function with global symbols that aren't in relocations
glabel GlobalSymbolTest
    addiu   $v0, $zero, 42
    jr      $ra
    nop

# This is a global symbol that should be exported
.globl exported_constant
.type exported_constant, @object
exported_constant:
    .word 0xCAFEBABE

.section .rodata
.align 2
dlabel test_rodata
    .word 0xDEADBEEF
.size test_rodata, . - test_rodata
