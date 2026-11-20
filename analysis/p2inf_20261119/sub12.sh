#!/bin/bash
# P2-INF: 部分木 (SCIP の道の先頭 12 個を固定) を当方で設定ごとに 1 スレッド・種 1 で解く。sh sub12.sh <mps> <秒> <名前> [環境変数=値 ...]
# 結果は logs/sub12v_<名前>.log の最後の行 (JSON)。
f=$1; tl=$2; n=$3; shift 3
cd "$(dirname "$0")/../.."
env "$@" ENOMOTO_MIP_THREADS=1 ENOMOTO_T_MIP_SEED=1 ENOMOTO_MIP_LOG=1 python3 scripts/run_miplib_benchmark.py --worker-ours $f --time-limit $tl --threads 1 > analysis/p2inf_20261119/logs/sub12v_$n.log 2>&1
echo "$n $(grep '^{' analysis/p2inf_20261119/logs/sub12v_$n.log | python3 -c 'import sys,json;r=json.loads(sys.stdin.read());print(r["status"],r["nodes"],round(r["time"],1))')"
