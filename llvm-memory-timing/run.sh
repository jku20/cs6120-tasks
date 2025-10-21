#!/bin/bash
cd build &&
  make &&
  cd .. &&
  clang -fpass-plugin=`echo build/pass/InstrumentPass.so` -c something.c &&
  cc -c rtlib/rtlib.c &&
  cc something.o rtlib.o
