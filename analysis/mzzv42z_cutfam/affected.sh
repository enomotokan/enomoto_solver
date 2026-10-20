#!/bin/bash
# 各問題を 8 秒だけ解き、根の最初の分離の implied bound カットの候補数 (既定 / ENOMOTO_MIP_IMPLBD_GENINT) を出す
D=$(cd $(dirname "$0") && pwd); mkdir -p $D/logs/aff
for p in "$@"; do
  for mode in def gen; do
    ex=""; [ $mode = gen ] && ex="ENOMOTO_MIP_IMPLBD_GENINT=1"
    env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_DEBUG_SEP=1 ENOMOTO_MIP_THREADS=4 $ex timeout 60 python3 $D/run_eno.py /home/user/enomoto_solver/.miplib_cache/$p.mps 8 > $D/logs/aff/$p.$mode.log 2>&1
  done
  f() { awk '/^SEP path/{exit} /^SEP clique/{c=$NF} /^SEP implied/{print $NF-c; exit}' $1; }
  echo "$p def=$(f $D/logs/aff/$p.def.log) gen=$(f $D/logs/aff/$p.gen.log)"
done
