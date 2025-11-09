509
@ack(m: int, n: int): int {
  speculate;
  zero: int = const 0;
  one: int = const 1;
  cond_m: bool = eq m zero;
  guard cond_m .__trace_abort_0;
  tmp: int = add n one;
  commit;
  jmp .__trace_end_0;
.__trace_abort_0:
  zero: int = const 0;
  one: int = const 1;
  cond_m: bool = eq m zero;
  br cond_m .m_zero .m_nonzero;
.m_zero:
  tmp: int = add n one;
.__trace_end_0:
  ret tmp;
.m_nonzero:
  cond_n: bool = eq n zero;
  speculate;
  guard cond_n .__trace_abort_1;
  m1: int = sub m one;
  commit;
  jmp .__trace_end_1;
.__trace_abort_1:
  br cond_n .n_zero .n_nonzero;
.n_zero:
  m1: int = sub m one;
.__trace_end_1:
  tmp: int = call @ack m1 one;
  ret tmp;
.n_nonzero:
  m1: int = sub m one;
  n1: int = sub n one;
  t1: int = call @ack m n1;
  t2: int = call @ack m1 t1;
  ret t2;
}
@main(m: int, n: int) {
  tmp: int = call @ack m n;
  print tmp;
}
