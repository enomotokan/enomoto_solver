#!/bin/bash
# 正規方程式の分解を自前の並列マルチフロンタル法で行う (faer が supernodal を選び演算量の見積もりが 1e7 以上のとき。各問題で交互に計測):
#   default: faer (AMD)
#   mf: マルチフロンタル法 (ENOMOTO_T_CHOL_BACKEND=2、並べ替えは AMD)
#   mf_ord2: マルチフロンタル法 + 演算量が大きければ METIS も試す (ENOMOTO_T_CHOL_ORDER=2)
cd "$(dirname "$0")/../../.."
V="default= mf=ENOMOTO_T_CHOL_BACKEND=2 mf_ord2=ENOMOTO_T_CHOL_BACKEND=2,ENOMOTO_T_CHOL_ORDER=2"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab57/nk.jsonl >> benchmarks/crossover/ab57/nk.log 2>&1
P="qap15 s250r10 scpm1 fome13 ns1688926 ex10"
.venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only $P --variants $V \
  --reps 1 --time-limit 300 --out benchmarks/crossover/ab57/mitt.jsonl >> benchmarks/crossover/ab57/mitt.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab57/done.txt
