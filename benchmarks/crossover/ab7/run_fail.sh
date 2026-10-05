#!/bin/bash
# 内点法の正則化の下げ方 (ENOMOTO_T_IPM_REG_MODE) の比較 (内点法が失敗する 6 問)。
cd "$(dirname "$0")/../../.."
run() {
  name=$1; shift
  [ -s benchmarks/crossover/ab7/fail_$name.jsonl ] && [ $(wc -l < benchmarks/crossover/ab7/fail_$name.jsonl) -ge 6 ] && return
  env "$@" .venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --methods ipm_crossover \
    --only dfl001 greenbeb perold pilot.we ship12l osa-30 \
    --reps 1 --time-limit 600 --out benchmarks/crossover/ab7/fail_$name.jsonl \
    >> benchmarks/crossover/ab7/fail_$name.log 2>&1
}
run base X=0
run mode1 ENOMOTO_T_IPM_REG_MODE=1
run mode2_k1 ENOMOTO_T_IPM_REG_MODE=2 ENOMOTO_T_IPM_REG_KAPPA=1
run mode2_k0.1 ENOMOTO_T_IPM_REG_MODE=2 ENOMOTO_T_IPM_REG_KAPPA=0.1
run mode2_k0.01 ENOMOTO_T_IPM_REG_MODE=2 ENOMOTO_T_IPM_REG_KAPPA=0.01
run mode3_k1 ENOMOTO_T_IPM_REG_MODE=3 ENOMOTO_T_IPM_REG_KAPPA=1
run mode3_k0.1 ENOMOTO_T_IPM_REG_MODE=3 ENOMOTO_T_IPM_REG_KAPPA=0.1
echo ALL_DONE >> benchmarks/crossover/ab7/fail_done.txt
