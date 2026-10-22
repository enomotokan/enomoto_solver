#!/bin/bash
# HiGHS (計装版、1 スレッド) を 1 回: run_highs.sh <問題> <名前> <presolve on|off> <時間上限> [追加のオプション行...]
# ログ logs/<名前>.err (HRND/HSEP/HADD 行) と、ラウンドごとの LP のカットの行 dumps/<名前>.cuts
P=$1; N=$2; PRE=$3; T=$4; shift 4
D=$(cd $(dirname $0) && pwd); R=$D/../..
OPT=$D/logs/$N.opt
{ echo "threads = 1"; echo "mip_rel_gap = 0"; echo "presolve = $PRE"; echo "time_limit = $T"; for o in "$@"; do echo "$o"; done; } > $OPT
HB=${HIGHS_BIN:-/home/user/hs/HiGHS/build/bin/highs}
LD_LIBRARY_PATH=$(dirname $HB)/../lib HIGHS_LPCUT_DUMP=$D/dumps/$N.cuts $HB --model_file $R/.miplib_cache/$P.mps --options_file $OPT > $D/logs/$N.out 2> $D/logs/$N.err
grep -E "HRNDEND" $D/logs/$N.err | head -3
grep -E "Primal bound|Dual bound|Nodes|Timing" $D/logs/$N.out | head -5
