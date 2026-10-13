#!/bin/bash
# 2 回目の比較 (Mittelmann 10 問、各版 2 回を交互に):
#   1 スレッド (solve の既定): base / packed / packed + 細い supernode の併合 (alpha 16, 64)
#   4 スレッド: base / packed / packed + 併合 64 / packed + 併合 64 + 大きな前線のかたまりごとの Cholesky
cd "$(dirname "$0")/../.."
P="qap15 fome13 s250r10 s100 scpm1 ns1688926 neos-5052403-cygnet rail4284 square41 nug08-3rd"
K=ENOMOTO_T_MF_PACKED=1
.venv/bin/python scripts/mf_bench/run.py --only $P \
  --variants base= packed=$K a16=$K,ENOMOTO_T_MF_AMALG=16 a64=$K,ENOMOTO_T_MF_AMALG=64 \
  --reps 2 --time-limit 900 --out benchmarks/mf_bench/ab2_t1.jsonl >> benchmarks/mf_bench/ab2.log 2>&1
.venv/bin/python scripts/mf_bench/run.py --only $P --threads 4 \
  --variants base= packed=$K a64=$K,ENOMOTO_T_MF_AMALG=64 a64b=$K,ENOMOTO_T_MF_AMALG=64,ENOMOTO_T_MF_BCHOL=1 \
  --reps 2 --time-limit 900 --out benchmarks/mf_bench/ab2_t4.jsonl >> benchmarks/mf_bench/ab2.log 2>&1
echo ALL_DONE >> benchmarks/mf_bench/ab2.log
