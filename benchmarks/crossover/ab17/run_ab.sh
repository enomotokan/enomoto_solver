#!/bin/bash
# 第 16 回の案 3 (PDHG を 1 歩進めて検出) の測り直し、案 2 の歩幅で 1 歩進める案 4、Mittelmann の小さめの 8 問。
# 途中で止まっても続きから再開できる。
cd "$(dirname "$0")/../../.."
P="qap15 s250r10 datt256 scpm1 nug08-3rd fome13 ns1688926 ex10"
mitt() {
  name=$1; shift
  env ENOMOTO_T_XO_QUALITY=1 "$@" .venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only $P --methods ipm_crossover \
    --reps 1 --time-limit 300 --out benchmarks/crossover/ab17/mitt_$name.jsonl >> benchmarks/crossover/ab17/mitt_$name.log 2>&1
}
run() {
  name=$1; shift
  env ENOMOTO_T_XO_QUALITY=1 "$@" .venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --methods ipm_crossover \
    --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab17/$name.jsonl \
    >> benchmarks/crossover/ab17/$name.log 2>&1
}
run base X=0
run pdhg3 ENOMOTO_T_XO_GAMMA_PDHG=3
run pdhg4 ENOMOTO_T_XO_GAMMA_PDHG=4
mitt base X=0
mitt pdhg3 ENOMOTO_T_XO_GAMMA_PDHG=3
mitt pdhg4 ENOMOTO_T_XO_GAMMA_PDHG=4
echo ALL_DONE >> benchmarks/crossover/ab17/done.txt
