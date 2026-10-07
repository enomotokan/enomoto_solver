#!/bin/bash
# HiGHS の内点法 + クロスオーバーと今の既定を、各問題で交互に計測する (第 21〜23 回の改良の後)。
cd "$(dirname "$0")/../../.."
P="qap15 s250r10 datt256 scpm1 nug08-3rd fome13 ns1688926 ex10"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants highs_ipm= default= \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab24/nk.jsonl >> benchmarks/crossover/ab24/nk.log 2>&1
.venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only $P --variants highs_ipm= default= \
  --reps 1 --time-limit 300 --out benchmarks/crossover/ab24/mitt.jsonl >> benchmarks/crossover/ab24/mitt.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab24/done.txt
