// RUN: clang -O0 -emit-llvm -Xclang -disable-O0-optnone -S %s -o - | opt -S -passes="mem2reg" | opt -S -passes="my-licm" -load-pass-plugin=../build/licm/LICMPass.so | FileCheck %s
// CHECK: %1 = add nsw i32 1, 2
// CHECK: br label %2

int main() {
  int acc;
  int out = 0;
  int a = 1;
  int b = 2;
  for (int i = 0; i < 100; i++) {
    for (int j = 0; j < 100; j++) {
      out = a + b;
      acc += i + j;
    }
  }
  return out;
}
