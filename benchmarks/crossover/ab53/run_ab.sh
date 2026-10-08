#!/bin/bash
# 正規方程式の Cholesky の並べ替え (各問題で交互に計測):
#   default: AMD
#   best: 行数 1000 以上なら AMD と METIS (nested dissection) の両方で記号分解し、因子の非零の少ない方 (ENOMOTO_T_CHOL_ORDER=2)
#   metis: 行数 1000 以上なら METIS (ENOMOTO_T_CHOL_ORDER=1)
cd "$(dirname "$0")/../../.."
V="default= best=ENOMOTO_T_CHOL_ORDER=2 metis=ENOMOTO_T_CHOL_ORDER=1"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab53/nk.jsonl >> benchmarks/crossover/ab53/nk.log 2>&1
P="qap15 s250r10 scpm1 fome13 ns1688926 ex10"
.venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only $P --variants $V \
  --reps 1 --time-limit 300 --out benchmarks/crossover/ab53/mitt.jsonl >> benchmarks/crossover/ab53/mitt.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab53/done.txt
