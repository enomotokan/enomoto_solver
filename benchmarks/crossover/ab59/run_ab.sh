#!/bin/bash
# 頂点の採用判定で y を補正する (上下限の間にある基底変数の被約費用を 0 にする) 試験と、自前のマルチフロンタル法との組み合わせ
# (各問題で交互に計測):
#   default: 既定 (faer、補正なし)
#   yfix: 補正あり (ENOMOTO_T_XO_ACCEPT_YFIX=1)
#   mf_yfix: マルチフロンタル法 (ENOMOTO_T_CHOL_BACKEND=2) + 補正あり
cd "$(dirname "$0")/../../.."
V="default= yfix=ENOMOTO_T_XO_ACCEPT_YFIX=1 mf_yfix=ENOMOTO_T_CHOL_BACKEND=2,ENOMOTO_T_XO_ACCEPT_YFIX=1"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab59/nk.jsonl >> benchmarks/crossover/ab59/nk.log 2>&1
P="qap15 s250r10 scpm1 fome13 ns1688926 ex10"
.venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only $P --variants $V \
  --reps 1 --time-limit 300 --out benchmarks/crossover/ab59/mitt.jsonl >> benchmarks/crossover/ab59/mitt.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab59/done.txt
