#!/bin/bash
# 拡大系 (準定値、LDLᵀ) も自前のマルチフロンタル法で分解する (各問題で交互に計測):
#   default: 既定 (正規方程式はマルチフロンタル法、拡大系は faer)
#   aug_mf: 拡大系もマルチフロンタル法 (ENOMOTO_T_AUG_BACKEND=2、演算量の見積もりが 2e7 以上のとき)
cd "$(dirname "$0")/../../.."
V="default= aug_mf=ENOMOTO_T_AUG_BACKEND=2"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab62/nk.jsonl >> benchmarks/crossover/ab62/nk.log 2>&1
P="qap15 s250r10 scpm1 fome13 ns1688926 ex10"
.venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only $P --variants $V \
  --reps 1 --time-limit 300 --out benchmarks/crossover/ab62/mitt.jsonl >> benchmarks/crossover/ab62/mitt.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab62/done.txt
