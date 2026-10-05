#!/bin/bash
# Megiddo 式の押し出しの比較 (Netlib 93 問 + Kennington 16 問)。既定値は第 3 回で決めたもの
# (逐次分解・双対の押し出しの時間打ち切り 2 倍・|D| < 0.7 m で省略)。
cd "$(dirname "$0")/../../.."
run() {
  name=$1; shift
  env "$@" .venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --methods ipm_crossover \
    --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab4/$name.jsonl \
    >> benchmarks/crossover/ab4/$name.log 2>&1
}
run base X=0
run megiddo ENOMOTO_T_XO_MEGIDDO=1
run megiddo_sw100 ENOMOTO_T_XO_MEGIDDO=1 ENOMOTO_T_XO_MEGIDDO_SWITCH=100
run megiddo_sw1000 ENOMOTO_T_XO_MEGIDDO=1 ENOMOTO_T_XO_MEGIDDO_SWITCH=1000
echo ALL_DONE >> benchmarks/crossover/ab4/done.txt
