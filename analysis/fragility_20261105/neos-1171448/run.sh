#!/bin/bash
# 使い方: run.sh <タグ> <スレッド数> [追加の環境変数 ...]
# 例: run.sh a1 4 ENOMOTO_T_MIP_SEED=1
cd "$(dirname "$0")/../../.."
tag=$1; th=$2; shift 2
out=analysis/fragility_20261105/neos-1171448/log_${tag}.txt
env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=$th "$@" python scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/neos-1171448.mps --time-limit 600 --threads $th > $out 2>&1
tail -1 $out
