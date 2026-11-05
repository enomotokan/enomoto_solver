#!/bin/bash
# FJ と Local-MIP + 連鎖の単独の比べ (ENOMOTO_MIP_DEBUG_FJ_VS_LMIP=3、heur_ls_20261023 追記 2 の 55 問) を ENOMOTO_MIP_LMIP2 の設定ごとに回す。
# 使い方: run_vs.sh <設定名> [並列の数]   設定名 base (既定) / b / l / w / blw など (base で始まる名は既定、それ以外は ENOMOTO_MIP_LMIP2=<設定名の _ より前>。base2・l_2 は 2 回目)
# 1 スレッド・時間の上限 1 秒 (診断は上限と関係なく FJ 3 秒 × 2 + Local-MIP 3 秒を回す) -> logs/<問題>_<設定名>.log
cd "$(dirname "$0")"
cfg=$1; par=${2:-3}
P40=$(python3 -c "import sys; sys.path.insert(0,'../../scripts'); from run_miplib_benchmark import PROBLEMS; print(' '.join(PROBLEMS))")
P16="nursesched-sprint02 hypothyroid-k1 ns1116954 neos-950242 drayage-25-23 drayage-100-23 beasleyC3 mc11 physiciansched6-2 uccase12 neos-2978193-inde neos-911970 mzzv42z neos-960392 roll3000 app1-1"
case $cfg in base*) E="";; *) E="ENOMOTO_MIP_LMIP2=${cfg%%_*}";; esac
for p in $P40 $P16; do echo $p; done | awk '!s[$0]++' | xargs -P $par -I{} sh -c "
  [ -f ../../.miplib_cache/{}.mps ] || exit 0
  [ -s logs/{}_$cfg.log ] && grep -q FJVSLM logs/{}_$cfg.log && exit 0
  env ENOMOTO_MIP_THREADS=1 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_DEBUG_FJ_VS_LMIP=3 $E timeout 120 python3 eno_run.py ../../.miplib_cache/{}.mps 1 1 > logs/{}_$cfg.log 2>&1"
echo "DONE $cfg"
