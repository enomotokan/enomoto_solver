#!/bin/bash
# 当方を 1 問だけ回してログを残す (ENOMOTO_MIP_LOG=1)。
# 使い方 (リポジトリの根から): bash analysis/rootlp_20261120/one.sh <札> <種> <秒> <スレッド> <問題> [K=V ...]
# ログ: analysis/rootlp_20261120/logs/<問題>_<札>_s<種>_t<スレッド>.log.gz、結果: res_one.jsonl
cd "$(dirname "$0")/../.."
D=analysis/rootlp_20261120; tag=$1; s=$2; tl=$3; th=$4; p=$5; shift 5
f=$D/logs/${p}_${tag}_s${s}_t${th}.log
env "$@" ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=$th ENOMOTO_T_MIP_SEED=$s python3 scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$p.mps --time-limit $tl --threads $th > $f 2>&1
out=$(grep '^{' $f | tail -1)
gzip -f $f
echo "{\"problem\": \"$p\", \"tag\": \"$tag\", \"seed\": $s, \"threads\": $th, \"env\": \"$*\", \"result\": ${out:-null}}" >> $D/res_one.jsonl
echo "$p $tag s$s t$th $(echo $out | cut -c1-160)"
