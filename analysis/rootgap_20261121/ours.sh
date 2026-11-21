#!/bin/sh
# 当方を 1 スレッド・種 1・600 秒で 1 回ずつ解き、時刻つきのログ (ENOMOTO_MIP_LOG=1) を logs/ours_<問題>_t1.log に残す。
# 使い方: sh ours.sh [問題…]  (環境変数はそのまま子に渡る。札を変えるなら TAG=...)
cd "$(dirname "$0")"
R=$(cd ../.. && pwd)
TAG=${TAG:-ours}
PS=${*:-"exp-1-500-5-5 app1-1 neos-1171448 mc11 beasleyC3 p200x1188c decomp2"}
for p in $PS; do
  ENOMOTO_T_MIP_SEED=1 python "$R/analysis/smallprob_overhead_20261028/prof_run.py" ours "$R/.miplib_cache/$p.mps" "logs/${TAG}_${p}_t1.log" --threads 1 --time-limit 600 >/dev/null 2>&1
  echo "$p $(tail -1 logs/${TAG}_${p}_t1.log)"
done
