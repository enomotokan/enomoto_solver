#!/bin/bash
# 基底の選択で列を受理する閾値 (ENOMOTO_T_XO_LI_TOL、消去後の成分 / 列の最大値) の比較。
# 1e-9 ではほぼ一次従属な列も通り、悪条件な基底で仕上げが長引く (pilot87・pilot.ja)。途中で止まっても続きから再開できる。
cd "$(dirname "$0")/../../.."
run() {
  name=$1; shift
  env ENOMOTO_T_XO_QUALITY=1 "$@" .venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --methods ipm_crossover \
    --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab21/$name.jsonl \
    >> benchmarks/crossover/ab21/$name.log 2>&1
}
run base X=0
run li1e-4 ENOMOTO_T_XO_LI_TOL=1e-4
run li1e-2 ENOMOTO_T_XO_LI_TOL=1e-2
echo ALL_DONE >> benchmarks/crossover/ab21/done.txt
