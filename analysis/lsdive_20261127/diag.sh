#!/bin/sh
# 30n20b8 (種 1、1 スレッド、600 秒) を LSDIVE つきでログを取って解き、新しい暫定解の出どころと種類ごとのダイビングの数を見る
cd "$(dirname "$0")"
M=$(python3 -c "import sys;sys.path.insert(0,'../../scripts');import run_miplib_benchmark as rb;from pathlib import Path;print(rb.fetch('30n20b8',Path('../../.miplib_cache')))")
for cfg in ls def; do
  E=""; [ $cfg = ls ] && E="ENOMOTO_MIP_LSDIVE=1"
  env $E ENOMOTO_MIP_THREADS=1 ENOMOTO_T_MIP_SEED=1 ENOMOTO_MIP_LOG=1 python3 ../../scripts/run_miplib_benchmark.py --worker-ours "$M" --time-limit 600 --threads 1 2> diag_$cfg.err | tail -1 > diag_$cfg.out
  grep -E "new incumbent|LSDIVE|TRSTAT: tree start" diag_$cfg.err > diag_$cfg.sum; gzip -f diag_$cfg.err
done
