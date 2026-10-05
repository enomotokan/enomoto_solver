#!/bin/bash
# 段階 A + 内点法、PDLP の近接中心 (500 反復) の比較 (Netlib 93 問 + Kennington 16 問)。
# 既定値は第 4 回までで決めたもの (Megiddo 式あり、Farkas 判定の修正後)。
cd "$(dirname "$0")/../../.."
run() {
  name=$1; shift
  env "$@" .venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --methods ipm_crossover \
    --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab6/$name.jsonl \
    >> benchmarks/crossover/ab6/$name.log 2>&1
}
run base X=0
run staged ENOMOTO_T_IPM_STAGED=1
run pdlp1_it500 ENOMOTO_T_XO_PDLP=1 ENOMOTO_T_PDLP_MAX_ITERS=500
echo ALL_DONE >> benchmarks/crossover/ab6/done.txt
