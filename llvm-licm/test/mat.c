#include <string.h>

/* This scale factor will be changed to equalise the runtime of the
   benchmarks. */
#define LOCAL_SCALE_FACTOR 46
#define UPPERLIMIT 100
#define RANDOM_VALUE (RandomInteger())
#define ZERO 0
#define MOD_SIZE 8095
typedef long matrix[UPPERLIMIT][UPPERLIMIT];

/*
 * Multiplies arrays A and B and stores the result in ResultArray.
 */
void Multiply(matrix A, matrix B, matrix Res) {
  register int Outer, Inner, Index;

  for (Outer = 0; Outer < UPPERLIMIT; Outer++)
    for (Inner = 0; Inner < UPPERLIMIT; Inner++) {
      Res[Outer][Inner] = ZERO;
      for (Index = 0; Index < UPPERLIMIT; Index++)
        Res[Outer][Inner] += A[Outer][Index] * B[Index][Inner];
    }
}

matrix ArrayA_ref, ArrayA, ArrayB_ref, ArrayB, ResultArray;
int main() {
  for (int i = 0; i < 10; i++) {
    // memcpy(ArrayA, ArrayA_ref, UPPERLIMIT * UPPERLIMIT * sizeof(ArrayA[0][0]));
    // memcpy(ArrayB, ArrayB_ref, UPPERLIMIT * UPPERLIMIT * sizeof(ArrayA[0][0]));

    Multiply(ArrayA, ArrayB, ResultArray);
  }
  return 0;
}
