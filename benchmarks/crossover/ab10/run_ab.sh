#!/bin/bash
# 失敗する問題への対策の比較 (Netlib 93 問 + Kennington 16 問)。途中で止まっても続きから再開できる。
cd "$(dirname "$0")/../../.."
run() {
  name=$1; shift
  env "$@" .venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --methods ipm_crossover \
    --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab10/$name.jsonl \
    >> benchmarks/crossover/ab10/$name.log 2>&1
}
run base X=0
run gap1e8 ENOMOTO_T_XO_ACCEPT_GAP_REL=1e8
run gondzio_small ENOMOTO_T_IPM_GONDZIO=2 ENOMOTO_T_IPM_GONDZIO_SMALL_STEP=0.1
run both ENOMOTO_T_XO_ACCEPT_GAP_REL=1e8 ENOMOTO_T_IPM_GONDZIO=2 ENOMOTO_T_IPM_GONDZIO_SMALL_STEP=0.1
echo ALL_DONE >> benchmarks/crossover/ab10/done.txt
