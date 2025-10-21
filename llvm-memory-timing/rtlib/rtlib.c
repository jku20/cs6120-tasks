#include <inttypes.h>
#include <stdio.h>
#include <unistd.h>

/* rdtsc.h - test program for rdtsc
# For documentation, see https://github.com/fordsfords/rdtsc
#
# This code and its documentation is Copyright 2022 Steven Ford
# and licensed "public domain" style under Creative Commons "CC0":
#   http://creativecommons.org/publicdomain/zero/1.0/
# To the extent possible under law, the contributors to this project have
# waived all copyright and related or neighboring rights to this work.
# In other words, you can use this code for any purpose without any
# restrictions.  This work is published from: United States.  The project home
# is https://github.com/fordsfords/rdtsc
*/
#ifndef NSTM_H
#define NSTM_H

#ifdef __cplusplus
extern "C" {
#endif

#define RDTSC(rdtsc_val_)                                                      \
  do {                                                                         \
    uint32_t rdtsc_hi_, rdtsc_lo_;                                             \
    __asm__ volatile("rdtsc" : "=a"(rdtsc_lo_), "=d"(rdtsc_hi_));              \
    rdtsc_val_ = (uint64_t)rdtsc_hi_ << 32 | rdtsc_lo_;                        \
  } while (0)

#ifdef __cplusplus
}
#endif

#endif /* NSTM_H */

static uint64_t start_ticks;
static uint64_t end_ticks;

void start_timer() { RDTSC(start_ticks); }
void end_timer() {
  RDTSC(end_ticks);
  printf("%ld\n", end_ticks - start_ticks);
}
