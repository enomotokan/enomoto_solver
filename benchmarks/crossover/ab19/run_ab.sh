#!/bin/bash
# 内点法 + クロスオーバーを独立な成分ごとに分けて行う案 (既定) と分けない案 (ENOMOTO_T_XO_SPLIT_MIN_VARS=0) の比較。
# 途中で止まっても続きから再開できる。
cd "$(dirname "$0")/../../.."
P="qap15 s250r10 datt256 scpm1 nug08-3rd fome13 ns1688926 ex10"
mitt() {
  name=$1; shift
  env ENOMOTO_T_XO_QUALITY=1 "$@" .venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only $P --methods ipm_crossover \
    --reps 1 --time-limit 300 --out benchmarks/crossover/ab19/mitt_$name.jsonl >> benchmarks/crossover/ab19/mitt_$name.log 2>&1
}
run() {
  name=$1; shift
  env ENOMOTO_T_XO_QUALITY=1 "$@" .venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --methods ipm_crossover \
    --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab19/$name.jsonl \
    >> benchmarks/crossover/ab19/$name.log 2>&1
}
run nosplit ENOMOTO_T_XO_SPLIT_MIN_VARS=0
run split X=0
mitt nosplit ENOMOTO_T_XO_SPLIT_MIN_VARS=0
mitt split X=0
echo ALL_DONE >> benchmarks/crossover/ab19/done.txt
