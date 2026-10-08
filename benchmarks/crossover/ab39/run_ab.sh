#!/bin/bash
# 基底の選択の改良案 (各問題で交互に計測):
#   default: 今の既定
#   tol01: 一次独立とみなすしきい値を 1e-2 → 0.1 (ENOMOTO_T_XO_LI_TOL=0.1)
#   order: 基底の候補 B を境界からの距離 / |s_j| の大きい順に並べる (ENOMOTO_T_XO_LI_ORDER=1)
#   piv05: しきい値つき部分ピボットの比 0.1 → 0.5 (ENOMOTO_T_XO_LI_PIV_REL=0.5)
cd "$(dirname "$0")/../../.."
V="default= tol01=ENOMOTO_T_XO_LI_TOL=0.1 order=ENOMOTO_T_XO_LI_ORDER=1 piv05=ENOMOTO_T_XO_LI_PIV_REL=0.5"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab39/nk.jsonl >> benchmarks/crossover/ab39/nk.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab39/done.txt
