#!/bin/bash
# mc11 を 1 回解く。使い方: run.sh 名前 [SUBMIP_HIGHS の値 (空なら既定)] [追加の環境変数 ...]
# ログは logs/名前.log (標準エラー)、結果の JSON は logs/名前.json
cd "$(dirname "$0")/../.."
name=$1; h=$2; shift 2
d=analysis/mc11_h1_20261030/logs
if [ -n "$h" ]; then export ENOMOTO_MIP_SUBMIP_HIGHS="$h"; fi
env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 "$@" python3 scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/mc11.mps \
  --time-limit 60 --threads 4 2> $d/$name.log | tail -1 > $d/$name.json
cat $d/$name.json
