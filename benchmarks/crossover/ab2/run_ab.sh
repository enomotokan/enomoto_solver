#!/bin/bash
# Fable の調査を受けた改良案の比較計測 (Netlib 93 問 + Kennington 16 問、内点法 + クロスオーバーのみ)。
cd "$(dirname "$0")/../../.."
run() {
  name=$1; shift
  env "$@" .venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --methods ipm_crossover \
    --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab2/$name.jsonl \
    >> benchmarks/crossover/ab2/$name.log 2>&1
}
run base X=0
run factor_seq ENOMOTO_T_FACTOR_SEQ=1
run noimprove10 ENOMOTO_T_IPM_NOIMPROVE=10
run vyrel1e-3 ENOMOTO_T_XO_VY_REL=1e-3
run dualskip0.7 ENOMOTO_T_XO_DUAL_SKIP_FRAC=0.7
run dualtime2 ENOMOTO_T_XO_DUAL_TIME_FACTOR=2
echo ALL_DONE >> benchmarks/crossover/ab2/done.txt
