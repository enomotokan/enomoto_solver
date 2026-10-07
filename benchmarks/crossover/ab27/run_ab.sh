#!/bin/bash
# 射影の押し出しを省いた既定のもとで、検出の歩幅 τ_j の上限 (ENOMOTO_T_XO_GAMMA_CAP) と γ = 1 を比べる (各問題で交互に計測)。
cd "$(dirname "$0")/../../.."
V="default= cap100=ENOMOTO_T_XO_GAMMA_CAP=100 cap10=ENOMOTO_T_XO_GAMMA_CAP=10 cap1=ENOMOTO_T_XO_GAMMA_CAP=1 gamma1=ENOMOTO_T_XO_GAMMA_PDHG=0"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab27/nk.jsonl >> benchmarks/crossover/ab27/nk.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab27/done.txt
