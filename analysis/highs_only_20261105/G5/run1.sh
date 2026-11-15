#!/bin/bash
# 使い方: run1.sh <ours|highs> <問題> <秒> <タグ> [環境変数 ...]
cd $(dirname $0)
while [ -f /tmp/G5_build.lock ]; do sleep 5; done
s=$1; p=$2; t=$3; tag=$4; shift 4
R=/home/user/enomoto_solver
f=$R/.miplib_cache/$p.mps; [ -f $p.mps ] && f=$(pwd)/$p.mps
[ -f $f ] || [ -f $f.gz ] || (cd $R && python -c "import sys;sys.path.insert(0,'scripts');import run_miplib_benchmark as r;from pathlib import Path;r.fetch('$p',Path('.miplib_cache'))")
[ -f $f ] || f=$f.gz
if [ $s = ours ]; then
  env "$@" ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 python $R/scripts/run_miplib_benchmark.py --worker-ours $f --time-limit $t --threads 4 > logs/${p}_$tag.log 2>&1
else
  env "$@" ENOMOTO_BENCH_HIGHS_LOG=1 python $R/scripts/run_miplib_benchmark.py --worker-highs $f --time-limit $t --threads 4 > logs/${p}_$tag.log 2>&1
fi
echo "{\"problem\":\"$p\",\"tag\":\"$tag\",\"env\":\"$*\",\"result\":$(tail -1 logs/${p}_$tag.log)}" >> res.jsonl
