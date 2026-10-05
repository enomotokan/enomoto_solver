#!/bin/bash
# 確定版での本計測 (Netlib 93 問 + Kennington・Mittelmann の前処理後 5000 行以上、両方式、600 秒)。
cd "$(dirname "$0")/../../.."
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington mittelmann \
  --sizes benchmarks/crossover/presolved_sizes.json --min-presolved-rows 5000 \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/final/results.jsonl \
  >> benchmarks/crossover/final/run.log 2>&1
echo ALL_DONE >> benchmarks/crossover/final/done.txt
