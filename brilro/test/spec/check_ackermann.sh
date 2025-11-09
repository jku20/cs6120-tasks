#!/bin/bash
for i in `seq 1 3`; do
  for j in `seq 1 3`; do
    base=`bril2json < ackermann.bril | brili $i $j`
    opt=`bril2json < ackermann.spec | brili $i $j`
    if (( "$opt" != "$base" )); then
      echo "error on input $i $j: found $opt but expected $base"
      exit 1
    fi
  done
done
