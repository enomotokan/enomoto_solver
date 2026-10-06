#!/bin/bash
# 内点法の双対化 (単体法が双対化する問題は内点法でも双対 LP を解く) の比較。途中で止まっても続きから再開できる。
cd "$(dirname "$0")/../../.."
P="supportcase10 neos neos3 neos-5251015 physiciansched3-3 rmine15 graph40-40"
mitt() {
  name=$1; shift
  env "$@" .venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only $P --methods ipm_crossover \
    --reps 1 --time-limit 600 --out benchmarks/crossover/ab14/mitt_$name.jsonl >> benchmarks/crossover/ab14/mitt_$name.log 2>&1
}
mitt dualize ENOMOTO_T_XO_DUALIZE=1
mitt base ENOMOTO_T_XO_DUALIZE=0
echo ALL_DONE >> benchmarks/crossover/ab14/done.txt
