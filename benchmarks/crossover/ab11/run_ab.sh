#!/bin/bash
# 同時実行で二段解法の下界を内点法に渡す案の比較 (Netlib 93 問 + Kennington 16 問、同時実行の閾値を 0 にして
# 全問を同時実行する)。途中で止まっても続きから再開できる。
cd "$(dirname "$0")/../../.."
run() {
  name=$1; shift
  env ENOMOTO_T_RACE_MIN_ROWS=0 "$@" .venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --methods race \
    --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab11/$name.jsonl \
    >> benchmarks/crossover/ab11/$name.log 2>&1
}
run race_base X=0
run race_bound1e-6 ENOMOTO_T_XO_BOUND_GAP=1e-6
run race_bound1e-4 ENOMOTO_T_XO_BOUND_GAP=1e-4
echo ALL_DONE >> benchmarks/crossover/ab11/done.txt
