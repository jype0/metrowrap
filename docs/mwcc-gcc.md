Metrowerks to GCC Arg Mapping
=============================

Informational
--------------

| Metrowerks | GCC | 
|------------|-----|-----------------|
| -version | --version | Print the compiler version |
| -timing  | _na_ |
| -progress | _na_ |
| -v, -verbose |  --verbose (for `ld`) | 
| -search    | -I | search path |
| -[no]wraplines | _na_ | |
| -maxerrors max | -fmax-errors=<max> | Maximum number or errors to print/report |
| -maxwarnings | _na_ |
| -msgstyle [mpw, std, gcc, IDE, parseable ] | _na_ | |
| -[no]stderr | _na_ | |


Preprocessing, Precompiling, Input Control
-----------------------------------------

| Metrowerks | GCC | 
|------------|-----|-----------------|
| -c   | -c | compile only, do not link |
| -[no]codegen |  | |
| -[no]convertpaths | | |
| -cwd [proj, source, explicit, include] | | |
| -D+ or -d[efine] name[=<value>] | -D<macro>[=<value>], --define-macro, --define-macro= | define a macro, value is 1 if none provided |
| -[no]defaults | | |
| -dis[assemble] | | |
| -E | | preprocess source files |
| -EP | | preprocess and strip line directives |
| -ext <extension> |
| -gccinc[ludes] | | adopt GCC `#include` semantics |
| -[no]gccdep[ends] |  | enable GCC style dependencies files |
| -i, -I | -I | |
| -I+ or -i path | | |
| -include | | | 


**Notes**

`-gccdeps` should be used if `-MD` or `-MMD` are used. The output file path, however
is not correct and replaces the output exension with `.d` instead of appending it to the
output file.

-------------------------------------------------------------------------------
Preprocessing, Precompiling, and Input File Control Options                    
-------------------------------------------------------------------------------
  -ir path                   # global; append a recursive access path to       
                             #   current #include list                         
  -[no]keepobj[ects]         # global; keep object files generated after       
                             #   invoking linker; if disabled, intermediate    
                             #   object files are temporary and deleted after  
                             #   link stage; objects are always kept when      
                             #   compiling                                     
  -M                         # global; cased; scan source files for            
                             #   dependencies and emit Makefile, do not        
                             #   generate object code                          
  -MM                        # global; cased; like -M, but do not list system  
                             #   include files                                 
  -MD                        # global; cased; like -M, but write dependency    
                             #   map to a file (see ~gccdep) and generate      
                             #   object code                                   
  -MMD                       # global; cased; like -MD, but do not list system 
                             #   include files                                 
  -make                      # global; scan source files for dependencies and  
                             #   emit Makefile, do not generate object code    
  -nofail                    # continue working after errors in earlier files  
  -nolink                    # global; compile only, do not link               
  -noprecompile              # do not precompile any files based on the        
                             #   filename extension                            
  -nosyspath                 # global; treat #include <...> like #include      
                             #   "..."; always search both user and system     
                             #   path lists                                    
  -o file|dir                # specify output filename or directory for object 
                             #   file(s) or text output, or output filename    
                             #   for linker if called                          
  -P                         # global; cased; preprocess and send output to    
                             #   file; do not generate code                    
  -precompile file|dir       # generate precompiled header from source; write  
                             #   header to 'file' if specified, or put header  
                             #   in 'dir'; if argument is "", write header to  
                             #   source-specified location; if neither is      
                             #   defined, header filename is derived from      
                             #   source filename; note: the driver can tell    
                             #   whether to precompile a file based on its     
                             #   extension; '-precompile file source' then is  
                             #   the same as '-c -o file source'               
  -preprocess                # global; preprocess source files                 
  -prefix file               # prefix text file or precompiled header onto all 
                             #   source files                                  
  -S                         # global; cased; passed to all tools;             
                             #   disassemble and send output to file           
  -[no]stdinc                # global; use standard system include paths       
                             #   (specified by the environment variable        
                             #   %MWCIncludes%); added after all system '-I'   
                             #   paths; default                                
  -U+ | -u[ndefine] name     # cased; undefine symbol 'name'                   

