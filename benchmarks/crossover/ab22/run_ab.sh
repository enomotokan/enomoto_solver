#!/bin/bash
# 基底の足りない列を、スラックの前に内点法の被約費用 |s_j| の小さい列から埋める案 (ENOMOTO_T_XO_FILL_BY_S=K) の比較。
# 途中で止まっても続きから再開できる。
cd "$(dirname "$0")/../../.."
run() {
  name=$1; shift
  env ENOMOTO_T_XO_QUALITY=1 "$@" .venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --methods ipm_crossover \
    --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab22/$name.jsonl \
    >> benchmarks/crossover/ab22/$name.log 2>&1
}
run base X=0
run fill4 ENOMOTO_T_XO_FILL_BY_S=4
echo ALL_DONE >> benchmarks/crossover/ab22/done.txt
