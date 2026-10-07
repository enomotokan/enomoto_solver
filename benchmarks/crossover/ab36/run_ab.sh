#!/bin/bash
# 内点法 + クロスオーバーの比較 (各問題で交互に計測):
#   old: 従来 (基底の選択の行の非零を行列全体で数える、Megiddo 式の押し出しで稠密切替を早めない)
#   new: 基底の選択の修正 + Megiddo 式の押し出しの間だけ LU の稠密切替を早める
#   accept: new + Megiddo 式の押し出し後の頂点を内点法の双対の下界で確かめて返す (ENOMOTO_T_XO_ACCEPT_GAP=1e-8)
cd "$(dirname "$0")/../../.."
P="qap15 s250r10 datt256 scpm1 nug08-3rd fome13 ns1688926 ex10"
V="old=ENOMOTO_T_XO_LI_ROWCNT_ALL=1,ENOMOTO_T_XO_MEGIDDO_DENSE=0 new= accept=ENOMOTO_T_XO_ACCEPT_GAP=1e-8"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab36/nk.jsonl >> benchmarks/crossover/ab36/nk.log 2>&1
.venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only $P --variants $V highs_ipm= \
  --reps 1 --time-limit 300 --out benchmarks/crossover/ab36/mitt.jsonl >> benchmarks/crossover/ab36/mitt.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab36/done.txt
