#!/bin/bash
# 第 15 回の続き: 正則化の変更 + 発散の打ち切り (Gondzio なし、新しい既定) を同じ問題で計測する。
# 第 15 回 (run_ab.sh) が終わってから再ビルドして走らせる。途中で止まっても続きから再開できる。
cd "$(dirname "$0")/../../.."
until [ -e benchmarks/crossover/ab15/done.txt ]; do sleep 30; done
if [ ! -e benchmarks/crossover/ab15/built2.txt ]; then
  .venv/bin/maturin develop --release > benchmarks/crossover/ab15/build2.log 2>&1 && date -u > benchmarks/crossover/ab15/built2.txt || exit 1
fi
P="qap15 s250r10 datt256 scpm1 nug08-3rd fome13 ns1688926 ex10 supportcase10"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --methods ipm_crossover \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab15/reg_blowup.jsonl \
  >> benchmarks/crossover/ab15/reg_blowup.log 2>&1
.venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only $P --methods ipm_crossover \
  --reps 1 --time-limit 300 --out benchmarks/crossover/ab15/mitt_reg_blowup.jsonl >> benchmarks/crossover/ab15/mitt_reg_blowup.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab15/done2.txt
