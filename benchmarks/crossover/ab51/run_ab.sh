#!/bin/bash
# 内点法の終盤の停滞への手当て (各問題で交互に計測):
#   default: 今の既定
#   jump: 正則化が下限にある間に主残差がそれまでの最小の 100 倍以上に跳ね上がったら、ρ・δ を一度だけ 1e-8 に上げる
#         (ENOMOTO_T_IPM_STALL_BUMP=1e-8、ENOMOTO_T_IPM_STALL_JUMP=100)
#   jump_pc: jump + 非零 10 万以上の問題だけ内点法の前に Pock–Chambolle、検出はそろえた空間で γ = 1
cd "$(dirname "$0")/../../.."
J="ENOMOTO_T_IPM_STALL_BUMP=1e-8,ENOMOTO_T_IPM_STALL_JUMP=100"
V="default= jump=$J jump_pc=$J,ENOMOTO_T_XO_IPM_PC=1,ENOMOTO_T_XO_GAMMA_PDHG=6,ENOMOTO_T_XO_IPM_PC_MIN_NNZ=100000"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab51/nk.jsonl >> benchmarks/crossover/ab51/nk.log 2>&1
P="qap15 s250r10 scpm1 fome13 ns1688926 ex10"
.venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only $P --variants $V \
  --reps 1 --time-limit 300 --out benchmarks/crossover/ab51/mitt.jsonl >> benchmarks/crossover/ab51/mitt.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab51/done.txt
