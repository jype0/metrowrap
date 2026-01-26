#include "common.h"

INCLUDE_ASM("tests/data", GlobalSymbolTest);

void call_global_symbol_test(void) {
    GlobalSymbolTest();
}
