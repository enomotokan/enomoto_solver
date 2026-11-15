#!/bin/bash
# 課題 PDLG: 内点法が「最適」で終わらなかったときだけ PDLP (500 反復) の解から正則化 1e-8 で解き直す試作の比較 (109 問、3 回交互)。
# R500 = 受理できない点だけ解き直す、R500any = 受理できる未収束の点でも解き直す。
cd "$(dirname "$0")/../../.."
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --reps 3 --time-limit 600 \
  --out benchmarks/crossover/pdlg/r109_retry.jsonl --variants \
  base= \
  R500=ENOMOTO_T_XO_IPM_RETRY_PDLP=500 \
  R500any=ENOMOTO_T_XO_IPM_RETRY_PDLP=500,ENOMOTO_T_XO_IPM_RETRY_PDLP_ANY=1 \
  >> benchmarks/crossover/pdlg/r109_retry.log 2>&1
echo ALL_DONE >> benchmarks/crossover/pdlg/r109_retry.log
