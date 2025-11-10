@main {
  speculate;
  v1: bool = const false;
  commit;
  jmp .__trace_end_0;
.__trace_abort_0:
  jmp .L1;
.L1:
  jmp .L2;
.L2:
  jmp .L3;
.L3:
  jmp .L4;
.L4:
  v1: bool = const false;
.__trace_end_0:
  v2: bool = const false;
  ret;
}
