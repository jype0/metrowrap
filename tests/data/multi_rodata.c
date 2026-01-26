#include "common.h"

INCLUDE_ASM("tests/data", MultiRodata);

// This function references the assembly function to ensure linking
void call_multi_rodata(void) {
    MultiRodata();
}
