#!/bin/bash
# 使い方: run1.sh 名前 問題 秒 [KEY=VAL ...]  (ENOMOTO を 4 スレッドで 1 回解き、logs/ と res.jsonl に残す)
cd "$(dirname "$0")"
name=$1; prob=$2; tl=$3; shift 3
R=../../..
env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 "$@" timeout $((tl+300)) python $R/scripts/run_miplib_benchmark.py --worker-ours $R/.miplib_cache/$prob.mps --time-limit $tl --threads 4 > logs/${name}_${prob}.out 2> logs/${name}_${prob}.err
last=$(tail -1 logs/${name}_${prob}.out)
echo "{\"name\": \"$name\", \"problem\": \"$prob\", \"env\": \"$*\", \"result\": ${last:-null}}" >> res.jsonl
gzip -f logs/${name}_${prob}.err
