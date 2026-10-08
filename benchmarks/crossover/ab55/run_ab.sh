#!/bin/bash
# 不正確な Newton 方向への手当て (各問題で交互に計測):
#   default: 今の既定
#   aug: 稠密な列を Woodbury で扱う正規方程式で、予測子の相対残差が 1e-6 を超えたら拡大系に切り替える
#        (ENOMOTO_T_IPM_AUG_ON_INACCURATE=1e-6)
#   aug_acc: aug + 不正確で進まない反復が 3 回続いたらその反復だけ正則化を強める (ENOMOTO_T_IPM_SOLVE_ACC2=1e-6、_K=3)
cd "$(dirname "$0")/../../.."
A="ENOMOTO_T_IPM_AUG_ON_INACCURATE=1e-6"
V="default= aug=$A aug_acc=$A,ENOMOTO_T_IPM_SOLVE_ACC2=1e-6,ENOMOTO_T_IPM_SOLVE_ACC2_K=3"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab55/nk.jsonl >> benchmarks/crossover/ab55/nk.log 2>&1
P="qap15 s250r10 scpm1 fome13 ns1688926 ex10"
.venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only $P --variants $V \
  --reps 1 --time-limit 300 --out benchmarks/crossover/ab55/mitt.jsonl >> benchmarks/crossover/ab55/mitt.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab55/done.txt
