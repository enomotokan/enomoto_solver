#!/bin/bash
# クロスオーバーの仕上げを二段解法の主ループ (与えた基底から開始) で行う案の比較。第 12 回が終わってから動かす。
# 途中で止まっても続きから再開できる。
cd "$(dirname "$0")/../../.."
until [ -e benchmarks/crossover/ab12/done.txt ]; do sleep 60; done
P="qap15 s250r10 datt256 scpm1 nug08-3rd fome13 ns1688926 ex10"
mitt() {
  name=$1; shift
  env "$@" .venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only $P --methods ipm_crossover \
    --reps 1 --time-limit 300 --out benchmarks/crossover/ab13/mitt_$name.jsonl >> benchmarks/crossover/ab13/mitt_$name.log 2>&1
}
run() {
  name=$1; shift
  env "$@" .venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --methods ipm_crossover \
    --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab13/$name.jsonl \
    >> benchmarks/crossover/ab13/$name.log 2>&1
}
run cleanup_main ENOMOTO_T_XO_CLEANUP_MAIN=1
mitt cleanup_main ENOMOTO_T_XO_CLEANUP_MAIN=1
mitt switch10_cleanup_main ENOMOTO_T_IPM_SWITCH_AUG=10 ENOMOTO_T_XO_CLEANUP_MAIN=1
echo ALL_DONE >> benchmarks/crossover/ab13/done.txt
