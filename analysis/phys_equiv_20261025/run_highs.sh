#!/bin/bash
# run_highs.sh <名前> <秒> [mps] -> logs/<名前>.{log,tr,subst,lift}。HIGHS_BIN で計測版 (highs_trace.patch を当てた v1.15.1)。
# 余分の環境変数 (HIGHS_TRACE_NOSUBST など) はそのまま渡る。log_dev_level=1 で探りの中身も出す
cd "$(dirname "$0")"
name=$1; T=$2; mps=${3:-../../.miplib_cache/physiciansched6-2.mps}
opt=logs/$name.opt
printf "time_limit = %s\nlog_dev_level = 1\n" "$T" > $opt
rm -f logs/$name.subst logs/$name.lift
HIGHS_TRACE=1 HIGHS_TRACE_SUBSTDUMP=logs/$name.subst HIGHS_TRACE_LIFTDUMP=logs/$name.lift ${HIGHS_BIN:-/home/user/HiGHS/build/bin/highs} --options_file $opt $mps > logs/$name.log 2> logs/$name.tr
grep -E "Status|Primal bound|Dual bound|Nodes|Timing" logs/$name.log | head -8
