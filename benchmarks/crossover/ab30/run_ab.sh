#!/bin/bash
# 小さな問題で内点法 + クロスオーバーを 1 スレッドのプールで解く案 (既定、ENOMOTO_T_XO_SERIAL_NNZ) と従来の並列のままの比較。
cd "$(dirname "$0")/../../.."
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants serial= par=ENOMOTO_T_XO_SERIAL_NNZ=0 \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab30/nk.jsonl >> benchmarks/crossover/ab30/nk.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab30/done.txt
