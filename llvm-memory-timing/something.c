#include <stdlib.h>

static int arr[100000];
static int size = 100000;

int main(int argc, const char **argv) {
  int acc = 0;
  for (int i = 0; i < size; i++) {
    int idx = rand() % size;
    [[clang::annotate("time")]]
    int j = arr[idx];
    acc += j;
  }
  return acc;
}
