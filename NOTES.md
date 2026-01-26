Compiler Notes
==============

Interface
---------

GCC supports `-fuse-ld=<linker>` to specify a linker at runtime (probably as a result of `gold`, `mold`, `wild`, etc.).
Clang supports runtime linker selection with `-DLLVM_USE_LINKER=gold`

Neither clang or gcc support runtime assembler selection, but do support passing args to the assemler with the
`-Xassembler` or  `-Wa,` options.
