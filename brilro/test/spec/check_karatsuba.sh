#!/bin/bash
nums='234 56908 30945 3945908 2934 2 0 1'
for i in $nums; do
  for j in $nums; do
    echo "checking $i $j"
    base=`bril2json < karatsuba.bril | brili $i $j`
    opt=`bril2json < karatsuba.spec | brili $i $j`
    if (( "$opt" != "$base" )); then
      echo "error on input $i $j: found $opt but expected $base"
      exit 1
    fi
  done
done
