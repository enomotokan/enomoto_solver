#!/bin/bash
# 同じ設定を 2 回ずつ回して、回ごとに同じになるかを見る: bash analysis/workcount_20261125/rep.sh <python> 問題 秒 並べる本数 名前=設定 ...
# 設定は c2c・combo1k (comboharm の combo + HEUR_ADAPT=1k)・def に、空白区切りの環境変数を足したもの (例 "c2c ENOMOTO_MIP_SUBMIP_TICKS=1")
cd "$(dirname "$0")/../.."
PY=$1; P=$2; TL=$3; PAR=$4; shift 4
base() { case $1 in
  c2c) python3 -c "import sys; sys.path.insert(0,'analysis/combo2_20261125'); from cfg import C2C; print(' '.join(f'{k}={v}' for k,v in C2C.items()))";;
  combo1k) python3 -c "import sys; sys.path.insert(0,'analysis/combo_20261124'); from cfg import COMBO; c=dict(COMBO, ENOMOTO_MIP_HEUR_ADAPT='1k'); print(' '.join(f'{k}={v}' for k,v in c.items()))";;
  *) echo "";; esac; }
for spec in "$@"; do name=${spec%%=*}; rest=${spec#*=}; b=${rest%% *}; extra=""; [ "$b" != "$rest" ] && extra=${rest#* }; for r in 1 2; do echo "$name|$(base $b) $extra|$r"; done; done |
xargs -P $PAR -I{} bash -c 'IFS="|" read name envs r <<< "{}"; out=$(env $envs ENOMOTO_MIP_THREADS=1 ENOMOTO_T_MIP_SEED=1 '"$PY"' scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/'"$P"'.mps --time-limit '"$TL"' --threads 1 2>/dev/null | tail -1); echo "'"$P"' $name r$r $out"'
