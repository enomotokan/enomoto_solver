#!/bin/bash
# 内点法の Gondzio の多重中心性補正子 (ENOMOTO_T_IPM_GONDZIO) の回数の比較 (各問題で交互に計測)。射影の押し出しを省いた後の
# クロスオーバーで、第 15 回 (仕上げが長引いた) の結論が変わるかを見る。
cd "$(dirname "$0")/../../.."
V="default= g1=ENOMOTO_T_IPM_GONDZIO=1 g2=ENOMOTO_T_IPM_GONDZIO=2 g3=ENOMOTO_T_IPM_GONDZIO=3 g2small=ENOMOTO_T_IPM_GONDZIO=2,ENOMOTO_T_IPM_GONDZIO_SMALL_STEP=0.5"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab29/nk.jsonl >> benchmarks/crossover/ab29/nk.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab29/done.txt
