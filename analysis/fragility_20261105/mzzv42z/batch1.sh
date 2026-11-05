#!/bin/bash
cd "$(dirname "$0")"  # (計測のときは analysis/fragility_20261105/ に置いていた)
for s in 0 1 2 3; do ./run1.sh t4_s${s} $s 4; done
./run1.sh t4_s0_r2 0 4
for s in 4 5 6; do ./run1.sh t4_s${s} $s 4; done
./run1.sh t4_s0_r3 0 4
for s in 7 8 9 10; do ./run1.sh t4_s${s} $s 4; done
for s in 0 1 2; do ./run1.sh t1_s${s} $s 1; done
