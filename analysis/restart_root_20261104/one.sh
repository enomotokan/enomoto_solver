#!/bin/bash
# 1 問を ENOMOTO の詳細ログつきで解く。使い方: one.sh 名前 問題 秒 [環境変数=値 ...]
# 出力: logs/名前_問題.out (結果の JSON と利用者向けログ)、logs/名前_問題.err (詳細ログ)
D=$(cd "$(dirname "$0")" && pwd); WT=$(cd "$D/../.." && pwd)
name=$1; prob=$2; t=$3; shift 3
env $(env | grep -o '^ENOMOTO_[A-Z0-9_]*' | sed 's/^/-u /') ENOMOTO_MIP_LOG=1 "$@" python "$WT/scripts/run_miplib_benchmark.py" --worker-ours "$WT/.miplib_cache/$prob.mps" --time-limit "$t" --threads ${THREADS:-4} > "$D/logs/${name}_$prob.out" 2> "$D/logs/${name}_$prob.err"
tail -1 "$D/logs/${name}_$prob.out" | cut -c1-300
