#!/bin/bash
# PDLP の比較 (内点法が失敗して二段解法へ戻る 6 問)。v0_* は Farkas 判定の符号の修正前の版での結果。
cd "$(dirname "$0")/../../.."
run() {
  name=$1; shift
  env "$@" .venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --methods ipm_crossover \
    --only dfl001 greenbeb perold pilot.we ship12l osa-30 \
    --reps 1 --time-limit 600 --out benchmarks/crossover/ab5/fail_$name.jsonl \
    >> benchmarks/crossover/ab5/fail_$name.log 2>&1
}
run base X=0
run pdlp1_it500 ENOMOTO_T_XO_PDLP=1 ENOMOTO_T_PDLP_MAX_ITERS=500
run pdlp1_it2000 ENOMOTO_T_XO_PDLP=1 ENOMOTO_T_PDLP_MAX_ITERS=2000
run pdlp2_it2000 ENOMOTO_T_XO_PDLP=2 ENOMOTO_T_PDLP_MAX_ITERS=2000
run pdlp1 ENOMOTO_T_XO_PDLP=1
run pdlp3_e6 ENOMOTO_T_XO_PDLP=3 ENOMOTO_T_PDLP_EPS=1e-6 ENOMOTO_T_PDLP_TIME=120
echo ALL_DONE >> benchmarks/crossover/ab5/fail_done.txt
