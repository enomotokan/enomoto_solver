#!/bin/bash
# run_mitt.sh の続き (neos などの MPS を取得した後。双対の押し出しは主実行可能な頂点のときだけ、特異のときの取り消しなしの最終版)。
# 2 本を同時に走らせる: run_mitt2.sh <出力名> <問題...>
cd "$(dirname "$0")/../../.."
OUT=$1; shift
V="back1=ENOMOTO_T_XO_DUALIZE_BACK=1,ENOMOTO_T_XO_DUALIZE_MAX_COL_NNZ=45 back0=ENOMOTO_T_XO_DUALIZE_BACK=0"
.venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only "$@" --variants $V \
  --reps 1 --time-limit 600 --out benchmarks/crossover/ab63/$OUT.jsonl >> benchmarks/crossover/ab63/$OUT.log 2>&1