-------------------------------------------------------------------------------
Front-End C/C++ Language Options                                               
-------------------------------------------------------------------------------
  -ansi keyword              # specify ANSI conformance options, overriding    
                             #   the given settings                            
     off                     #    same as '-stdkeywords off', '-enum min', and 
                             #      '-strict off'                              
     on|relaxed              #    same as '-stdkeywords on', '-enum min', and  
                             #      '-strict on'                               
     strict                  #    same as '-stdkeywords on', '-enum int', and  
                             #      '-strict on'                               
                             #                                                 
  -bool on|off               # enable C++ 'bool' type, 'true' and 'false'      
                             #   constants; default is on                      
  -char keyword              # set sign of 'char'                              
     signed                  #    chars are signed; default                    
     unsigned                #    chars are unsigned                           
                             #                                                 
  -Cpp_exceptions on|off     # passed to linker;                               
                             #   enable or disable C++ exceptions; default is  
                             #   on                                            
  -dialect | -lang keyword   # passed to linker;                               
                             #   specify source language                       
     c                       #    treat source as C always                     
     c++                     #    treat source as C++ always                   
     ec++                    #    generate warnings for use of C++ features    
                             #      outside Embedded C++ subset (implies       
                             #      '-dialect cplus')                          
     c99                     #    compile with C99 extensions                  
                             #                                                 
  -enum keyword              # specify default size for enumeration types      
     min                     #    use the minimal-sized type                   
     int                     #    use int-sized enums; default                 
                             #                                                 
  -for_scoping on|off        # control legacy (non-standard) for-scoping       
                             #   behavior; when enabled, variables declared in 
                             #   'for' loops are visible to the enclosing      
                             #   scope; when disabled, such variables are      
                             #   scoped to the loop only; default is off       
  -fl[ag] pragma             # specify an 'on/off' compiler #pragma;           
                             #   '-flag foo' is the same as '#pragma foo on',  
                             #   '-flag no-foo' is the same as '#pragma foo    
                             #   off'; use '-pragma' option for other cases    
  -inline keyword[,...]      # specify inline options                          
     on|smart                #    turn on inlining for 'inline' functions;     
                             #      default                                    
     none|off                #    turn off inlining                            
     auto                    #    auto-inline small functions (without         
                             #      'inline' explicitly specified)             
     noauto                  #    do not auto-inline; default                  
     all                     #    turn on aggressive inlining: same as         
                             #      '-inline on, auto'                         
     deferred                #    defer inlining until end of compilation      
                             #      unit; this allows inlining of functions    
                             #      defined before and after the caller        
     level=n                 #    cased; inline functions up to 'n' levels     
                             #      deep; level 0 is the same as '-inline on'; 
                             #      for 'n', range 0 - 8                       
     [no]bottomup            #    inline bottom-up, starting from leaves of    
                             #      the call graph rather than the top-level   
                             #      function                                   
                             #                                                 
  -iso_templates on|off      # enable ISO C++ template parser; default is off  
  -[no]mapcr                 # reverse mapping of '\n' and '\r' so that        
                             #   '\n'==13 and '\r'==10 (for Macintosh MPW      
                             #   compatability)                                
  -msext keyword             # [dis]allow Microsoft VC++ extensions            
     on                      #    enable extensions: redefining macros,        
                             #      allowing XXX::yyy syntax when declaring    
                             #      method yyy of class XXX,                   
                             #      allowing extra commas,                     
                             #      ignoring casts to the same type,           
                             #      treating function types with equivalent    
                             #      parameter lists but different return types 
                             #      as equal,                                  
                             #      allowing pointer-to-integer conversions,   
                             #      and various syntactical differences        
     off                     #    disable extensions; default on non-x86       
                             #      targets                                    
                             #                                                 
  -[no]multibyte[aware]      # enable multi-byte character encodings for       
                             #   source text, comments, and strings            
  -once                      # prevent header files from being processed more  
                             #   than once                                     
  -pragma ...                # specify a #pragma for the compiler such as      
                             #   "#pragma ..."; quote the parameter if you     
                             #   provide an argument (i.e., '-pragma "myopt    
                             #   reset"')                                      
  -r[equireprotos]           # require prototypes                              
  -relax_pointers            # relax pointer type-checking rules               
  -RTTI on|off               # select run-time typing information (for C++);   
                             #   default is on                                 
  -stdkeywords on|off        # allow only standard keywords; default is off    
  -str[ings] keyword[,...]   # specify string constant options                 
     [no]reuse               #    reuse strings; equivalent strings are the    
                             #      same object; default                       
     [no]pool                #    pool strings into a single data object       
     [no]readonly            #    make all string constants read-only          
                             #                                                 
  -strict on|off             # specify ANSI strictness checking; default is    
                             #   off                                           
  -trigraphs on|off          # enable recognition of trigraphs; default is off 
  -wchar_t on|off            # enable wchar_t as a built-in C++ type; default  
                             #   is off                                        

