#!/bin/bash
# 内点法の前に Pock–Chambolle で行・列をそろえる案と、その空間で γ = 1 にする検出 (各問題で交互に計測):
#   default: 今の既定 (PDHG の 1 歩 + γ = τ_j)
#   pc: 内点法を Pock–Chambolle でそろえた問題で解く (ENOMOTO_T_XO_IPM_PC=1)、検出は既定
#   pc6: pc + そろえた空間で γ = 1 (ENOMOTO_T_XO_GAMMA_PDHG=6、PDHG の 1 歩なし)
#   g6: 内点法は既定のまま、検出だけ γ_j = dc_j² β/γ_c
cd "$(dirname "$0")/../../.."
V="default= pc=ENOMOTO_T_XO_IPM_PC=1 pc6=ENOMOTO_T_XO_IPM_PC=1,ENOMOTO_T_XO_GAMMA_PDHG=6 g6=ENOMOTO_T_XO_GAMMA_PDHG=6"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab41/nk.jsonl >> benchmarks/crossover/ab41/nk.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab41/done.txt
