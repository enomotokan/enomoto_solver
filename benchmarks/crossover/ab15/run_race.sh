#!/bin/bash
# 第 15 回の追加: 既定の同時実行 (auto) での従来設定と新しい既定の比較。途中で止まっても続きから再開できる。
cd "$(dirname "$0")/../../.."
OLD="ENOMOTO_T_IPM_REG0=0.1 ENOMOTO_T_IPM_RHO_MIN=1e-10 ENOMOTO_T_IPM_DELTA_MIN=1e-10 ENOMOTO_T_IPM_GONDZIO=0 ENOMOTO_T_IPM_BLOWUP=0"
P="qap15 s250r10 datt256 scpm1 nug08-3rd fome13 ns1688926 ex10 supportcase10"
for v in base new; do
  E=$([ $v = base ] && echo "$OLD" || echo "X=0")
  env $E .venv/bin/python scripts/crossover_bench/run.py --set kennington --methods race \
    --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab15/race_$v.jsonl >> benchmarks/crossover/ab15/race_$v.log 2>&1
  env $E .venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only $P --methods race \
    --reps 1 --time-limit 300 --out benchmarks/crossover/ab15/mitt_race_$v.jsonl >> benchmarks/crossover/ab15/mitt_race_$v.log 2>&1
done
echo ALL_DONE >> benchmarks/crossover/ab15/done_race.txt
