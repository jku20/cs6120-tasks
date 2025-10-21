#!/bin/python3

import sys

count = 0
acc = 0;
s = sys.stdin.readlines()
for line in s:
    count += 1;
    acc += int(line)
print(acc / count);
