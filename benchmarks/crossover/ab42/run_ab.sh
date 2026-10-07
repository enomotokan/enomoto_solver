#!/bin/bash
# 前処理の後でもう一度行・列をそろえる案 (ENOMOTO_T_POST_SCALE、双対二段解法と内点法 + クロスオーバーの両方。各問題で交互に計測):
#   *_d: 既定 (最初の Ruiz だけ)
#   *_p1: + 前処理の後に Ruiz
#   *_p2: + 前処理の後に Ruiz + Pock–Chambolle
#   *_p1n: 最初の Ruiz なし (ENOMOTO_T_RUIZ_ITERS=0) + 前処理の後に Ruiz
cd "$(dirname "$0")/../../.."
V="x_d:ipm_crossover= x_p1:ipm_crossover=ENOMOTO_T_POST_SCALE=1 x_p2:ipm_crossover=ENOMOTO_T_POST_SCALE=2 x_p1n:ipm_crossover=ENOMOTO_T_RUIZ_ITERS=0,ENOMOTO_T_POST_SCALE=1
   s_d:slope_intercept= s_p1:slope_intercept=ENOMOTO_T_POST_SCALE=1 s_p2:slope_intercept=ENOMOTO_T_POST_SCALE=2 s_p1n:slope_intercept=ENOMOTO_T_RUIZ_ITERS=0,ENOMOTO_T_POST_SCALE=1"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 30 --time-limit 120 --out benchmarks/crossover/ab42/nk.jsonl >> benchmarks/crossover/ab42/nk.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab42/done.txt
