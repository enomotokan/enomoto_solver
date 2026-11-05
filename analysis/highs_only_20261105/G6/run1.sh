#!/bin/bash
# 使い方: run1.sh 名前 問題 秒 solver(ours|highs) [環境変数 KEY=VAL ...]
# 1 回解いて logs/<問題>_<名前>.log (標準エラー) と res.jsonl (結果の JSON) に残す。ENOMOTO_ の環境変数は消してから与える。
D=$(cd $(dirname $0); pwd); WT=$(cd $D/../../..; pwd)
name=$1; prob=$2; tl=$3; kind=$4; shift 4
for k in $(env | grep -o '^ENOMOTO_[A-Z0-9_]*'); do unset $k; done
export ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_LOG=1 ENOMOTO_BENCH_HIGHS_LOG=1
for kv in "$@"; do export "$kv"; done
log=$D/logs/${prob}_${name}.log
t0=$(date +%s)
out=$(python $WT/scripts/run_miplib_benchmark.py --worker-$kind $WT/.miplib_cache/$prob.mps --time-limit $tl --threads 4 2>$log | tee -a $log | grep '^{' | tail -1)
echo "{\"problem\": \"$prob\", \"name\": \"$name\", \"solver\": \"$kind\", \"tl\": $tl, \"env\": \"$*\", \"wall\": $(( $(date +%s) - t0 )), \"result\": ${out:-null}}" >> $D/res.jsonl
