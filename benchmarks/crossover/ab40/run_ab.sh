#!/bin/bash
# PDHG の歩幅 τ_j の前処理: 共通の前処理で Ruiz 済みなので Pock–Chambolle だけにする案 (各問題で交互に計測):
#   default: Ruiz 10 回 + Pock–Chambolle
#   ruiz0: Pock–Chambolle だけ (ENOMOTO_T_PDLP_RUIZ_ITERS=0)
cd "$(dirname "$0")/../../.."
V="default= ruiz0=ENOMOTO_T_PDLP_RUIZ_ITERS=0"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab40/nk.jsonl >> benchmarks/crossover/ab40/nk.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab40/done.txt
