#!/bin/bash
# 内点法の前の Pock–Chambolle と検出の組み合わせ (検出を γ = 1 にした後、各問題で交互に計測):
#   default: 今の既定 (内点法は Ruiz だけ、検出は元の単位で γ = 1)
#   pc: 内点法の前に Pock–Chambolle (ENOMOTO_T_XO_IPM_PC=1)、検出は元の単位で γ = 1
#   pc6: pc + そろえた空間で γ = 1 (ENOMOTO_T_XO_GAMMA_PDHG=6)
cd "$(dirname "$0")/../../.."
V="default= pc=ENOMOTO_T_XO_IPM_PC=1 pc6=ENOMOTO_T_XO_IPM_PC=1,ENOMOTO_T_XO_GAMMA_PDHG=6"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab49/nk.jsonl >> benchmarks/crossover/ab49/nk.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab49/done.txt
