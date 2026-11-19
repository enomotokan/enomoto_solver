#!/bin/bash
# P2-INF: 途中で衝突の重みを上げる試作 (CONFLICT_WEIGHT_DYN) の確かめ。問題:種:秒:設定,... を 4 スレッドで順に回し logs/dyncheck.txt に足す。
# 設定: def / dyn (DYN=0.1) / dynlx (DYN=0.1 + LIT_EXPLAIN)。リポジトリの根から: PYTHONPATH=<ビルド> bash analysis/p2inf_20261119/dyncheck.sh 問題:種:秒:設定,設定 ...
cd "$(dirname "$0")/../.."
O=analysis/p2inf_20261119/logs/dyncheck.txt
for spec in "$@"; do IFS=: read p s tl cs <<< "$spec"
  f=$(python3 -c "import sys;sys.path.insert(0,'scripts');import run_miplib_benchmark as rb;from pathlib import Path;print(rb.fetch('$p',Path('.miplib_cache')))")
  for c in ${cs//,/ }; do
    case $c in def) E="X=1";; dyn) E="ENOMOTO_MIP_CONFLICT_WEIGHT_DYN=0.1";; dynlx) E="ENOMOTO_MIP_CONFLICT_WEIGHT_DYN=0.1 ENOMOTO_MIP_LIT_EXPLAIN=1";; esac
    log=analysis/p2inf_20261119/logs/dyn_${p}_${c}_s$s.log
    env $E ENOMOTO_MIP_THREADS=4 ENOMOTO_T_MIP_SEED=$s ENOMOTO_MIP_LOG=1 python3 scripts/run_miplib_benchmark.py --worker-ours $f --time-limit $tl --threads 4 > $log 2>&1
    r=$(grep '^{' $log | python3 -c 'import sys,json;r=json.loads(sys.stdin.read());print(r["status"],round(r["time"],1),r["nodes"])')
    raised=$(grep -c "CW_DYN" $log)
    echo "$p s$s $c $r raised=$raised" | tee -a $O
  done
done
