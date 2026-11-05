#!/bin/bash
# 55 問 (heur_ls_20261023 の batch5.sh と同じ 40 問 + 16 問) で ENOMOTO_MIP_DEBUG_FJ_VS_LMIP=3 を設定ごとに回す。
# 設定は引数 (既定 "def c1")。def/def2 = 既定、c1 = ENOMOTO_MIP_LMIP1_CAND=1、d = ENOMOTO_MIP_LMIP1_CAND=d (重複だけ除く)。
# logs/<問題>_<設定>.log。問題ごとに設定の順を入れ替える
cd "$(dirname "$0")"
P40=$(python3 -c "import sys; sys.path.insert(0,'../../scripts'); from run_miplib_benchmark import PROBLEMS; print(' '.join(PROBLEMS))")
P16="nursesched-sprint02 hypothyroid-k1 ns1116954 neos-950242 drayage-25-23 drayage-100-23 beasleyC3 mc11 physiciansched6-2 uccase12 neos-2978193-inde neos-911970 mzzv42z neos-960392 roll3000 app1-1"
CFG="${*:-def c1}"
k=0
for p in $P40 $P16; do
  [ -f ../../.miplib_cache/$p.mps ] || { echo "$p no mps"; continue; }
  k=$((k+1)); order="$CFG"; [ $((k%2)) = 0 ] && order=$(echo $CFG | tr ' ' '\n' | tac | tr '\n' ' ')
  for c in $order; do
    [ -s logs/${p}_$c.log ] && continue
    E=""; [ $c = c1 ] && E="ENOMOTO_MIP_LMIP1_CAND=1 ENOMOTO_MIP_LOCALMIP_DEBUG=1"; [ $c = d ] && E="ENOMOTO_MIP_LMIP1_CAND=d ENOMOTO_MIP_LOCALMIP_DEBUG=1"
    env ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_DEBUG_FJ_VS_LMIP=3 $E timeout 120 python3 ../heur_ls_20261023/eno_run.py ../../.miplib_cache/$p.mps 12 4 > logs/${p}_$c.log 2>&1
    echo "$p $c $(grep -h FJVSLM logs/${p}_$c.log)"
  done
done
echo VS55 DONE
