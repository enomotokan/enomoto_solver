#!/bin/bash
# 課題 PDLG: PDLP の解から内点法を始める案の比較 (Netlib 93 + Kennington 16 = 109 問、案を問題ごとに交互に 3 回)。
# p2 = ENOMOTO_T_XO_PDLP=2 (PDLP 500 反復の解を近接中心と初期点に)、A = 初期点の相補積を [μ/κ, κμ] に寄せる、
# C = 正則化 ρ・δ の初期値 (ENOMOTO_T_PDLP_WARM_REG0)。m1 = ENOMOTO_T_XO_PDLP=1 (近接中心だけ)。
cd "$(dirname "$0")/../../.."
P=ENOMOTO_T_XO_PDLP=2,ENOMOTO_T_PDLP_MAX_ITERS=500
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --reps 3 --time-limit 600 \
  --out benchmarks/crossover/pdlg/r109.jsonl --variants \
  base= \
  p2=$P \
  p2C8=$P,ENOMOTO_T_PDLP_WARM_REG0=1e-8 \
  p2C6=$P,ENOMOTO_T_PDLP_WARM_REG0=1e-6 \
  p2A10C8=$P,ENOMOTO_T_PDLP_WARM_CENTER=10,ENOMOTO_T_PDLP_WARM_REG0=1e-8 \
  p2B1C8=$P,ENOMOTO_T_PDLP_WARM_CSTEPS=1,ENOMOTO_T_PDLP_WARM_REG0=1e-8 \
  m1C8=ENOMOTO_T_XO_PDLP=1,ENOMOTO_T_PDLP_MAX_ITERS=500,ENOMOTO_T_PDLP_WARM_REG0=1e-8 \
  >> benchmarks/crossover/pdlg/r109.log 2>&1
echo ALL_DONE >> benchmarks/crossover/pdlg/r109.log
