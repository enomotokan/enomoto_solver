#!/bin/bash
# 使い方: run1.sh 名前 問題 秒 [環境変数 KEY=VAL ...]
# ENOMOTO を 4 スレッドで 1 回解いて logs/<問題>_<名前>.log.gz (標準エラー) と res.jsonl (結果の JSON) に残す。ENOMOTO_ の環境変数は消してから与える。
D=$(cd $(dirname $0); pwd); WT=$(cd $D/../../..; pwd)
name=$1; prob=$2; tl=$3; shift 3
for k in $(env | grep -o '^ENOMOTO_[A-Z0-9_]*'); do unset $k; done
export ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_LOG=1
for kv in "$@"; do export "$kv"; done
log=$D/logs/${prob}_${name}.log
t0=$(date +%s.%N)
out=$(python $WT/scripts/run_miplib_benchmark.py --worker-ours $WT/.miplib_cache/$prob.mps --time-limit $tl --threads 4 2>$log | grep '^{' | tail -1)
wall=$(python3 -c "import time;print(round(time.time()-$t0,1))")
root=$(python3 $D/rootinfo.py $log)
echo "{\"problem\": \"$prob\", \"name\": \"$name\", \"tl\": $tl, \"env\": \"$*\", \"wall\": $wall, \"root\": $root, \"result\": ${out:-null}}" >> $D/res.jsonl
gzip -f $log
