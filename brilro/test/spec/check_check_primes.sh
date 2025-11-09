#!/bin/bash
for i in `seq 2 50`; do
  `bril2json < check-primes.bril | brili $i > /tmp/check_check_primes_base.out`
  `bril2json < check-primes.spec | brili $i > /tmp/check_check_primes_opt.out`
  echo "checking $i"
  diff /tmp/check_check_primes_opt.out /tmp/check_check_primes_base.out || exit 1
done
rm /tmp/check_check_primes_base.out
rm /tmp/check_check_primes_opt.out
