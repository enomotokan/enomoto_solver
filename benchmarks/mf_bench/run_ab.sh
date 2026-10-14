#!/bin/bash
# マルチフロンタル法の更新行列の台形の形 (ENOMOTO_T_MF_PACKED) と親の前線への直接書き込み (ENOMOTO_T_MF_DIRECT) の比較
# (Mittelmann のうち内点法でマルチフロンタル法を使う 10 問、内点法 + クロスオーバー、各版 3 回を交互に)。
cd "$(dirname "$0")/../.."
.venv/bin/python scripts/mf_bench/run.py \
  --only qap15 fome13 s250r10 s100 scpm1 ns1688926 neos-5052403-cygnet rail4284 square41 nug08-3rd \
  --variants base= packed=ENOMOTO_T_MF_PACKED=1 direct=ENOMOTO_T_MF_DIRECT=1 both=ENOMOTO_T_MF_PACKED=1,ENOMOTO_T_MF_DIRECT=1 \
  --reps 3 --time-limit 900 --out benchmarks/mf_bench/ab.jsonl >> benchmarks/mf_bench/ab.log 2>&1
echo ALL_DONE >> benchmarks/mf_bench/ab.log
