#!/bin/bash
# 1 回解いて区間計時 (ENOMOTO_MIP_XPROF) と証明の数え (ENOMOTO_PROF_TREE_PAR) を残す。リポジトリの根から:
# bash analysis/proofprop_20261123/one.sh <問題> <種> <秒> <スレッド> <札> [K=V ...]
# ログ: analysis/proofprop_20261123/logs/<問題>_<札>_s<種>_t<スレッド>.log、結果の行は res_one.jsonl
cd "$(dirname "$0")/../.."
D=analysis/proofprop_20261123; p=$1; s=$2; tl=$3; th=$4; tag=$5; shift 5
f=$D/logs/${p}_${tag}_s${s}_t${th}.log
env "$@" ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_XPROF=1 ENOMOTO_PROF_TREE_PAR=1 ENOMOTO_MIP_THREADS=$th ENOMOTO_T_MIP_SEED=$s python3 scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$p.mps --time-limit $tl --threads $th > $f 2>&1
out=$(grep '^{' $f | tail -1)
echo "{\"problem\": \"$p\", \"tag\": \"$tag\", \"seed\": $s, \"threads\": $th, \"env\": \"$*\", \"result\": ${out:-null}}" >> $D/res_one.jsonl
echo "$p $tag s$s t$th $(echo $out | cut -c1-140)"
grep -h "^XPROF" $f | grep -E " (m_|p_)" | tr -s ' ' | paste -sd' '
