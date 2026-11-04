#!/bin/bash
# 使い方: rootrun.sh 問題 秒 タグ [環境変数=値 ...]  -> logs/<問題>_<タグ>.log に ENOMOTO のログを残し、根の要点を出す
cd "$(dirname "$0")"
p=$1; t=$2; tag=$3; shift 3
mkdir -p logs
env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_DEBUG_SEPPROF=1 "$@" timeout $((t+120)) python3 ../../scripts/run_miplib_benchmark.py --worker-ours ../../.miplib_cache/$p.mps --time-limit $t --threads 4 > logs/${p}_$tag.log 2>&1
echo "== $p $tag"
grep -E "cut loop done|background cuts: [0-9]+ rounds|SEPPROF root.*cands/chosen" logs/${p}_$tag.log | cut -c1-300 | head -4
tail -1 logs/${p}_$tag.log | cut -c1-250
