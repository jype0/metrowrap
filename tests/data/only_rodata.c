#include "common.h"

INCLUDE_RODATA("tests/data", OnlyRodata);

void use_rodata(void) {
    extern const unsigned char OnlyRodataSymbol[];
    // Reference it so it doesn't get optimized away
    volatile const unsigned char* ptr = OnlyRodataSymbol;
}
