#!/bin/bash
# HiGHS (計装版 highs_septime.patch、1 スレッド、根だけ) を 1 回: run_highs.sh <問題> <名前> <時間上限> [追加のオプション行...]
# ログ logs/<名前>.err (HSEPT: 分離のラウンドごとの段階の時間、HRND/HADD など)
P=$1; N=$2; T=$3; shift 3
D=$(cd $(dirname $0) && pwd); R=$D/../..
OPT=$D/logs/$N.opt
{ echo "threads = 1"; echo "mip_rel_gap = 0"; echo "time_limit = $T"; echo "mip_max_nodes = 1"; for o in "$@"; do echo "$o"; done; } > $OPT
HB=${HIGHS_BIN:?HIGHS_BIN}
LD_LIBRARY_PATH=$(dirname $HB)/../lib $HB --model_file $R/.miplib_cache/$P.mps --options_file $OPT > $D/logs/$N.out 2> $D/logs/$N.err
echo "$N: $(grep -E HRNDEND $D/logs/$N.err | head -1)"
