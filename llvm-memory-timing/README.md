# LLVM Memory Timing
This pass instruments all memory instructions with a timer, recording how long each takes. A runtime library supporting you processor must be linked into the final program. Included here is support for x86\_64 processors using a timer based on the rdtsc instruction.
