#!/bin/bash
# 試作 2 (ALNS を時間で呼ぶ形) で種 0〜9 を回し、58 問 (def・rob) を回す
cd "$(dirname "$0")"  # (計測のときは analysis/fragility_20261105/ に置いていた)
for s in 0 1 2 3 4 5 6 7 8 9; do ./run1.sh rob2_t4_s${s} $s 4 ENOMOTO_MIP_ROBUST_STALL=1; done
python run58.py def rob
echo all done
