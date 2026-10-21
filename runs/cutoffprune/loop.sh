#!/bin/bash
# 使い方: loop.sh <名前> <問題> <回数> <スレッド> <秒> [ENV=VAL ...]
name=$1; prob=$2; n=$3; th=$4; tl=$5; shift 5
d=$(dirname "$0"); mkdir -p "$d/logs"
for i in $(seq 1 $n); do
  log="$d/logs/${name}_${prob}_$i.log"
  env "$@" ENOMOTO_MIP_LOG=1 python "$d/run1.py" "$d/../../.miplib_cache/$prob.mps" $th $tl > "$log.out" 2> "$log"
  r=$(grep RESULT "$log.out")
  w=$(grep -c "WARNING\|MIP_DEBUG_SOL lost\|MIP_DEBUG_SOL: node\|MIP_DEBUG_SOL: dual\|MIP_DEBUG_SOL: the tree\|PROOF_DIAG" "$log")
  echo "$i $r warn=$w" >> "$d/${name}_${prob}.summary"
done
