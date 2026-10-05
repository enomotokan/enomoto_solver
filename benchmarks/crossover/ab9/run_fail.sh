#!/bin/bash
# 内点法が失敗する問題への対策の比較 (失敗 6 問、正則化の下げ方は既定の mode1)。途中で止まっても再開できる。
cd "$(dirname "$0")/../../.."
run() {
  name=$1; shift
  env "$@" .venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --methods ipm_crossover \
    --only dfl001 greenbeb perold pilot.we ship12l osa-30 \
    --reps 1 --time-limit 600 --out benchmarks/crossover/ab9/fail_$name.jsonl \
    >> benchmarks/crossover/ab9/fail_$name.log 2>&1
}
run base X=0
run gap1e8 ENOMOTO_T_XO_ACCEPT_GAP_REL=1e8
run nan5 ENOMOTO_T_IPM_NAN_RECOVER=5
run gondzio_small ENOMOTO_T_IPM_GONDZIO=2 ENOMOTO_T_IPM_GONDZIO_SMALL_STEP=0.1
run all3 ENOMOTO_T_XO_ACCEPT_GAP_REL=1e8 ENOMOTO_T_IPM_NAN_RECOVER=5 ENOMOTO_T_IPM_GONDZIO=2 ENOMOTO_T_IPM_GONDZIO_SMALL_STEP=0.1
echo ALL_DONE >> benchmarks/crossover/ab9/fail_done.txt
