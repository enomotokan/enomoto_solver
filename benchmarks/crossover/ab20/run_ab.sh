#!/bin/bash
# HiGHS の内点法 (IPX) + クロスオーバーとの比較 (各問題で 2 つを続けて計測)。途中で止まっても続きから再開できる。
cd "$(dirname "$0")/../../.."
P="qap15 s250r10 datt256 scpm1 nug08-3rd fome13 ns1688926 ex10"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --methods highs_ipm ipm_crossover \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab20/nk.jsonl >> benchmarks/crossover/ab20/nk.log 2>&1
.venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only $P --methods highs_ipm ipm_crossover \
  --reps 1 --time-limit 300 --out benchmarks/crossover/ab20/mitt.jsonl >> benchmarks/crossover/ab20/mitt.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab20/done.txt
