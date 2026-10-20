#!/bin/bash
# 既定と試作を問題ごとに交互に解く (4 スレッド・60 秒)。使い方: pair.sh <回> 問題...  試作の設定は環境変数 TRIAL (空白区切り)
D=$(cd $(dirname "$0") && pwd); R=/home/user/enomoto_solver
r=$1; shift
for p in "$@"; do
  for mode in def trial; do
    ex=""; [ $mode = trial ] && ex="$TRIAL"
    out=$(env ENOMOTO_MIP_THREADS=4 $ex python3 $R/scripts/run_miplib_benchmark.py --worker-ours $R/.miplib_cache/$p.mps --time-limit 60 --threads 4 | tail -1)
    echo "$p $mode r$r $out"
  done
done
