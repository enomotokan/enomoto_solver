#!/bin/bash
# 各問題で交互に計測 (Netlib + Kennington、3 回):
#   二段解法: 既定 / A2 (LU の稠密切替を行数 2000 以上・直前の LU が 1 行 16 要素以上に広げる、閾値 0.3)
#   内点法 + クロスオーバー: 既定 (基底の選択の行の非零を候補の列だけで数える) / rowall (従来の行列全体で数える) /
#     A2 / fix (非基底と検出した構造列を固定して小さな問題で続ける、ENOMOTO_T_XO_FIX=0.05)
cd "$(dirname "$0")/../../.."
A2="ENOMOTO_T_LU_DENSE_SWITCH_AUTO_MIN_M=2000,ENOMOTO_T_LU_DENSE_SWITCH_AUTO_LU_PER_ROW=16"
V="base:slope_intercept= A2:slope_intercept=$A2 base:ipm_crossover= rowall:ipm_crossover=ENOMOTO_T_XO_LI_ROWCNT_ALL=1 A2:ipm_crossover=$A2 fix:ipm_crossover=ENOMOTO_T_XO_FIX=0.05"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab35/nk.jsonl >> benchmarks/crossover/ab35/nk.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab35/done.txt