-------------------------------------------------------------------------------
C/C++ Warning Options                                                          
-------------------------------------------------------------------------------
  -w[arn[ings]]              # global; for this tool;                          
    keyword[,...]            #   warning options                               
     off                     #    passed to all tools;                         
                             #      turn off all warnings                      
     on                      #    passed to all tools;                         
                             #      turn on most warnings                      
     [no]cmdline             #    passed to all tools;                         
                             #      command-line driver/parser warnings        
     [no]err[or] |           #    passed to all tools;                         
       [no]iserr[or]         #      treat warnings as errors                   
     all                     #    turn on all warnings, require prototypes     
     [no]pragmas |           #    illegal #pragmas                             
       [no]illpragmas        #                                                 
     [no]empty[decl]         #    empty declarations                           
     [no]possible |          #    possible unwanted effects                    
       [no]unwanted          #                                                 
     [no]unusedarg           #    unused arguments                             
     [no]unusedvar           #    unused variables                             
     [no]unused              #    same as -w [no]unusedarg,[no]unusedvar       
     [no]extracomma |        #    extra commas                                 
       [no]comma             #                                                 
     [no]pedantic |          #    pedantic error checking                      
       [no]extended          #                                                 
     [no]hidevirtual |       #    hidden virtual functions                     
       [no]hidden[virtual]   #                                                 
     [no]implicit[conv]      #    implicit arithmetic conversions              
     [no]notinlined          #    'inline' functions not inlined               
     [no]largeargs           #    passing large arguments to unprototyped      
                             #      functions                                  
     [no]structclass         #    inconsistent use of 'class' and 'struct'     
     [no]padding             #    padding added between struct members         
     [no]notused             #    result of non-void-returning function not    
                             #      used                                       
     [no]unusedexpr          #    use of expressions as statements without     
                             #      side effects                               
     display|dump            #    display list of active warnings              
                             #                                                 

-------------------------------------------------------------------------------
MIPS BackEnd Options                                                           
-------------------------------------------------------------------------------
  -farcall on|off            # specify how external routines will be called;   
                             #   'on' forces calls through pointers; 'off'     
                             #   allows direct or PC-relative calls; default   
                             #   is off                                        
  -fp keyword[,...]          # specify floating-point options; this option is  
                             #   ignored for processors with known absent or   
                             #   present floating-point units                  
     off                     #    disable floating-point code; use software    
                             #      emulation instead                          
     single                  #    support single-precision (32-bit) floats;    
                             #      default                                    
                             #                                                 
  -profile                   # enable calls to profiler                        

-------------------------------------------------------------------------------
Optimizer Options                                                              
-------------------------------------------------------------------------------
  -O                         # cased; same as '-O2'                            
  -O+keyword[,...]           # cased; specify UNIX-style optimization options  
     0                       #    '-O0'; same as '-opt off'                    
     1                       #    '-O1'; same as '-opt level=1'                
     2                       #    '-O2'; same as -opt level=2'                 
     3                       #    '-O3'; same as -opt level=3'                 
     4                       #    '-O4'; same as -opt level=4,intrinsics'      
     p                       #    same as '-opt speed'                         
     s                       #    same as '-opt space'                         
                             #                                                 
  -opt keyword[,...]         # specify optimization options                    
     off                     #    suppress all optimizations; default          
     on                      #    same as -opt level=2                         
     all|full                #    same as -opt speed, level=4,intrinsics       
     l[evel]=num             #    set optimization level:                      
                             #      level 0: no optimizations; debug safe      
                             #      level 1: local optimizations only;         
                             #      peephole, dead code elimination; debug     
                             #      safe                                       
                             #      level 2: adds common subexpression         
                             #      elimination, copy and expression           
                             #      propagation; debug safe                    
                             #      level 3: adds instruction scheduling, tail 
                             #      call optimization, loop-invariant code     
                             #      motion, strength reduction, dead store     
                             #      elimination, loop unrolling [with '-opt    
                             #      speed' only]; not debug safe               
                             #      level 4: like level 3 with more            
                             #      comprehensive optimizations from levels 1  
                             #      and 2; not debug safe                      
                             #      ; for 'num', range 0 - 4; default is 0     
     [no]intrinsics          #    inlining of intrinsic functions              
     display|dump            #    display list of active optimizations         
                             #                                                 

-------------------------------------------------------------------------------
Debugging Control Options                                                      
-------------------------------------------------------------------------------
  -g                         # global; cased; generate debugging information;  
                             #   same as '-sym full,elf'                       
  -sym keyword[,...]         # global; specify debugging options               
     off                     #    do not generate debugging information;       
                             #      default                                    
     on                      #    turn on debugging information                
     full                    #    store full paths in objects                  
                             #                                                 

-------------------------------------------------------------------------------
Project Options                                                                
-------------------------------------------------------------------------------
  -sdatathreshold short      # set maximum size in bytes for constant data     
                             #   objects before being spilled from SCONST      
                             #   section into data section; implies '-model    
                             #   absolute'; default is 8                       

