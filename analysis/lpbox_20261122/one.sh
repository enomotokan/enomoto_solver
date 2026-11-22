#!/bin/bash
# 1 回の求解をログつきで回す: one.sh <問題> <ours|highs> <種> <秒> <名前> [環境変数=値 ...]
# ログは logs/<問題>_<名前>_s<種>.log (標準エラー + 結果の JSON、各行の頭に起動からの経過秒)。4 スレッド。
D=$(cd "$(dirname "$0")" && pwd); WT=$(cd "$D/../.." && pwd)
p=$1; who=$2; seed=$3; tl=$4; name=$5; shift 5
mps=$(cd "$WT" && python -c "import sys;sys.path.insert(0,'scripts');import run_miplib_benchmark as rb;from pathlib import Path;print(rb.fetch('$p',Path('.miplib_cache')))")
cd "$WT"
env -u ENOMOTO_MIP_LOG $(env | grep -o '^ENOMOTO_[A-Z0-9_]*' | sed 's/^/-u /') \
  ENOMOTO_MIP_THREADS=4 ENOMOTO_T_MIP_SEED=$seed ENOMOTO_MIP_LOG=1 ENOMOTO_BENCH_HIGHS_LOG=1 "$@" \
  python scripts/run_miplib_benchmark.py --worker-$([ $who = highs ] && echo highs || echo ours) "$mps" --time-limit $tl --threads 4 \
  2>&1 | perl -MTime::HiRes=time -ne 'BEGIN{$t0=time} printf("[%7.2f] %s", time-$t0, $_)' > "$D/logs/${p}_${name}_s${seed}.log"
tail -1 "$D/logs/${p}_${name}_s${seed}.log"
