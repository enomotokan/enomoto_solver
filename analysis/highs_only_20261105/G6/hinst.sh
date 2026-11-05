#!/bin/bash
# 計測用パッチ (scripts/highs_instrument.patch) を当てた HiGHS 1.15.1 の highs-bin で 1 回解く。
# 使い方: hinst.sh 名前 問題 秒 [KEY=VAL ...] (HIGHS_SKIP_SEP などの環境変数、または opt:名前=値 で HiGHS のオプション)
D=$(cd $(dirname $0); pwd); WT=$(cd $D/../../..; pwd)
H=${HIGHS_BIN:-/tmp/claude-0/highs_src/build/bin/highs}
name=$1; prob=$2; tl=$3; shift 3
opt=$D/logs/${prob}_${name}.opt
printf "time_limit = %s\nthreads = 4\nmip_rel_gap = 0\n" $tl > $opt
for kv in "$@"; do case $kv in opt:*) echo "${kv#opt:}" | sed 's/=/ = /' >> $opt;; *) export "$kv";; esac; done
$H --options_file $opt $WT/.miplib_cache/$prob.mps > $D/logs/${prob}_${name}.log 2>&1
