#!/bin/bash
# 正規方程式のパターンの作り方を変えた後の計測と、内点法の反復数の記録 (HiGHS の第 20 回と比べる)。
cd "$(dirname "$0")/../../.."
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants default= \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab23/all.jsonl >> benchmarks/crossover/ab23/all.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab23/done.txt
