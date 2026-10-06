#!/bin/bash
# Mittelmann の小さめの 8 問 (前処理後の行数・列数が小さい順から選んだもの) を、今の既定で 3 方式
# (二段解法、内点法 + クロスオーバー、同時実行) と、同時実行の下界なしで解く。1 回ずつ、300 秒まで。
cd "$(dirname "$0")/../../.."
P="qap15 s250r10 datt256 scpm1 nug08-3rd fome13 ns1688926 ex10"
.venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only $P --methods slope_intercept ipm_crossover race \
  --reps 1 --time-limit 300 --out benchmarks/crossover/mitt_small/results.jsonl >> benchmarks/crossover/mitt_small/run.log 2>&1
ENOMOTO_T_XO_BOUND_GAP=0 .venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only $P --methods race \
  --reps 1 --time-limit 300 --out benchmarks/crossover/mitt_small/race_nobound.jsonl >> benchmarks/crossover/mitt_small/run_nobound.log 2>&1
echo ALL_DONE >> benchmarks/crossover/mitt_small/done.txt
