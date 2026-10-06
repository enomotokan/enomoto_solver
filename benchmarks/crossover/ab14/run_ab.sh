#!/bin/bash
# 非基底の検出の比 γ を PDLP の主の歩幅 τ_j = (η/ω) dc_j² にする案 (ENOMOTO_T_XO_GAMMA_PDHG) と、
# γ の全体の位置への敏感さ (ENOMOTO_T_XO_GAMMA_MULT) の比較。途中で止まっても続きから再開できる。
cd "$(dirname "$0")/../../.."
run() {
  name=$1; shift
  env ENOMOTO_T_XO_QUALITY=1 "$@" .venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --methods ipm_crossover \
    --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab14/$name.jsonl \
    >> benchmarks/crossover/ab14/$name.log 2>&1
}
run base X=0
run pdhg1 ENOMOTO_T_XO_GAMMA_PDHG=1
run pdhg2 ENOMOTO_T_XO_GAMMA_PDHG=2
run pdhg3 ENOMOTO_T_XO_GAMMA_PDHG=3
run base_x0.01 ENOMOTO_T_XO_GAMMA_MULT=0.01
run base_x100 ENOMOTO_T_XO_GAMMA_MULT=100
run pdhg1_x0.01 ENOMOTO_T_XO_GAMMA_PDHG=1 ENOMOTO_T_XO_GAMMA_MULT=0.01
run pdhg1_x100 ENOMOTO_T_XO_GAMMA_PDHG=1 ENOMOTO_T_XO_GAMMA_MULT=100
echo ALL_DONE >> benchmarks/crossover/ab14/done.txt
