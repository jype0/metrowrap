#include "common.h"

#ifdef USE_ASM
INCLUDE_ASM("tests/data", Add);
#else
int Add(int a, int b) {
    // actually a subtract
    return a - b;
}
#endif

int Init(void) {
    return Add(1, 2);
}
