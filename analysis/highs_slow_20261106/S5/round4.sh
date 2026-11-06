#!/bin/bash
# 4 回目: neos-960392 の遅い種 (1・2) で、暫定解のない根の RENS の目標の固定率を変える (調整値の当たり)
cd "$(dirname "$0")"
for s in 1 2; do
  for r in 0.95 0.6; do
    python3 run1.py ours neos-960392 rate${r}_s$s 600 ENOMOTO_T_MIP_SEED=$s ENOMOTO_MIP_DEBUG_SUBMIP_STOP=1 ENOMOTO_T_MIP_NOINC_RENS_RATE=$r
  done
done
# swath1 の木の主スレッドの区間計時 (表示だけ)
for s in 0 1; do
  python3 run1.py ours swath1 xprof_s$s 600 ENOMOTO_T_MIP_SEED=$s ENOMOTO_MIP_XPROF=1
done
