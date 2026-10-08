#!/bin/bash
# 内点法の終盤の不正確な Newton 方向への手当て (各問題で交互に計測):
#   default: 今の既定
#   acc: 予測子の Newton 系の相対残差が 1e-6 を超え、しかも相対残差の最悪値が 0.95 倍未満に減らない反復が 3 回続いたら、
#        その反復だけ ρ・δ を一時的に 1e-10 (続けば 100 倍ずつ) に強めて解く (ENOMOTO_T_IPM_SOLVE_ACC2=1e-6、_K=3)
#   jump2: 第 52 回の跳ね上がり対策
cd "$(dirname "$0")/../../.."
V="default= acc=ENOMOTO_T_IPM_SOLVE_ACC2=1e-6,ENOMOTO_T_IPM_SOLVE_ACC2_K=3 jump2=ENOMOTO_T_IPM_STALL_BUMP=1e-8,ENOMOTO_T_IPM_STALL_JUMP=100,ENOMOTO_T_IPM_STALL_JUMP_K=2"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab54/nk.jsonl >> benchmarks/crossover/ab54/nk.log 2>&1
P="qap15 s250r10 scpm1 fome13 ns1688926 ex10"
.venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only $P --variants $V \
  --reps 1 --time-limit 300 --out benchmarks/crossover/ab54/mitt.jsonl >> benchmarks/crossover/ab54/mitt.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab54/done.txt
