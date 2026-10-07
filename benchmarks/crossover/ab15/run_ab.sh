#!/bin/bash
# 内点法の正則化の初期値・下限 (0.1・1e-10 → 1e-4・1e-13) と Gondzio の補正子 (0 → 2 回) の比較
# (analysis/fable_ipm_vs_clarabel_20261006.md §5.1・§5.2)。途中で止まっても続きから再開できる。
cd "$(dirname "$0")/../../.."
OLD="ENOMOTO_T_IPM_REG0=0.1 ENOMOTO_T_IPM_RHO_MIN=1e-10 ENOMOTO_T_IPM_DELTA_MIN=1e-10 ENOMOTO_T_IPM_GONDZIO=0"
P="qap15 s250r10 datt256 scpm1 nug08-3rd fome13 ns1688926 ex10 supportcase10"
mitt() {
  name=$1; shift
  env "$@" .venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only $P --methods ipm_crossover \
    --reps 1 --time-limit 300 --out benchmarks/crossover/ab15/mitt_$name.jsonl >> benchmarks/crossover/ab15/mitt_$name.log 2>&1
}
run() {
  name=$1; shift
  env "$@" .venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --methods ipm_crossover \
    --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab15/$name.jsonl \
    >> benchmarks/crossover/ab15/$name.log 2>&1
}
run base $OLD
run reg ENOMOTO_T_IPM_GONDZIO=0
run reg_gondzio X=0
mitt base $OLD
mitt reg ENOMOTO_T_IPM_GONDZIO=0
mitt reg_gondzio X=0
echo ALL_DONE >> benchmarks/crossover/ab15/done.txt
