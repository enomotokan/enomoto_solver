#!/bin/bash
# 群 3 の 9 問を 1 回ずつ解き、時刻つきのログを残す (当方 4 スレッド・1 スレッド、SCIP の出力つき)。
# 使い方: bash prof_all.sh <名札> [問題…]  (環境変数はそのまま子に渡る)
set -u
cd "$(dirname "$0")"
R=$(cd ../../.. && pwd)
TAG=$1; shift
PS=${*:-"hypothyroid-k1 neos8 exp-1-500-5-5 swath1 seymour1 beasleyC3 neos-1171448 app1-1 mc11"}
for p in $PS; do
  for th in 4 1; do
    python "$R/analysis/smallprob_overhead_20261028/prof_run.py" ours "$R/.miplib_cache/$p.mps" "logs/${TAG}_${p}_t${th}.log" --threads $th --time-limit 600 >/dev/null 2>&1
    echo "$p t$th $(tail -1 logs/${TAG}_${p}_t${th}.log)"
  done
done
