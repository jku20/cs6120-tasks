; RUN: opt -S -load-pass-plugin=../build/licm/LICMPass.so -passes=my-licm < %s | FileCheck %s
define i32 @main() {
; CHECK: add
; CHECK: .header
  br label %.header;
.header:
  %1 = phi i32 [2, %0], [%2, %.header];
  %2 = add i32 %1, 1;
  %3 = icmp eq i32 %2, 10;
  %4 = add i32 1, 1;
  br i1 %3, label %.exit, label %.header;
.exit:
  %5 = phi i32 [%2, %.header];
  ret i32 %5; 
}
