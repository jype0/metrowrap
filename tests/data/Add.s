.set noat      /* allow manual use of $at */
.set noreorder /* don't insert nops after branches */

glabel Add
	add $v0, $a0, $a1
    jr $ra
    nop
.size Add, . - Add
    nop
