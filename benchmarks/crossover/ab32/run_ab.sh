#!/bin/bash
# qap15・scpm1 の調査で入れた変更の退行確認 (各問題で交互に計測): 基底の選択で非零の少ない行をピボットに選ぶ
# (ENOMOTO_T_XO_LI_MARKOWITZ)、Megiddo 式の押し出しの時間の上限 (ENOMOTO_T_XO_MEGIDDO_TIME_FACTOR)、大きな因子の並列分解
# (ENOMOTO_T_FACTOR_PAR_NNZ)。
cd "$(dirname "$0")/../../.."
V="default= nomark=ENOMOTO_T_XO_LI_MARKOWITZ=0 nodeadline=ENOMOTO_T_XO_MEGIDDO_TIME_FACTOR=1000000 seqfactor=ENOMOTO_T_FACTOR_PAR_NNZ=0"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab32/nk.jsonl >> benchmarks/crossover/ab32/nk.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab32/done.txt
