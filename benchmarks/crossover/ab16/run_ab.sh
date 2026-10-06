#!/bin/bash
# 正則化の下限の比較 (正規方程式の悪条件による収束後の発散への対策)。基準は第 15 回の採用設定
# (benchmarks/crossover/ab15/reg_blowup.jsonl・mitt_reg_blowup.jsonl、同じビルド)。途中で止まっても続きから再開できる。
cd "$(dirname "$0")/../../.."
P="qap15 s250r10 datt256 scpm1 nug08-3rd fome13 ns1688926 ex10 supportcase10"
run() {
  name=$1; shift
  env "$@" .venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --methods ipm_crossover \
    --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab16/$name.jsonl \
    >> benchmarks/crossover/ab16/$name.log 2>&1
  env "$@" .venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only $P --methods ipm_crossover \
    --reps 1 --time-limit 300 --out benchmarks/crossover/ab16/mitt_$name.jsonl >> benchmarks/crossover/ab16/mitt_$name.log 2>&1
}
run rho1e-10 ENOMOTO_T_IPM_RHO_MIN=1e-10
run floor1e-11 ENOMOTO_T_IPM_RHO_MIN=1e-11 ENOMOTO_T_IPM_DELTA_MIN=1e-11
run floor1e-12 ENOMOTO_T_IPM_RHO_MIN=1e-12 ENOMOTO_T_IPM_DELTA_MIN=1e-12
echo ALL_DONE >> benchmarks/crossover/ab16/done.txt
