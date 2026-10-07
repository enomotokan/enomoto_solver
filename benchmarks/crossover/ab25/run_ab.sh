#!/bin/bash
# 押し出しを省く案の比較 (各問題で交互に計測): 双対の押し出しを常に省く (ENOMOTO_T_XO_DUAL_SKIP_FRAC=2)、
# さらに射影による主の押し出しも省いて基底の選択 + Megiddo 式の押し出しに任せる (ENOMOTO_T_XO_MEGIDDO_SWITCH=1e9)。
cd "$(dirname "$0")/../../.."
until [ -e benchmarks/crossover/ab24/done.txt ]; do sleep 30; done
P="qap15 s250r10 datt256 scpm1 nug08-3rd fome13 ns1688926 ex10"
V="default= nodual=ENOMOTO_T_XO_DUAL_SKIP_FRAC=2 nopush=ENOMOTO_T_XO_DUAL_SKIP_FRAC=2,ENOMOTO_T_XO_MEGIDDO_SWITCH=1000000000"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab25/nk.jsonl >> benchmarks/crossover/ab25/nk.log 2>&1
.venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only $P --variants $V \
  --reps 1 --time-limit 300 --out benchmarks/crossover/ab25/mitt.jsonl >> benchmarks/crossover/ab25/mitt.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab25/done.txt
