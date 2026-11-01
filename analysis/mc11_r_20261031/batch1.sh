#!/bin/bash
# 1: mc11 を hs_pi0_bgw (bgw) と hs_pi0_bgw_r (r) で各 5 回 (交互)
cd "$(dirname "$0")"
for i in 1 2 3 4 5; do
  ./run.sh mc11 mc11_bgw_$i bgw
  ./run.sh mc11 mc11_r_$i r
done
