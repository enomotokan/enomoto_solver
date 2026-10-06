#!/bin/bash
# 正規方程式が停滞したら拡大系に切り替える案の比較。Mittelmann の小さめの 8 問 (内点法 + クロスオーバー単独) と、
# Netlib 93 問 + Kennington 16 問。途中で止まっても続きから再開できる。
cd "$(dirname "$0")/../../.."
P="qap15 s250r10 datt256 scpm1 nug08-3rd fome13 ns1688926 ex10"
mitt() {
  name=$1; shift
  env "$@" .venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only $P --methods ipm_crossover \
    --reps 1 --time-limit 300 --out benchmarks/crossover/ab12/mitt_$name.jsonl >> benchmarks/crossover/ab12/mitt_$name.log 2>&1
}
run() {
  name=$1; shift
  env "$@" .venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --methods ipm_crossover \
    --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab12/$name.jsonl \
    >> benchmarks/crossover/ab12/$name.log 2>&1
}
mitt switch10 ENOMOTO_T_IPM_SWITCH_AUG=10
mitt aug ENOMOTO_IPM_AUGMENTED=1
run base X=0
run switch10 ENOMOTO_T_IPM_SWITCH_AUG=10
run switch20 ENOMOTO_T_IPM_SWITCH_AUG=20
echo ALL_DONE >> benchmarks/crossover/ab12/done.txt
