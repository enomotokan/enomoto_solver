#!/bin/bash
# 頂点を確かめて返す案を既定にした後の HiGHS (内点法 + クロスオーバー) との比較 (各問題で交互に計測)。
cd "$(dirname "$0")/../../.."
V="highs_ipm= default="
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab38/nk.jsonl >> benchmarks/crossover/ab38/nk.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab38/done.txt
