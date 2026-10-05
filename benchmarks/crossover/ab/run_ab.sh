#!/bin/bash
# 内点法の改良案 6・7 の比較計測 (Netlib 93 問 + Kennington 16 問、内点法 + クロスオーバーのみ)。
cd "$(dirname "$0")/../../.."
run() {
  name=$1; shift
  env "$@" .venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --methods ipm_crossover \
    --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab/$name.jsonl \
    >> benchmarks/crossover/ab/$name.log 2>&1
}
run base X=0
run gondzio2 ENOMOTO_T_IPM_GONDZIO=2
run prox_always ENOMOTO_T_IPM_PROX_ALWAYS=1
run eps1e-6 ENOMOTO_T_IPM_EPS=1e-6
echo ALL_DONE >> benchmarks/crossover/ab/done.txt
