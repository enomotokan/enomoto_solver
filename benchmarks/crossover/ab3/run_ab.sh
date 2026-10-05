#!/bin/bash
# 第 2 回で通った案の組み合わせの確認 (Netlib 93 問 + Kennington 16 問)。
cd "$(dirname "$0")/../../.."
run() {
  name=$1; shift
  env "$@" .venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --methods ipm_crossover \
    --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab3/$name.jsonl \
    >> benchmarks/crossover/ab3/$name.log 2>&1
}
run seq_time2 ENOMOTO_T_FACTOR_SEQ=1 ENOMOTO_T_XO_DUAL_TIME_FACTOR=2
run seq_time2_skip0.7 ENOMOTO_T_FACTOR_SEQ=1 ENOMOTO_T_XO_DUAL_TIME_FACTOR=2 ENOMOTO_T_XO_DUAL_SKIP_FRAC=0.7
echo ALL_DONE >> benchmarks/crossover/ab3/done.txt
