#!/bin/bash
# WINWHY の続き 2: (1) DRCR の規則を外す形の種 3・4 の残り、(2) 試作 CLQEXT_KNAP が当たった 8 問を既定と対に (種 1・2、120 秒)、(3) 600 秒の組
D=$(cd "$(dirname "$0")" && pwd)
for p in cbs-cta ns1208400; do for s in 3 4; do
  python3 $D/run.py abl $p --only ours --tag def --seed $s
  python3 $D/run.py abl $p --only ours --tag foldp --env ENOMOTO_MIP_FOLD=rip --seed $s
done; done
for s in 1 2; do
  python3 $D/run.py knap --only ours --tag kdef --seed $s
  python3 $D/run.py knap --only ours --tag knap120 --env ENOMOTO_MIP_CLQEXT_KNAP=1 --seed $s
done
python3 $D/run.py o600
