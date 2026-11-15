#!/bin/bash
# 使い方: one.sh 問題 秒 ログの名 [環境変数=値 ...]  (ENOMOTO、4 スレッド、種 1、ENOMOTO_MIP_LOG=1)
# 結果の行 (JSON) はログの最後の行。ログは logs/<名>.log.gz
D=$(cd "$(dirname "$0")" && pwd); WT=$(cd "$D/../.." && pwd)
p=$1; tl=$2; name=$3; shift 3
cd "$WT"
env $(env | grep -o '^ENOMOTO_[A-Z0-9_]*' | sed 's/^/-u /') ENOMOTO_MIP_THREADS=4 ENOMOTO_T_MIP_SEED=${SEED:-1} ENOMOTO_MIP_LOG=1 "$@" \
  python3 -u scripts/run_miplib_benchmark.py --worker-ours ".miplib_cache/$p.mps" --time-limit "$tl" --threads 4 2>&1 \
  | awk '{ printf "%8.2f %s\n", systime()-t0, $0; fflush() }' t0=$(date +%s) | gzip > "$D/logs/$name.log.gz"
zcat "$D/logs/$name.log.gz" | tail -n 1
