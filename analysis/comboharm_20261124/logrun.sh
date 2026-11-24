#!/bin/bash
# ログつきの 1 回: bash analysis/comboharm_20261124/logrun.sh 問題 def|combo スレッド 秒 [環境変数=値 ...]
# ログは analysis/comboharm_20261124/logs_<問題>_<設定>_t<スレッド>.err (コミットしない)
cd "$(dirname "$0")/../.."
p=$1; c=$2; th=$3; tl=$4; shift 4
E=""; [ "$c" = combo ] && E=$(python3 -c "import sys; sys.path.insert(0,'analysis/combo_20261124'); from cfg import COMBO; print(' '.join(f'{k}={v}' for k,v in COMBO.items()))")
env -u ENOMOTO_MIP_CONFCUT $E "$@" ENOMOTO_MIP_THREADS=$th ENOMOTO_T_MIP_SEED=1 ENOMOTO_MIP_LOG=1 timeout $((tl+600)) .venv/bin/python scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$p.mps --time-limit $tl --threads $th 2> analysis/comboharm_20261124/logs_${p}_${c}_t$th.err | tail -1
