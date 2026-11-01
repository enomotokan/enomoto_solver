#!/bin/bash
# 1 問を 1 回解く。使い方: run.sh 問題 名前 [追加の環境変数 ...] (例: run.sh mc11 w_s_1 ENOMOTO_MIP_BG_CUTS_WAIT=1 ENOMOTO_MIP_SUBMIP_HIGHS=s)
# ログは logs/名前.log (標準エラー)、結果の JSON は logs/名前.json
cd "$(dirname "$0")/../.."
prob=$1; name=$2; shift 2
d=analysis/bgcuts_wait_20261030/logs
env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 "$@" python3 scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$prob.mps \
  --time-limit 60 --threads 4 2> $d/$name.log | tail -1 > $d/$name.json
echo "$name $(cat $d/$name.json)"
