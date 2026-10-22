#!/bin/bash
# highs_bin.sh <問題> <秒> <名前>: 計測用パッチ (scripts/highs_instrument.patch + HADD の反復・時間) の highs-bin を 4 スレッドで
cd "$(dirname "$0")"
p=$1; T=$2; name=$3
n=1; while [ -e logs/${p}_${name}_${n}.log ]; do n=$((n+1)); done
printf 'threads=4\nmip_rel_gap=0\ntime_limit=%s\n' "$T" > /tmp/hopt_$$.txt
${HIGHS_BIN:-/home/user/hb/HiGHS/build/bin/highs} --options_file /tmp/hopt_$$.txt ../../.miplib_cache/$p.mps > logs/${p}_${name}_${n}.log 2>&1
rm -f /tmp/hopt_$$.txt
echo "$p $name $n $(grep -E 'Status|Primal bound|Dual bound|Timing' logs/${p}_${name}_${n}.log | head -4 | tr -s ' ' | tr '\n' ' ')"
