#!/bin/bash
# 正規方程式の Cholesky の並べ替え (Newton 方向の手当てを既定にした後、各問題で交互に計測):
#   default: AMD
#   ord2: AMD の演算量の見積もりが 1e8 以上なら METIS も試し、演算量が 0.8 倍未満なら METIS (ENOMOTO_T_CHOL_ORDER=2)
cd "$(dirname "$0")/../../.."
V="default= ord2=ENOMOTO_T_CHOL_ORDER=2"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab56/nk.jsonl >> benchmarks/crossover/ab56/nk.log 2>&1
P="qap15 s250r10 scpm1 fome13 ns1688926 ex10"
.venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only $P --variants $V \
  --reps 1 --time-limit 300 --out benchmarks/crossover/ab56/mitt.jsonl >> benchmarks/crossover/ab56/mitt.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab56/done.txt
