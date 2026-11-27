#!/bin/bash
# 壁時計に依らないかの確かめ: combo1k (comboharm の combo + HEUR_ADAPT=1k) を、SUBMIP_TICKS の有無とログ (ENOMOTO_MIP_LOG) の有無で回す。
# ログは壁時計の進みを変える (出力の手間) ので、ログの有無で結果が変われば、どこかが壁時計で決まっている。
# bash analysis/workcount_20261125/logcheck.sh <python> 問題 秒 "ST の値 ..." 出力
cd "$(dirname "$0")/../.."
PY=$1; P=$2; TL=$3; VS=$4; OUT=$5
E=$(python3 -c "import sys; sys.path.insert(0,'analysis/combo_20261124'); from cfg import COMBO; c=dict(COMBO, ENOMOTO_MIP_HEUR_ADAPT='1k'); print(' '.join(f'{k}={v}' for k,v in c.items()))")
for v in $VS; do for lg in log nolog; do
  ( L=""; [ $lg = log ] && L="ENOMOTO_MIP_LOG=1"
    out=$(env $E ENOMOTO_MIP_SUBMIP_TICKS=$v $L ENOMOTO_MIP_THREADS=1 ENOMOTO_T_MIP_SEED=1 $PY scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$P.mps --time-limit $TL --threads 1 2>/dev/null | tail -1)
    echo "$P combo1k ST=$v $lg $out" >> $OUT ) &
done; done
wait
