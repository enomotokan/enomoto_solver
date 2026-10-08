#!/bin/bash
# 内点法の終盤の跳ね上がりへの手当て (跳ね上がりが 2 反復続いたときだけ。各問題で交互に計測):
#   default: 今の既定
#   jump2: 正則化が下限にある間に主残差がそれまでの最小の 100 倍以上の状態が 2 反復続いたら、ρ・δ を一度だけ 1e-8 に上げる
cd "$(dirname "$0")/../../.."
V="default= jump2=ENOMOTO_T_IPM_STALL_BUMP=1e-8,ENOMOTO_T_IPM_STALL_JUMP=100,ENOMOTO_T_IPM_STALL_JUMP_K=2"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab52/nk.jsonl >> benchmarks/crossover/ab52/nk.log 2>&1
P="qap15 s250r10 scpm1 fome13 ns1688926 ex10"
.venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only $P --variants $V \
  --reps 2 --single-run-above 1000 --time-limit 300 --out benchmarks/crossover/ab52/mitt.jsonl >> benchmarks/crossover/ab52/mitt.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab52/done.txt
