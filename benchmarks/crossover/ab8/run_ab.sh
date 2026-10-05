#!/bin/bash
# 正則化の下げ方の比較 (Netlib 93 問 + Kennington 16 問)。途中で止まっても続きから再開できる。
cd "$(dirname "$0")/../../.."
run() {
  name=$1; shift
  env "$@" .venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --methods ipm_crossover \
    --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab8/$name.jsonl \
    >> benchmarks/crossover/ab8/$name.log 2>&1
}
run base X=0
run mode1 ENOMOTO_T_IPM_REG_MODE=1
run mode3_k1 ENOMOTO_T_IPM_REG_MODE=3 ENOMOTO_T_IPM_REG_KAPPA=1
run mode2_k0.01 ENOMOTO_T_IPM_REG_MODE=2 ENOMOTO_T_IPM_REG_KAPPA=0.01
echo ALL_DONE >> benchmarks/crossover/ab8/done.txt
