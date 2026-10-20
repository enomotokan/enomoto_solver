#!/bin/bash
# 40 問で ENOMOTO_MIP_RESTART_REF が働く (再スタート前の暫定解が再スタート後に実行不能で参照解になる) 問題を探す。
# 使い方: scan.sh <秒> [環境変数=値 ...]。結果は logs/scan.txt
T=$1; shift
D=$(cd $(dirname "$0") && pwd)
for p in gen-ip002 gen-ip054 pk1 gr4x6 markshare_4_0 mas74 mas76 neos5 p0201 misc07 flugpl dcmulti gt2 khb05250 qnet1 rout 10teams air03 blend2 fiber noswot qiu neos-911970 30n20b8 binkar10_1 neos-860300 eil33-2 nw04 assign1-5-8 beavma blp-ar98 ran14x18-disj-8 h80x6320d supportcase26 timtab1 bppc4-08 cvs16r128-89 mik-250-20-75-4 neos-1456979 graph20-20-1rand; do
  mps=/home/user/enomoto_solver/.miplib_cache/$p.mps
  [ -f $mps ] || { curl -sS -o $mps.gz https://miplib.zib.de/WebData/instances/$p.mps.gz && gunzip -f $mps.gz; }
  env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_RESTART_REF=1 "$@" python3 $D/run_eno.py $mps $T > $D/logs/scan_$p.log 2>&1
  echo "$p restarts=$(grep -c '^Restart ' $D/logs/scan_$p.log) ref=$(grep -c 'kept as the RINS reference' $D/logs/scan_$p.log) $(grep RESULT $D/logs/scan_$p.log)"
done
