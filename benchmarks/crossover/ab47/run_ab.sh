#!/bin/bash
# 自動選択 (RootSolver::Auto) で二段解法と内点法 + クロスオーバーを同時に解く行数の下限 (ENOMOTO_T_RACE_MIN_ROWS)。
# 小さな問題 (非零 2 万未満) の同時実行は、二段解法を呼び出し元のスレッドで、内点法を使い回しの 1 スレッドのプールで解く。
#   r5000: 今の既定 (5000 行以上だけ同時実行、それ未満は二段解法だけ)
#   r1000 / r200 / r0: 下限を 1000 / 200 / 0 行に下げる
cd "$(dirname "$0")/../../.."
V="r5000:race= r1000:race=ENOMOTO_T_RACE_MIN_ROWS=1000 r200:race=ENOMOTO_T_RACE_MIN_ROWS=200 r0:race=ENOMOTO_T_RACE_MIN_ROWS=0"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab47/nk.jsonl >> benchmarks/crossover/ab47/nk.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab47/done.txt
