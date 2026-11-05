#!/bin/bash
# FJ と Local-MIP + 連鎖の単独の比べ (ENOMOTO_MIP_DEBUG_FJ_VS_LMIP=3) を、ENOMOTO_MIP_LMIP3 の組ごとに 55 問 (+1) で回す。
# vs_run.sh <名前> [ENOMOTO_MIP_LMIP3 の値]  -> logs/vs_<名前>/<問題>.log。4 本ずつ並べ、各 1 スレッド・時間上限 10 秒
cd "$(dirname "$0")"
name=$1; val=$2
P40=$(python3 -c "import sys; sys.path.insert(0,'../../scripts'); from run_miplib_benchmark import PROBLEMS; print(' '.join(PROBLEMS))")
P16="nursesched-sprint02 hypothyroid-k1 ns1116954 neos-950242 drayage-25-23 drayage-100-23 beasleyC3 mc11 physiciansched6-2 uccase12 neos-2978193-inde neos-911970 mzzv42z neos-960392 roll3000 app1-1"
mkdir -p logs/vs_$name
for p in $(echo $P40 $P16 | tr ' ' '\n' | awk '!s[$0]++'); do
  [ -f ../../.miplib_cache/$p.mps ] || continue
  [ -s logs/vs_$name/$p.log ] && continue
  echo $p
done | xargs -P 4 -I{} env ENOMOTO_MIP_THREADS=1 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_DEBUG_FJ_VS_LMIP=3 ${val:+ENOMOTO_MIP_LMIP3=$val} \
  timeout 60 sh -c "python3 ../heur_ls_20261023/eno_run.py ../../.miplib_cache/{}.mps 10 1 > logs/vs_$name/{}.log 2>&1"
echo "$name DONE"
