#!/bin/bash
# bnatt400 の SCIP (1 スレッド) の部品を 1 つずつ切って解く。使い方: bash scipabl.sh <秒> <名前 パラメータ...> (1 回分)
# → scipabl.txt に 1 行 (名前・パラメータ・結果の JSON・最初の解 (時刻・ノード・出した部品)・再スタートの数)
cd "$(dirname "$0")"
T=$1; n=$2; shift 2; M=../../.miplib_cache/bnatt400.mps
out=$(python scipab.py $M $T $n "$@" 2>/dev/null | grep '^{')
fs=$(grep "First Solution" logs/bnatt400_$n.log.stat | sed 's/ \+/ /g' | cut -c1-160)
rs=$(grep "number of runs" logs/bnatt400_$n.log.stat | sed "s/ \+/ /g")
echo "$n | $* | $out | $fs | $rs" | tee -a scipabl.txt
