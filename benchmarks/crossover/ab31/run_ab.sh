#!/bin/bash
# 1 スレッドで解く問題の非零の上限 (ENOMOTO_T_XO_SERIAL_NNZ) の比較と、HiGHS との差 (各問題で交互に計測)。
cd "$(dirname "$0")/../../.."
V="highs_ipm= default= t100k=ENOMOTO_T_XO_SERIAL_NNZ=100000 t1m=ENOMOTO_T_XO_SERIAL_NNZ=1000000"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab31/nk.jsonl >> benchmarks/crossover/ab31/nk.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab31/done.txt
