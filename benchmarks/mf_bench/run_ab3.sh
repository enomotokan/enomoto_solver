#!/bin/bash
# 3 回目の比較 (4 スレッド、Mittelmann 10 問、各版 2 回を交互に): 並列のときの大きな前線の三角行列の解法を L21 の行のかたまり
# ごとに並列に (ENOMOTO_T_MF_PAR_TRSM)、L のかたまりの 0 埋めを並列に (ENOMOTO_T_MF_PAR_PREP)。基準は 2 回目の最良
# (台形 + 併合 64 + かたまりごとの Cholesky)。
cd "$(dirname "$0")/../.."
P="qap15 fome13 s250r10 s100 scpm1 ns1688926 neos-5052403-cygnet rail4284 square41 nug08-3rd"
B=ENOMOTO_T_MF_PACKED=1,ENOMOTO_T_MF_AMALG=64,ENOMOTO_T_MF_BCHOL=1
.venv/bin/python scripts/mf_bench/run.py --only $P --threads 4 \
  --variants a64b=$B a64bt=$B,ENOMOTO_T_MF_PAR_TRSM=1 a64btp=$B,ENOMOTO_T_MF_PAR_TRSM=1,ENOMOTO_T_MF_PAR_PREP=1 \
  --reps 2 --time-limit 900 --out benchmarks/mf_bench/ab3_t4.jsonl >> benchmarks/mf_bench/ab3.log 2>&1
echo ALL_DONE >> benchmarks/mf_bench/ab3.log
