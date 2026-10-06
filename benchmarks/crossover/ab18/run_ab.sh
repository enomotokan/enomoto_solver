#!/bin/bash
# 内点法の改良 (正則化の初期値・下限、発散の打ち切り、双対化) を取り込んだ上で、非基底の検出を
# PDHG の 1 歩 (既定、案 3) と γ = 1 (ENOMOTO_T_XO_GAMMA_PDHG=0) で比べる。途中で止まっても続きから再開できる。
cd "$(dirname "$0")/../../.."
P="qap15 s250r10 datt256 scpm1 nug08-3rd fome13 ns1688926 ex10"
mitt() {
  name=$1; shift
  env ENOMOTO_T_XO_QUALITY=1 "$@" .venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only $P --methods ipm_crossover \
    --reps 1 --time-limit 300 --out benchmarks/crossover/ab18/mitt_$name.jsonl >> benchmarks/crossover/ab18/mitt_$name.log 2>&1
}
run() {
  name=$1; shift
  env ENOMOTO_T_XO_QUALITY=1 "$@" .venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --methods ipm_crossover \
    --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab18/$name.jsonl \
    >> benchmarks/crossover/ab18/$name.log 2>&1
}
run gamma1 ENOMOTO_T_XO_GAMMA_PDHG=0
run pdhg3 X=0
mitt gamma1 ENOMOTO_T_XO_GAMMA_PDHG=0
mitt pdhg3 X=0
echo ALL_DONE >> benchmarks/crossover/ab18/done.txt
