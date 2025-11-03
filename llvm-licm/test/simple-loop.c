// RUN: clang -O0 -emit-llvm -Xclang -disable-O0-optnone -S simple-loop.c && opt -S -passes="mem2reg" < simple-loop.ll | opt -S -passes="my-licm" -load-pass-plugin=../build/licm/LICMPass.so | FileCheck %s
// CHECK: .lr.ph:                                           ; preds = %0
// CHECK: add nsw i32 1, 2
// CHECK: br
// CHECK: 4:
int a = 10;
int main() {
  int acc = 0;
  int b = 1;
  int c = 2;
  int d;
  for (int i = 0; i < a; i++) {
    d = b + c;
    acc += i + acc;
  }
  return acc + d;
}
