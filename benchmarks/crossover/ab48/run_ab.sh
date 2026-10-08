#!/bin/bash
# 内点法 + クロスオーバーの改良案 (各問題で交互に計測、高精度化を既定にした後):
#   default: 今の既定 (検出は PDHG の 1 歩 + γ = τ_j)
#   g1: 検出は 1 歩を進めず γ = τ_j (ENOMOTO_T_XO_GAMMA_PDHG=1)
#   g0: 検出は内点法の出力から直接 γ = 1 (ENOMOTO_T_XO_GAMMA_PDHG=0)
#   gz6: 補正子の回数を分解と求解の時間の比で決める (ENOMOTO_T_IPM_GONDZIO_AUTO=6)
#   pc5: 内点法の前に Pock–Chambolle を行だけ (ENOMOTO_T_XO_IPM_PC=5)
cd "$(dirname "$0")/../../.."
V="default= g1=ENOMOTO_T_XO_GAMMA_PDHG=1 g0=ENOMOTO_T_XO_GAMMA_PDHG=0 gz6=ENOMOTO_T_IPM_GONDZIO_AUTO=6 pc5=ENOMOTO_T_XO_IPM_PC=5"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab48/nk.jsonl >> benchmarks/crossover/ab48/nk.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab48/done.txt
